# Build log

One row per work package, appended after its gate passes.

| PP | Title | Gate | Result | Notes |
|----|-------|------|--------|-------|
| PP-01 | Workspace substrate | G1 | PASS | Executed from the in-repo spec after the fabricated first attempt was discarded; fmt, clippy, tests, release build, forbidden-dep, unsafe, and log-determinism checks all green on rustc 1.81.0. |
| PP-02 | Determinism harness | G1 | PASS | Virtual clock, seeded SplitMix64 RNG, and golden snapshot framework added per spec with byte-exact pp-02-determinism golden; fmt, 17 unit + 1 doctest, and G1 all green on rustc 1.81.0. |
| PP-03 | IPC: schemas, framing, transport | G1 | PASS | IpcMessage/IpcEnvelope schemas, length-prefixed frame codec, and bounded in-process transport added per spec with byte-exact pp-03-ipc-wire golden; fmt, 12 unit tests, unchanged nbe-core 17 + 1 doctest, and G1 all green on rustc 1.81.0. |
| PP-04 | Process model: supervision, crash detection, restart | G1 | PASS | Pipe transport, renderer supervisor, and protocol-conforming nbe-renderer binary added per spec; end-to-end lifecycle, injected-crash detection, and stdin-EOF tests all green (15 ipc + 3 renderer + unchanged 17+1 core) with G1 PASS on rustc 1.81.0. |
| PP-05 | IDL codegen (parser, generator, DOM interface traits) | G1 | PASS | WebIDL subset parser, deterministic Rust trait generator, and committed generated interfaces.rs added per spec; 6 bindings + 3 renderer + unchanged 17+1 core tests green, generated file fmt-stable in compare mode, G1 PASS, smoke line unchanged on stderr, on rustc 1.81.0. |
| PP-06 | Renderer event loop (lanes, virtual time, turns) | G1 | PASS | EventLoop/LoopHandle/TurnOutcome added per spec with byte-exact pp-06-eventloop golden; 6 unit + 4 integration (PP-04 tests unchanged) + nbe-core 17+1, nbe-ipc 15, nbe-bindings 6, G1 PASS, smoke line unchanged, on rustc 1.81.0. |
| PP-07 | URL parser and origin | G1 | PASS | Spec §4 URL parser (WHATWG-informed: hosts domain/IPv4/IPv6, dot-segment removal, default-port stripping) and §5 Origin model added per spec with byte-exact pp-07-url golden; 14 nbe-net tests green in update and compare modes, G1 PASS, smoke line unchanged, lockfile bounded to the sanctioned nbe-net → nbe-core edge, on rustc 1.81.0. |
| PP-08 | Resource loader v1 (priority, cache, LRU) | G1 | PASS | Priority-ordered transport-abstracted loader and deterministic byte-budgeted LRU cache added per spec with byte-exact pp-08-loader golden; the two LRU tests landed under owner amendment A7 (spec errata correcting the survivor/eviction checks in two §4 test bodies), with 22 nbe-net tests green in update and compare modes, G1 PASS, smoke line unchanged, and an empty Cargo.lock diff on rustc 1.81.0. |
