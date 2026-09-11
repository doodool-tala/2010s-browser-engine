# Divergence register

Every deliberate deviation from 2011 Chrome/WebKit behavior. This file IS
the compatibility contract: a behavior that differs and is not listed here
is a bug.

| ID | Area | WebKit-2011 behavior | Our behavior | Rationale | PP |
|----|------|----------------------|--------------|-----------|-----|
| DV-001 | IPC wire format | Chrome 2011: proprietary binary IPC | length-prefixed serde_json frames | debuggable, byte-deterministic, adequate at our scale | PP-03 |
| DV-002 | URL schemes | Chrome 2011 parses all schemes (file, data, javascript, ftp, ...) | v1 parses absolute http/https only; other schemes fail | covered subset grows with the packages that need it | PP-07 |
| DV-003 | IDNA | Chrome 2011 punycodes non-ASCII hosts | ASCII-only hosts; non-ASCII fails to parse | full IDNA needs unicode tables; deferred until the corpus demands it | PP-07 |
| DV-004 | percent-encoding | Chrome 2011 re-encodes as parsing proceeds | v1 validates escapes (two hex digits) but does not normalize; some inputs Chrome accepts are rejected | byte-preserving v1; normalization lands with the resource loader | PP-07 |
