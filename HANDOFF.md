# HANDOFF: ontologicant (updated 2026-11-21)

You own `C:\Projects\ontologicant` (github.com/ThinkersJournal/ontologicant, public). Read `C:\Projects\CLAUDE.md`, then `docs/SPEC-STEP1.md`.
Sonnet only; never Opus. Primary checkout stays on `main`; branches in worktrees (`C:\Projects\ontologicant-wt-<topic>`). Cluster PM is pm-ants: [ASK]/[STATUS]/[FINDING] there, [READY] to pm-assist cc pm-ants.

## State (2026-11-21T00:30Z)
- Main = 8792a29, version 0.0.4 (PR #5 merged by pm-assist: pack loader, alignment propose/structural/json, doc examples, README audit fixes, canonical LICENSE-APACHE). day4-align deleted (remote 404 read back, main control ok), worktree removed.
- Fresh branch `day5-next` (worktree `C:\Projects\ontologicant-wt-next`) at main. Today's PR (UTC 2026-11-21) is spent: next PR not before 2026-11-22T00:00Z, version 0.0.5 lane-carried.
- Rulings: ONE PR per project per UTC day; own graph store; Equivalent-only alignment; never re-export semanticant_core::Provenance and our AssertionSource under one name. NEVER path/git-depend (semanticant 0.1.2 from crates.io).

- Soft-stopped 2026-11-21 (no row; CLAUDE.md no-idle rule; pm-ants memory request). Mail wakes me. On wake: ask pm-ants for a row first; keep committing on `day5-next`.

## Next
1. pm-ants rows done on `day5-next`: pack doctest, structural corpus (measured negative result, scorer unchanged; synthetic corpus, say so). Idle-ask pm-ants for more if time before the PR.
2. On/after 2026-11-22T00:00Z: ONE PR from `day5-next` (signed `- lane: ontologicant`). Do NOT bump the version: pm-assist allocates it at gate time; the change note is `changelog.d/day5-pack-example-and-corpus.md` (no kit in this repo). No pushes after [READY] without telling pm-assist.

## Holds / WAITs
WAIT: day-5 PR may open (UTC day 2026-11-22) | check: date -u +%F | wake: 2026-11-22T00:00:00Z
