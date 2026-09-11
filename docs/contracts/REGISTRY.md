# Frozen contract registry

The normative text of each contract is the rustdoc of the frozen path at the
freezing commit. Frozen = downstream work packages may rely on it. Changes
require an A-type amendment prompt that re-runs all downstream gates.
vN to vN+1: additive only. Breaking changes: new major row + amendment.

| ID | Contract | Path | Version | Frozen by | Status |
|----|----------|------|---------|-----------|--------|
| C-001 | Failure taxonomy + invariant! | nbe_core::error | v1 | PP-01 | frozen |
| C-002 | Engine tick primitive | nbe_core::tick | v1 | PP-01 | frozen |
| C-003 | Log subscriber init (stderr writer, A6) | nbe_core::logging | v2 | PP-01; A6 | frozen |
| C-004 | Virtual clock | nbe_core::clock | v1 | PP-02 | frozen |
| C-005 | Seeded RNG (SplitMix64) | nbe_core::rng | v1 | PP-02 | frozen |
| C-006 | Golden snapshot framework | nbe_core::snapshot | v1 | PP-02 | frozen |
| C-007 | IPC message schema (IpcMessage + IpcEnvelope) | nbe_ipc::message | v1 | PP-03 | frozen |
| C-008 | Length-prefixed frame codec | nbe_ipc::frame | v1 | PP-03 | frozen |
| C-009 | In-process IPC channel pair | nbe_ipc::transport | v1 | PP-03 | frozen |
| C-010 | Pipe transport (send_envelope + FrameReader) | nbe_ipc::pipe | v1 | PP-04 | frozen |
| C-011 | Renderer supervisor (spawn, send, recv, poll) | nbe_ipc::supervisor | v1 | PP-04 | frozen |
| C-012 | Renderer binary protocol loop | nbe_renderer bin | v1 | PP-04 | frozen |
| C-013 | WebIDL subset parser | nbe_bindings::idl | v1 | PP-05 | frozen |
| C-014 | Rust interface generator | nbe_bindings::gen | v1 | PP-05 | frozen |
| C-015 | Generated DOM interface traits | nbe_dom::interfaces | v1 | PP-05 | frozen |
| C-016 | Renderer event loop (lanes, virtual time, turns) | nbe_renderer::eventloop::EventLoop | v1 | PP-06 | frozen |
| C-017 | Task context handle | nbe_renderer::eventloop::LoopHandle | v1 | PP-06 | frozen |
| C-018 | Turn outcome reporting | nbe_renderer::eventloop::TurnOutcome | v1 | PP-06 | frozen |
