# ADR-0002: Error taxonomy
Status: accepted

## Decision
Three failure classes (nbe_core::error::FailureClass):
- RecoverableInput: malformed HTML/CSS/URL/JS. NEVER Result::Err. Parsers
  recover per spec and record diagnostics. Browsers never die on bad input.
- Module: infrastructure failures (network, IO, closed IPC). Result::Err.
- InvariantViolation: engine logic bug. Raised ONLY via invariant!. The
  process dies; the renderer becomes the sad tab.

## Consequences
- unwrap/expect/panic are clippy-denied in all non-test code.
- Exactly one function in the workspace contains panic!, auditable by grep.
