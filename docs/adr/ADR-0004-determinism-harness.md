# ADR-0004: Determinism harness
Status: accepted

## Decision
- Engine time is the VirtualClock (Tick counter); engine code never
  reads the OS clock.
- All engine randomness flows through SeededRng (SplitMix64, wrapping
  integer arithmetic, platform-independent by construction).
- Deterministic subsystems are regression-tested against byte-exact
  golden files in test-goldens/ via nbe_core::snapshot. Verification is
  byte comparison; FNV-1a hashes are reporting only.
- Goldens update only via NBE_UPDATE_GOLDENS=1 and land as reviewed
  diffs.

## Consequences
- Identical inputs produce identical engine state, logs, and goldens
  on any platform: the foundation for the G5/G6 snapshot gates.
- CI now mechanically rejects PRs that modify or delete scripts/ or
  existing docs/pp/ specs (append-only trust model).
