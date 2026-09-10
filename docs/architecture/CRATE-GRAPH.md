# Crate graph

15 crates under crates/, prefix nbe-. One-way tier dependencies; the table
of crates, missions, and allowed dependencies is the one in §6 of
docs/pp/PP-01.md (copy it here verbatim, all columns).

Two process boundaries, enforced structurally:
1. nbe-shell (browser process) links ONLY core/ipc/net/util. It never links
   dom/layout/paint/js. Enforced mechanically by scripts/check-g1.sh.
2. nbe-renderer is spawned by nbe-shell as a separate OS process. A
   renderer crash kills one tab (the sad-tab); the browser survives.
   This is Chrome 2011's architecture.

Renderer dependency row (from §6 of docs/pp/PP-01.md; PP-04 records the
ipc dependency here):

| Crate | MISSION | PP-XX | May depend on |
|---|---|---|---|
| nbe-renderer | Renderer process: composes the web-side crates per tab; owns the event loop | PP-06 | core, ipc, and all web-side crates |
