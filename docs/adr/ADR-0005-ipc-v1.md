# ADR-0005: IPC v1 — schemas, framing, in-process transport
Status: accepted

## Decision
- The browser-renderer boundary speaks length-prefixed serde_json
  frames: 4-byte big-endian payload length + JSON payload. Our own
  protocol (divergence DV-001), chosen for debuggability and
  byte-determinism.
- Messages are tagged enums (IpcMessage) wrapped in routing envelopes
  (IpcEnvelope) with ProcessId sender and recipient. The browser is
  ProcessId::BROWSER (0); renderers are numbered from 1 by the process
  supervisor (PP-04).
- v1 transport is an in-process bounded channel: each frame is one
  chunk, exercising the exact encode, frame, decode, parse path that
  PP-04's real pipes will carry. Bounded capacity gives backpressure.
- Malformed input from a peer (oversize or zero-length frames, bad
  JSON) is Module-class: tear down the channel and supervise the peer,
  never crash the browser.
- Hard cap: MAX_FRAME_BYTES (16 MiB). Our own sender violating framing
  is an engine invariant violation; a peer violating it is a module
  error. This split is the error taxonomy (ADR-0002) in practice.

## Consequences
- The wire format is byte-deterministic and locked by the golden test:
  any change to field order, enum tags, or framing breaks CI until the
  golden is deliberately re-blessed.
- PP-04 replaces only the byte carrier (pipes), not the codec.
