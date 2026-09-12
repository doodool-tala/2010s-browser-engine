# ADR-0011: HTTP/1.1 transport v1
Status: accepted

## Decision
- Hand-rolled HTTP/1.1 over `std::net::TcpStream`, NOT hyper/tokio:
  zero new dependencies (the lockfile discipline bought by A4/A5),
  an era-appropriate surface (GET-only, one TCP connection per hop,
  `Connection: close`), and a fully auditable ~250-line client.
- HttpTransport implements the PP-08 Transport seam exactly; the
  loader never sees a 3xx — redirects resolve inside the transport
  (absolute and root-relative targets, DV-007), capped at
  DEFAULT_MAX_REDIRECTS = 20 hops (Chrome-2011-era value).
- Body framing: Content-Length, chunked transfer decoding, or
  read-to-close — each capped at MAX_BODY_BYTES (64 MiB), because a
  hostile server's Content-Length is untrusted input and must not
  become an allocation.
- Status: 2xx delivers a Resource; 3xx follows Location; everything
  else is a Module error with the code (DV-006); https is a Module
  error until TLS lands (DV-005).
- Requests advertise `Accept-Encoding: identity`; response bytes are
  delivered raw (DV-008). Header names are lowercased on parse;
  lookups are first-match.
- No timeouts: wall-clock is forbidden in engine code (ADR-0003); a
  hung server blocks the synchronous transport, and hang recovery
  belongs to process supervision, not the transport.

## Consequences
- Zero new lockfile entries: this package moves no manifest edges.
- Integration tests run the real client against real loopback TCP
  servers with canned responses — the server records each request
  before responding, so a completion implies its recorded request:
  deterministic assertions over real sockets.
- The golden framework is not used here; unit-level determinism
  lives in the URL and loader packages.
- The divergence register grows to eight entries (DV-005..008).
