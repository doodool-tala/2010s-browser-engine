# Build log

One row per work package, appended after its gate passes.

| PP | Title | Gate | Result | Notes |
|----|-------|------|--------|-------|
| PP-01 | Workspace substrate | G1 | PASS | Executed from the in-repo spec after the fabricated first attempt was discarded; fmt, clippy, tests, release build, forbidden-dep, unsafe, and log-determinism checks all green on rustc 1.81.0. |
| PP-02 | Determinism harness | G1 | PASS | Virtual clock, seeded SplitMix64 RNG, and golden snapshot framework added per spec with byte-exact pp-02-determinism golden; fmt, 17 unit + 1 doctest, and G1 all green on rustc 1.81.0. |
| PP-03 | IPC: schemas, framing, transport | G1 | PASS | IpcMessage/IpcEnvelope schemas, length-prefixed frame codec, and bounded in-process transport added per spec with byte-exact pp-03-ipc-wire golden; fmt, 12 unit tests, unchanged nbe-core 17 + 1 doctest, and G1 all green on rustc 1.81.0. |
| PP-04 | Process model: supervision, crash detection, restart | G1 | PASS | Pipe transport, renderer supervisor, and protocol-conforming nbe-renderer binary added per spec; end-to-end lifecycle, injected-crash detection, and stdin-EOF tests all green (15 ipc + 3 renderer + unchanged 17+1 core) with G1 PASS on rustc 1.81.0. |
