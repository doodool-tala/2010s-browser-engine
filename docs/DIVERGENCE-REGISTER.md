# Divergence register

Every deliberate deviation from 2011 Chrome/WebKit behavior. This file IS
the compatibility contract: a behavior that differs and is not listed here
is a bug.

| ID | Area | WebKit-2011 behavior | Our behavior | Rationale | PP |
|----|------|----------------------|--------------|-----------|-----|
| DV-001 | IPC wire format | Chrome 2011: proprietary binary IPC | length-prefixed serde_json frames | debuggable, byte-deterministic, adequate at our scale | PP-03 |
