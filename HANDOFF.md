# HANDOFF: ontologicant (updated 2026-11-18)

You own `C:\Projects\ontologicant` (github.com/ThinkersJournal/ontologicant, public). Read `C:\Projects\CLAUDE.md`, then `docs/SPEC-STEP1.md`.
Sonnet only; never Opus. Primary checkout stays on `main`; branches in worktrees (`C:\Projects\ontologicant-wt-<topic>`).

## State
- Merged: spec (#1), Day 1 core + graph (#2, 0.0.1). Branch protection is set by the PM (build/test ubuntu+windows, licence gate, cargo deny).
- Day 2 PR (branch `day2-kinds`, worktree `ontologicant-wt-kinds`): `ontologicant-kinds` 0.0.1 (Order, Taxonomy, Mereology with user-selected axioms, Vocabulary/Closure) and the shim swap:
  `Reason`/`Verdict<T>` now re-exported from `semanticant-core` 0.1.0 (crates.io). MSRV raised to 1.88 (semanticant-core's). Our `Provenance` was renamed `AssertionSource` (Relation field `assertion`) in this PR: it is assertion origin, semanticant's is a data-licence record; never re-export both under one name (PM ruling).
- Rulings: mereology default none; own graph store, petgraph adapter later; alignment is Step 2.

## Next (ONE PR per UTC day, counted by day OPENED; PM 2026-11-18: do not open a PR before its day)
0. #3 is a DRAFT opened 11-18 as a one-off the PM accepted: on 11-19 UTC read its CI, mark it ready, send [READY]. Day 3 work (identity) goes on a branch off day2-kinds; open its PR on 11-20 or later, after #3 merges.
1. After Day 2 merges: remove worktree + branch. Day 3: `ontologicant-identity` (aliases, equivalence classes, merge/split/undo with provenance).
2. Then `-constraints` (disjointness, cardinality, domain/range, functional/inverse-functional declared in kinds).

## Holds / WAITs
None.
