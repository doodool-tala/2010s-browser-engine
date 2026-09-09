//! Deterministic engine time. Normative reference: ADR-0003.
//!
//! Wall-clock time is FORBIDDEN in engine logic; only `nbe-shell` may read
//! the OS clock, and only at the OS-input boundary. Engine time is a
//! monotonically increasing tick counter. PP-02 builds the full virtual
//! clock on this primitive.

/// A monotonically increasing engine tick. Ordered and hashable so it can
/// key caches, logs, and snapshots deterministically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Tick(pub u64);

impl Tick {
    /// The first tick of the engine.
    pub const ORIGIN: Tick = Tick(0);

    /// Advance one tick.
    #[must_use]
    pub fn next(self) -> Tick {
        Tick(self.0 + 1)
    }
}

impl std::fmt::Display for Tick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "t{}", self.0)
    }
}
