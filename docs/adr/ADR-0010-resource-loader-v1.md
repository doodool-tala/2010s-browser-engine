# ADR-0010: Resource loader v1
Status: accepted

## Decision
- Loading is transport-abstracted: `Transport::fetch` is the seam the
  real HTTP client (next package) implements; v1 drives it
  synchronously — one fetch per pump turn.
- `pump` is the unit of progress: each pump advances virtual time by
  one tick (the PP-02 clock) and resolves exactly one pending fetch —
  highest priority first (High, Default, Low), FIFO within a priority
  (submit sequence).
- The cache is keyed by serialized URL and budgeted in bytes. LRU
  stamps are (virtual tick, touch sequence): eviction is fully
  deterministic. Resources larger than the budget are delivered but
  never cached. Failures are Module-class (Err to the waiter) and are
  never cached.
- v1 has no request coalescing: with a synchronous transport, the
  cache already collapses repeat requests across pumps. Coalescing
  arrives with the async transport.
- Cache/LRU consistency is enforced with `invariant!` — an engine
  logic bug there kills the process; a refused connection never does.
  The taxonomy's two branches, both in production.

## Consequences
- The golden test locks scheduling, cache, touch-recency, and
  eviction order byte-exact.
- The error taxonomy's Module branch is now in production alongside
  RecoverableInput — the two-branch split is real.
- Cache keys inherit DV-004: percent-encoding normalization must land
  before cache identity is trusted across origins.
