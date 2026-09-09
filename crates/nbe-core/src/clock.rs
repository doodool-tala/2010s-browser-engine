//! Virtual clock. Normative reference: ADR-0004.
//!
//! Engine time is deterministic: a monotonic tick counter owned by the
//! clock, advanced only by explicit engine actions. The OS clock is
//! never consulted. PP-06 builds the event loop's scheduler on this.

use crate::tick::Tick;

/// Deterministic virtual clock. Monotonically advancing; never reads
/// wall-clock time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VirtualClock {
    tick: Tick,
}

impl VirtualClock {
    /// A clock at `Tick::ORIGIN` (t0).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The current tick.
    #[must_use]
    pub fn now(&self) -> Tick {
        self.tick
    }

    /// Advance by one tick; returns the new current tick.
    pub fn advance(&mut self) -> Tick {
        self.tick = self.tick.next();
        self.tick
    }

    /// Advance by `n` ticks (0 is a no-op); returns the new tick.
    pub fn advance_by(&mut self, n: u64) -> Tick {
        for _ in 0..n {
            self.advance();
        }
        self.tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_clock_starts_at_origin() {
        assert_eq!(VirtualClock::new().now(), Tick::ORIGIN);
    }

    #[test]
    fn advance_is_monotonic() {
        let mut clock = VirtualClock::new();
        assert_eq!(clock.advance(), Tick(1));
        assert_eq!(clock.advance(), Tick(2));
        assert_eq!(clock.now(), Tick(2));
    }

    #[test]
    fn advance_by_counts_from_current() {
        let mut clock = VirtualClock::new();
        assert_eq!(clock.advance_by(5), Tick(5));
        assert_eq!(clock.advance_by(0), Tick(5));
        assert_eq!(clock.advance(), Tick(6));
    }

    #[test]
    fn two_clocks_advance_identically() {
        let mut a = VirtualClock::new();
        let mut b = VirtualClock::new();
        for _ in 0..100 {
            a.advance();
            b.advance();
        }
        assert_eq!(a.now(), b.now());
    }
}
