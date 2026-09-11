//! Resource loading: priority-ordered fetching over a transport
//! abstraction, with a deterministic LRU cache. Normative reference:
//! ADR-0010.
//!
//! `pump` is the unit of progress: one pump advances virtual time by
//! one tick (the PP-02 clock) and resolves exactly one pending fetch
//! — highest priority first, FIFO within a priority. Cache stamps are
//! (virtual tick, touch sequence): eviction order is fully
//! deterministic. Transport failures are Module-class (`Err` to the
//! waiter) and are never cached; malformed URLs never reach the
//! loader (they failed at parse time — RecoverableInput).

use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BinaryHeap, HashMap};
use std::fmt;

use nbe_core::clock::VirtualClock;
use nbe_core::error::ModuleError;
use nbe_core::tick::Tick;

use crate::url::Url;

/// One fetched resource body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    bytes: Vec<u8>,
    mime_type: Option<String>,
}

impl Resource {
    /// Construct a resource.
    pub fn new(bytes: Vec<u8>, mime_type: Option<String>) -> Self {
        Self { bytes, mime_type }
    }

    /// The body bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The MIME type, when the transport reported one.
    #[must_use]
    pub fn mime_type(&self) -> Option<&str> {
        self.mime_type.as_deref()
    }
}

/// Fetch priority. Higher pops first; same priority is FIFO.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchPriority {
    /// Pops before everything (2011 Chrome: parser-blocking loads).
    High,
    /// The common case.
    Default,
    /// Pops last (prefetch-style).
    Low,
}

impl FetchPriority {
    /// Heap ordering rank: smaller pops first.
    fn rank(self) -> u8 {
        match self {
            FetchPriority::High => 0,
            FetchPriority::Default => 1,
            FetchPriority::Low => 2,
        }
    }
}

/// Identifier of one submitted fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FetchId(u64);

impl fmt::Display for FetchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "f{}", self.0)
    }
}

/// A finished fetch. `Err` is Module-class: transport failure
/// (refused, reset, unsupported). Malformed URLs never get here —
/// they failed at parse time (RecoverableInput).
#[derive(Debug, Clone)]
pub struct CompletedFetch {
    /// The submit identifier.
    pub id: FetchId,
    /// The fetched URL.
    pub url: Url,
    /// The body, or the transport failure.
    pub result: Result<Resource, ModuleError>,
}

/// The transport seam the loader drives. v1 is synchronous: one
/// fetch per pump turn. The real HTTP client (next package)
/// implements this; tests use fakes.
pub trait Transport {
    /// Fetch one resource. Returning `Err` is a Module-class failure:
    /// the loader reports it to the waiter and never caches it.
    fn fetch(&mut self, url: &Url) -> Result<Resource, ModuleError>;
}

struct PendingFetch {
    rank: u8,
    seq: u64,
    id: FetchId,
    url: Url,
}

impl PartialEq for PendingFetch {
    fn eq(&self, other: &Self) -> bool {
        self.rank == other.rank && self.seq == other.seq
    }
}

impl Eq for PendingFetch {}

impl PartialOrd for PendingFetch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PendingFetch {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank.cmp(&other.rank).then(self.seq.cmp(&other.seq))
    }
}

struct CacheEntry {
    resource: Resource,
    stamp: (Tick, u64),
}

/// A resource loader: priority queue over a transport, plus a
/// byte-budgeted LRU cache keyed by serialized URL.
pub struct ResourceLoader<T: Transport> {
    transport: T,
    clock: VirtualClock,
    pending: BinaryHeap<Reverse<PendingFetch>>,
    next_fetch: u64,
    next_touch: u64,
    cache: HashMap<String, CacheEntry>,
    lru: BTreeMap<(Tick, u64), String>,
    byte_budget: usize,
    cached_bytes: usize,
}

impl<T: Transport> ResourceLoader<T> {
    /// Build a loader over `transport` with a cache byte budget of
    /// `byte_budget` (resources larger than the budget are delivered
    /// but not cached).
    pub fn new(transport: T, byte_budget: usize) -> Self {
        Self {
            transport,
            clock: VirtualClock::new(),
            pending: BinaryHeap::new(),
            next_fetch: 1,
            next_touch: 0,
            cache: HashMap::new(),
            lru: BTreeMap::new(),
            byte_budget,
            cached_bytes: 0,
        }
    }

    /// Queue a fetch. Returns its identifier; the completion is
    /// produced by `pump` in priority order (FIFO within a priority).
    pub fn submit(&mut self, url: Url, priority: FetchPriority) -> FetchId {
        let id = FetchId(self.next_fetch);
        self.next_fetch += 1;
        self.pending.push(Reverse(PendingFetch {
            rank: priority.rank(),
            seq: id.0,
            id,
            url,
        }));
        id
    }

    /// Run one loader turn: advance virtual time by one tick, pop the
    /// next pending fetch (highest priority, FIFO within), and
    /// resolve it — from the cache (re-stamping it) or the transport
    /// (storing the result, subject to budget and eviction).
    /// Returns `None` when nothing is pending.
    pub fn pump(&mut self) -> Option<CompletedFetch> {
        self.clock.advance();
        let Reverse(entry) = self.pending.pop()?;
        let key = entry.url.to_string();
        if let Some(resource) = self.cache_lookup(&key) {
            return Some(CompletedFetch {
                id: entry.id,
                url: entry.url,
                result: Ok(resource),
            });
        }
        let result = self.transport.fetch(&entry.url);
        match result {
            Ok(resource) => {
                let delivered = resource.clone();
                self.store_in_cache(key, resource);
                Some(CompletedFetch {
                    id: entry.id,
                    url: entry.url,
                    result: Ok(delivered),
                })
            }
            Err(e) => Some(CompletedFetch {
                id: entry.id,
                url: entry.url,
                result: Err(e),
            }),
        }
    }

    /// Number of fetches waiting to run.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Number of entries currently cached.
    #[must_use]
    pub fn cached_count(&self) -> usize {
        self.cache.len()
    }

    /// Total bytes currently cached.
    #[must_use]
    pub fn cached_bytes(&self) -> usize {
        self.cached_bytes
    }

    /// The underlying transport, for inspection.
    pub fn transport(&self) -> &T {
        &self.transport
    }

    fn cache_lookup(&mut self, key: &str) -> Option<Resource> {
        let entry = self.cache.get_mut(key)?;
        self.next_touch += 1;
        let stamp = (self.clock.now(), self.next_touch);
        self.lru.remove(&entry.stamp);
        entry.stamp = stamp;
        self.lru.insert(stamp, key.to_string());
        Some(entry.resource.clone())
    }

    fn store_in_cache(&mut self, key: String, resource: Resource) {
        let size = resource.bytes().len();
        if size > self.byte_budget {
            return; // oversize resources are delivered but never stored
        }
        self.next_touch += 1;
        let stamp = (self.clock.now(), self.next_touch);
        self.cached_bytes += size;
        self.cache
            .insert(key.clone(), CacheEntry { resource, stamp });
        self.lru.insert(stamp, key);
        while self.cached_bytes > self.byte_budget {
            let (oldest, key) = match self.lru.pop_first() {
                Some(pair) => pair,
                None => break,
            };
            nbe_core::invariant!(
                oldest != stamp,
                "eviction picked the entry inserted this turn"
            );
            if let Some(entry) = self.cache.remove(&key) {
                self.cached_bytes -= entry.resource.bytes().len();
            } else {
                nbe_core::invariant!(false, "lru and cache are out of sync: {key}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::url::parse_url;
    use nbe_core::snapshot::{check_golden, GoldenOutcome};

    type Responses = HashMap<String, Result<Resource, ModuleError>>;

    fn plain(body: &[u8]) -> Resource {
        Resource::new(body.to_vec(), Some(String::from("text/plain")))
    }

    fn url(input: &str) -> Url {
        parse_url(input).unwrap()
    }

    fn describe(completion: &CompletedFetch) -> String {
        match &completion.result {
            Ok(resource) => format!(
                "{} {} Ok bytes={}",
                completion.id,
                completion.url,
                resource.bytes().len()
            ),
            Err(e) => format!("{} {} Err {}", completion.id, completion.url, e),
        }
    }

    struct MockTransport {
        responses: Responses,
        calls: Vec<String>,
    }

    impl Transport for MockTransport {
        fn fetch(&mut self, url: &Url) -> Result<Resource, ModuleError> {
            let key = url.to_string();
            self.calls.push(key.clone());
            match self.responses.get(&key) {
                Some(result) => result.clone(),
                None => Err(ModuleError::new("mock-transport", "no response")),
            }
        }
    }

    fn mock(responses: Responses) -> MockTransport {
        MockTransport {
            responses,
            calls: Vec::new(),
        }
    }

    #[test]
    fn higher_priority_pops_first() {
        let mut loader = ResourceLoader::new(mock(Responses::new()), 1024);
        let low = loader.submit(url("http://a.test/low"), FetchPriority::Low);
        let high = loader.submit(url("http://a.test/high"), FetchPriority::High);
        let default = loader.submit(url("http://a.test/mid"), FetchPriority::Default);
        let first = loader.pump().unwrap();
        let second = loader.pump().unwrap();
        let third = loader.pump().unwrap();
        assert_eq!(first.id, high);
        assert_eq!(second.id, default);
        assert_eq!(third.id, low);
        assert!(first.result.is_err());
        assert!(second.result.is_err());
        assert!(third.result.is_err());
        assert!(loader.pump().is_none());
    }

    #[test]
    fn same_priority_runs_fifo() {
        let mut loader = ResourceLoader::new(mock(Responses::new()), 1024);
        let first = loader.submit(url("http://a.test/1"), FetchPriority::Default);
        let second = loader.submit(url("http://a.test/2"), FetchPriority::Default);
        assert_eq!(loader.pump().unwrap().id, first);
        assert_eq!(loader.pump().unwrap().id, second);
    }

    #[test]
    fn cache_hits_skip_the_transport() {
        let mut responses = Responses::new();
        responses.insert(String::from("http://a.test/x"), Ok(plain(b"payload")));
        let mut loader = ResourceLoader::new(mock(responses), 1024);

        loader.submit(url("http://a.test/x"), FetchPriority::Default);
        assert!(loader.pump().unwrap().result.is_ok());
        assert_eq!(loader.transport().calls.len(), 1);

        loader.submit(url("http://a.test/x"), FetchPriority::Default);
        let completion = loader.pump().unwrap();
        let bytes = match completion.result {
            Ok(resource) => resource.bytes().to_vec(),
            Err(_) => Vec::new(),
        };
        assert_eq!(bytes, b"payload".to_vec());
        assert_eq!(loader.transport().calls.len(), 1);
    }

    #[test]
    fn failures_are_reported_and_never_cached() {
        let mut responses: Responses = HashMap::new();
        responses.insert(
            String::from("http://a.test/bad"),
            Err(ModuleError::new("mock-transport", "refused")),
        );
        let mut loader = ResourceLoader::new(mock(responses), 1024);

        loader.submit(url("http://a.test/bad"), FetchPriority::Default);
        assert!(loader.pump().unwrap().result.is_err());
        loader.submit(url("http://a.test/bad"), FetchPriority::Default);
        assert!(loader.pump().unwrap().result.is_err());
        assert_eq!(loader.transport().calls.len(), 2);
        assert_eq!(loader.cached_count(), 0);
    }

    #[test]
    fn oversize_resources_bypass_the_cache() {
        let mut responses = Responses::new();
        responses.insert(
            String::from("http://a.test/big"),
            Ok(plain(b"0123456789abcdef")),
        );
        let mut loader = ResourceLoader::new(mock(responses), 8);

        loader.submit(url("http://a.test/big"), FetchPriority::Default);
        assert!(loader.pump().unwrap().result.is_ok());
        assert_eq!(loader.cached_count(), 0);

        loader.submit(url("http://a.test/big"), FetchPriority::Default);
        assert!(loader.pump().unwrap().result.is_ok());
        assert_eq!(loader.transport().calls.len(), 2);
    }

    #[test]
    fn lru_eviction_removes_the_oldest_entry() {
        let mut responses = Responses::new();
        responses.insert(String::from("http://a.test/a"), Ok(plain(b"aaaaaaaa")));
        responses.insert(String::from("http://a.test/b"), Ok(plain(b"bbbbbbbb")));
        responses.insert(String::from("http://a.test/c"), Ok(plain(b"cccccccc")));
        let mut loader = ResourceLoader::new(mock(responses), 16);

        loader.submit(url("http://a.test/a"), FetchPriority::Default);
        assert!(loader.pump().unwrap().result.is_ok());
        loader.submit(url("http://a.test/b"), FetchPriority::Default);
        assert!(loader.pump().unwrap().result.is_ok());
        assert_eq!(loader.cached_count(), 2);
        assert_eq!(loader.cached_bytes(), 16);

        loader.submit(url("http://a.test/c"), FetchPriority::Default);
        assert!(loader.pump().unwrap().result.is_ok());
        assert_eq!(loader.cached_count(), 2);
        assert_eq!(loader.cached_bytes(), 16);

        // The eviction picked a (the oldest), not b: b is served from
        // the cache with no new transport call.
        loader.submit(url("http://a.test/b"), FetchPriority::Default);
        assert!(loader.pump().unwrap().result.is_ok());
        assert_eq!(loader.transport().calls.len(), 3);

        // a really was evicted: fetching it goes to the wire.
        loader.submit(url("http://a.test/a"), FetchPriority::Default);
        assert!(loader.pump().unwrap().result.is_ok());
        assert_eq!(loader.transport().calls.len(), 4);
    }

    #[test]
    fn cache_touches_update_recency() {
        let mut responses = Responses::new();
        responses.insert(String::from("http://a.test/a"), Ok(plain(b"aaaaaaaa")));
        responses.insert(String::from("http://a.test/b"), Ok(plain(b"bbbbbbbb")));
        responses.insert(String::from("http://a.test/c"), Ok(plain(b"cccccccc")));
        let mut loader = ResourceLoader::new(mock(responses), 16);

        loader.submit(url("http://a.test/a"), FetchPriority::Default);
        assert!(loader.pump().unwrap().result.is_ok()); // a cached (t1)
        loader.submit(url("http://a.test/b"), FetchPriority::Default);
        assert!(loader.pump().unwrap().result.is_ok()); // b cached (t2)

        // Re-touch a: it becomes the most recently used.
        loader.submit(url("http://a.test/a"), FetchPriority::Default);
        assert!(loader.pump().unwrap().result.is_ok()); // cache hit
        assert_eq!(loader.transport().calls.len(), 2);

        // Inserting c evicts the untouched b, not the touched a.
        loader.submit(url("http://a.test/c"), FetchPriority::Default);
        assert!(loader.pump().unwrap().result.is_ok());
        assert_eq!(loader.cached_count(), 2);

        // a survived: served from the cache with no new transport call.
        loader.submit(url("http://a.test/a"), FetchPriority::Default);
        assert!(loader.pump().unwrap().result.is_ok());
        assert_eq!(loader.transport().calls.len(), 3);

        // b really was evicted: fetching it goes to the wire.
        loader.submit(url("http://a.test/b"), FetchPriority::Default);
        assert!(loader.pump().unwrap().result.is_ok());
        assert_eq!(loader.transport().calls.len(), 4);
    }

    #[test]
    fn golden_loader_schedule() {
        let mut responses = Responses::new();
        responses.insert(String::from("http://a.test/a"), Ok(plain(b"aaaaaaaa")));
        responses.insert(String::from("http://a.test/b"), Ok(plain(b"bbbbbbbb")));
        responses.insert(String::from("http://a.test/c"), Ok(plain(b"cccccccc")));
        responses.insert(
            String::from("http://a.test/e"),
            Err(ModuleError::new("mock-transport", "connection reset")),
        );
        let mut loader = ResourceLoader::new(mock(responses), 16);

        let a = url("http://a.test/a");
        loader.submit(a.clone(), FetchPriority::Default); // f1
        loader.submit(url("http://a.test/b"), FetchPriority::Low); // f2
        loader.submit(url("http://a.test/c"), FetchPriority::High); // f3
        loader.submit(url("http://a.test/e"), FetchPriority::Default); // f4

        let mut trace: Vec<String> = Vec::new();
        for _ in 0..4 {
            trace.push(describe(&loader.pump().unwrap()));
        }
        assert!(loader.pump().is_none());
        trace.push(String::from("idle"));

        loader.submit(a, FetchPriority::Default); // f5 — cache hit
        trace.push(describe(&loader.pump().unwrap()));

        trace.push(format!(
            "cached={} bytes={} calls={}",
            loader.cached_count(),
            loader.cached_bytes(),
            loader.transport().calls.len()
        ));
        let outcome = check_golden("pp-08-loader", trace.join("\n").as_bytes());
        assert!(
            matches!(outcome, GoldenOutcome::Matched | GoldenOutcome::Updated),
            "golden mismatch: {outcome:?}"
        );
    }
}
