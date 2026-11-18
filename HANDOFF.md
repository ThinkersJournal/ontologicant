# HANDOFF: ontologicant (updated 2026-11-18)

You own `C:\Projects\ontologicant` (github.com/ThinkersJournal/ontologicant, public). Read `C:\Projects\CLAUDE.md`, then `docs/SPEC-STEP1.md`.
Sonnet only; never Opus. Primary checkout stays on `main`; branches in worktrees (`C:\Projects\ontologicant-wt-<topic>`).

## State
- Spec merged (#1). PM rulings 2026-11-18: shared types come from `semanticant-core` (Reason, Verdict, Provenance, pack format) as a crates.io dependency once the PM publishes it
  (semanticant Day 1, 11-19); until then `ontologicant-core` holds a shim Reason/Verdict. Swap before `-core` grows. Mereology default profile: none. Own graph store; petgraph adapter later. Alignment is Step 2.
- Day 1 PR (branch `day1-core-graph`, worktree `ontologicant-wt-graph`): `ontologicant-core` + `ontologicant-graph` 0.0.1, 13 tests, 4 sabotages caught, CI copied from analysant, SPDX floor 4.
  Version allocated by the PM at [READY].

## Next (one PR per day)
1. After Day 1 merges: remove worktree + branch. Day 2: `ontologicant-kinds` (taxonomy closure/cycles, part-of profiles, relation properties).
2. Then `-identity`, then `-constraints`. Swap the shim for `semanticant-core` when published.

## Holds / WAITs
None.
