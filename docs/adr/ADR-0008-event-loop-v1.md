# ADR-0008: Renderer event loop v1
Status: accepted

## Decision
- The renderer runs a single-threaded event loop with two ready
  lanes — urgent (input priority, drained fully each turn) and normal
  (one task per turn) — plus a delayed lane keyed by virtual Tick.
- Virtual time is the only time: the clock advances exactly when the
  loop would otherwise idle, jumping to the earliest due task.
  Same-due tasks run in queueing order (seq). Blocking on IPC pauses
  virtual time; between messages the loop drains to full idle.
- A turn that ran any task performs exactly one "update the
  rendering" (counted; the render pipeline itself arrives with layout
  and paint).
- Tasks are 'static FnOnce closures receiving their turn's context
  handle by value; a task can read the tick and queue follow-ups,
  but cannot advance time or re-enter the loop.
- The renderer binary drives the loop from IPC: Initialize replies
  via a normal task, Ping replies ride the urgent lane, Shutdown
  drains and exits cleanly.

## Consequences
- Scheduling semantics are locked byte-exact by the golden test: any
  change to lane priority, promotion, or time advancement breaks CI
  until deliberately re-blessed.
- The PP-04 renderer integration tests passing unchanged is the
  behavior-preservation contract for this refactor.
- A self-perpetuating urgent task starves the loop by design (as in
  real engines); supervision at a higher layer owns such
  pathologies.
