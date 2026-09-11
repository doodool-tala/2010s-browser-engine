# ADR-0009: URL parser and origin v1
Status: accepted

## Decision
- parse_url parses ABSOLUTE http/https URLs via a structural scan
  (scheme, authority, path, query, fragment), not the WHATWG
  streaming state machine: the state machine exists for base-relative
  resolution, which arrives later; for absolute URLs the two are
  equivalent, and the structural form is shorter and auditable.
- Hosts: Domain (ASCII, lowercased, per-label validation), Ipv4 (the
  full WHATWG number parser: hex/octal/decimal parts, short forms,
  trailing dot), Ipv6 (full grammar: 4-hex-digit pieces, one "::"
  standing for at least one zero piece, embedded IPv4). IPv6
  serialization follows RFC 5952 (leftmost longest zero run).
- Normalizations: scheme and host lowercased; default ports (80/443)
  removed; empty path becomes "/"; dot segments removed; tabs and
  newlines stripped; leading/trailing C0+space trimmed. Userinfo
  splits at the LAST '@' and is kept raw (a raw '@' inside userinfo is
  accepted; round-trips by construction).
- parse_url returns Option<Url>: the RecoverableInput class (ADR-0002)
  exercised for real. The URL layer deliberately uses NO invariant! —
  every failure is input-class by design.
- Origin is the (scheme, host, port) tuple with default ports already
  stripped, so http://a/ and http://a:80/ share an origin.

## Consequences
- A golden test locks parse+serialize behavior byte-exact; any change
  to normalization or host handling breaks CI until re-blessed.
- Same-origin policy (later packages) is built on Origin equality,
  never URL string comparison.
- v1 out of scope (divergences DV-002..004): non-http(s) schemes,
  relative-URL resolution, IDNA, percent-encoding normalization.
- The eventual external corpus is the WHATWG urltestdata.json
  subset, which becomes a future gate once normalization and base
  resolution land.
