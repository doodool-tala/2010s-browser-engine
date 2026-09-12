# ADR-0012: Cookie jar v1
Status: accepted

## Decision
- The jar implements RFC 6265's v1 subset: §5.2 Set-Cookie parsing,
  §5.1.3 domain matching, §5.1.4 path matching, §5.3 storage rules
  (host-only vs Domain cookies, leading-dot normalization, default
  paths), §5.4 retrieval ordering (longer paths, then earlier
  creation), and Max-Age-over-Expires precedence.
- Time is a parameter: every store/retrieve takes `now` in epoch
  seconds. Wall-clock never enters engine code (ADR-0003); the
  browser process supplies real time at its boundary, tests supply
  fixed values, and the default is 0.
- The jar lives INSIDE HttpTransport (the v1 Transport seam cannot
  carry outgoing headers, so a loader-level jar would be dead code).
  Consequences: zero contract amendments for the seam, cookies
  absorb naturally at every redirect hop, and the architecture
  matches Chrome's — the network stack owns the cookie store.
  C-025 gains two additive builder methods (v2).
- Caps: 3000 total, 50 per domain; on every store, expired cookies
  are swept first, then the oldest by creation sequence. Determinism
  holds throughout: eviction order is a pure function of inputs.
- Malformed Set-Cookie is RecoverableInput (ADR-0002): silently
  dropped, never an error, never a crash.
- Public-suffix checking uses a built-in subset (DV-009). HttpOnly is
  stored but not enforced — no non-HTTP accessor exists yet.

## Consequences
- The jar's behavior is locked byte-exact by the golden test; the
  integration test proves cookies flow over real sockets.
- document.cookie (later, with the DOM) will read this jar through
  the eventual renderer↔browser IPC.
- Session cookies are memory-only by design; persistence is a later
  package.
