# HANDOFF: ontologicant (updated 2026-11-18)

You own `C:\Projects\ontologicant` (github.com/ThinkersJournal/ontologicant, public). Read `C:\Projects\CLAUDE.md`, `docs/IDEAS-AND-DECISIONS-2026-11-18.md`,
`docs/SPEC-SEED-2026-11-18.md`, `docs/SPEC-STEP1.md`. Sonnet only; never Opus. Primary checkout stays on `main`; branches in worktrees (`C:\Projects\ontologicant-wt-<topic>`).

## State
- Survey done (crates.io metadata only; no source or papers read, stated in the spec). `docs/SPEC-STEP1.md` is in the spec-step1 PR; version allocated by the PM at [READY].
- CireSnave approved building the four foundation tools (seed section 7): after the spec PR, build core+graph, kinds, identity, constraints, one PR per day, red-first with sabotage.
- Open [ASK]s are in SPEC-STEP1 section 7 (shared substrate A/B, mereology default, first-consumer date, own store vs petgraph). Do not block on them except `-core` ids/trace.

## Next
1. Send [READY] for the spec PR; after merge remove worktree `ontologicant-wt-spec` and delete branch.
2. Start `ontologicant-graph` (3.1) in a new worktree.

## Holds / WAITs
None.
