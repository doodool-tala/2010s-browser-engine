# ADR-0003: Deterministic logging and engine time
Status: accepted

## Decision
All engine output flows through tracing, installed once per process by
nbe_core::logging::init. No wall-clock timestamps, no ANSI, stderr only,
level via NBE_LOG (default info). print!/println!/eprintln! are
lint-denied in library crates. Engine time is nbe_core::tick::Tick.

## Consequences
- Log output is byte-identical across identical runs: usable as a
  regression tool and verified by G1.7.
- PP-02 builds the virtual clock and snapshot harness on Tick.
