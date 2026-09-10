# ADR-0006: Process model v1
Status: accepted

## Decision
- The supervisor spawns real OS processes (nbe-renderer) and owns
  their stdio pipes; the PP-03 codec rides the pipes unchanged.
- The renderer binary v1 is a protocol placeholder: Initialize →
  Ready, Ping → Pong, Shutdown → exit 0. The event loop and render
  pipeline arrive at PP-06.
- Crash = non-zero exit or closed stdout, detected via try_wait;
  restart decisions are the pure should_restart(RestartPolicy)
  function. DEFAULT: 3 attempts, then the tab is sad.
- Fault injection for tests: NBE_RENDERER_FAULT=exit-early. Test-only,
  never set by engine code.
- Renderer stderr is inherited: logs flow to the parent's console.

## Consequences
- The one-crash-one-tab architecture is now physically real and
  integration-tested end to end.
- Renderer pids are allocated from 1 upward by the supervisor.
- Heartbeat-deadline supervision (ping timeouts) is deferred to the
  event loop package; exit detection covers v1.
