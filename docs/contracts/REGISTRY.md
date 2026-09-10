# Frozen contract registry

The normative text of each contract is the rustdoc of the frozen path at the
freezing commit. Frozen = downstream work packages may rely on it. Changes
require an A-type amendment prompt that re-runs all downstream gates.
vN to vN+1: additive only. Breaking changes: new major row + amendment.

| ID | Contract | Path | Version | Frozen by | Status |
|----|----------|------|---------|-----------|--------|
| C-001 | Failure taxonomy + invariant! | nbe_core::error | v1 | PP-01 | frozen |
| C-002 | Engine tick primitive | nbe_core::tick | v1 | PP-01 | frozen |
| C-003 | Log subscriber init | nbe_core::logging | v1 | PP-01 | frozen |
| C-004 | Virtual clock | nbe_core::clock | v1 | PP-02 | frozen |
| C-005 | Seeded RNG (SplitMix64) | nbe_core::rng | v1 | PP-02 | frozen |
| C-006 | Golden snapshot framework | nbe_core::snapshot | v1 | PP-02 | frozen |
| C-007 | IPC message schema (IpcMessage + IpcEnvelope) | nbe_ipc::message | v1 | PP-03 | frozen |
| C-008 | Length-prefixed frame codec | nbe_ipc::frame | v1 | PP-03 | frozen |
| C-009 | In-process IPC channel pair | nbe_ipc::transport | v1 | PP-03 | frozen |
