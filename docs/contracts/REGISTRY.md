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
