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
| DV-005 | https fetches | Chrome 2011 loads https over TLS | v1 returns a Module error ("TLS not yet supported") | rustls lands in a later package | PP-09 |
| DV-006 | non-2xx responses | Chrome 2011 delivers the response body; error pages render it | v1 returns a Module error with the status; the body is discarded | error-page machinery arrives with navigation | PP-09 |
| DV-007 | redirect targets | Chrome 2011 resolves any relative reference (RFC 3986) | v1 follows absolute and root-relative locations only | full base resolution lands with the HTML base-URL machinery | PP-09 |
| DV-008 | content decoding | Chrome 2011 decodes charsets and decompresses encodings | v1 requests identity encoding and delivers raw bytes; the charset parameter is stored but unused | decoding lands with DOM string plumbing | PP-09 |
