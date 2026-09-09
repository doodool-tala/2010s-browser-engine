//! Deterministic seeded RNG. Normative reference: ADR-0004.
//!
//! SplitMix64: pure 64-bit integer arithmetic with wrapping operations,
//! identical on every platform. The engine needs tie-breaking noise
//! (cache eviction order, fallback selection); all such noise MUST come
//! from this type, seeded deterministically, so identical inputs produce
//! identical engine behavior and identical golden snapshots.

/// Deterministic SplitMix64 generator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeededRng {
    state: u64,
}

impl SeededRng {
    /// Create a generator from a seed. Identical seeds produce
    /// identical sequences, on every platform, forever.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Next raw 64-bit value.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Next value truncated to 32 bits.
    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    /// Next value in [0, 1) with 53 bits of precision.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / ((1u64 << 53) as f64))
    }

    /// Next value in [0, bound). Requires bound > 0. The modulo bias
    /// is acceptable for engine tie-breaking and is deterministic.
    pub fn next_bounded(&mut self, bound: u64) -> u64 {
        crate::invariant!(bound > 0, "next_bounded requires a positive bound");
        self.next_u64() % bound
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = SeededRng::new(42);
        let mut b = SeededRng::new(42);
        for _ in 0..256 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = SeededRng::new(0);
        let mut b = SeededRng::new(1);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn f64_values_in_unit_range() {
        let mut rng = SeededRng::new(7);
        for _ in 0..1000 {
            assert!((0.0..1.0).contains(&rng.next_f64()));
        }
    }

    #[test]
    fn bounded_values_below_bound() {
        let mut rng = SeededRng::new(9);
        for _ in 0..1000 {
            assert!(rng.next_bounded(7) < 7);
        }
    }
}
