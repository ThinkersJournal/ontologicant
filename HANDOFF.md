# HANDOFF: ontologicant (updated 2026-11-20)

You own `C:\Projects\ontologicant` (github.com/ThinkersJournal/ontologicant, public). Read `C:\Projects\CLAUDE.md`, then `docs/SPEC-STEP1.md`.
Sonnet only; never Opus. Primary checkout stays on `main`; branches in worktrees (`C:\Projects\ontologicant-wt-<topic>`). Cluster PM is pm-ants: [ASK]/[STATUS]/[FINDING] there, [READY] to pm-assist cc pm-ants.

## State (2026-11-20T01:10Z)
- Main = 6826e59, version 0.0.3 (PR #4 merged 00:21Z by pm-assist: identity, constraints, json + petgraph features, step-2 note). Old branch/worktree deleted (remote 404 read back, main control ok). Nothing published; consumers ask first.
- Branch `day4-align` (worktree `C:\Projects\ontologicant-wt-align`), pushed, commits: STEP2 note records SageAnt X4/X5 answer (no text exists; Equivalent-only default, scheme swappable). NO PR until 2026-11-21 UTC (one PR per UTC day, #4 was today's).
- Rulings: ONE PR per project per UTC day; limits PRs, not work. Mereology default none; own graph store; never re-export semanticant_core::Provenance and our AssertionSource under one name. Versions lane-carried.
- semanticant 0.1.2 published (pm-ants NOTICE 2026-11-20). `ontologicant-pack` built on `day4-align` (constraints/graph/identity from a verified pack, 9 tests, one sabotage caught); workspace is 0.0.4. NEVER path/git-depend.

## Next
1. Pack loader done. Next on `day4-align`: Step-2 `Equivalent`-only alignment over (GraphName, EntityId), reusing Matcher/MergeProposal-style proposals and text/json forms; keep the graph-name scheme swappable. Commit and push freely.
2. On/after 2026-11-21T00:00Z: ONE PR from `day4-align` (signed `- lane: ontologicant`), version 0.0.4, then [READY].

## Holds / WAITs
WAIT: day-4 PR may open (UTC day 2026-11-21) | check: date -u +%F | wake: 2026-11-21T00:00:00Z
