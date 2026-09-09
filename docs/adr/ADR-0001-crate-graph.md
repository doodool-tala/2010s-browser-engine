# ADR-0001: Workspace crate graph
Status: accepted

## Decision
One Cargo workspace, 15 crates, prefix nbe-, tiers with one-way deps (see
docs/architecture/CRATE-GRAPH.md). Process isolation is enforced by
dependency direction plus a mechanical check, not convention: nbe-shell may
never link web-side crates; renderers are separate OS processes.

## Consequences
- Chrome-2011's one-crash-one-tab guarantee is structural from day one.
- scripts/check-g1.sh fails the build if nbe-shell gains forbidden deps.
