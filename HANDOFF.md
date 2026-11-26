# HANDOFF: ontologicant (updated 2026-11-21)

You own `C:\Projects\ontologicant` (github.com/ThinkersJournal/ontologicant, public). Read `C:\Projects\CLAUDE.md`, then `docs/SPEC-STEP1.md`.
Sonnet only; never Opus. Primary checkout stays on `main`; branches in worktrees (`C:\Projects\ontologicant-wt-<topic>`). Cluster PM is pm-ants: [ASK]/[STATUS]/[FINDING] there, [READY] to pm-assist cc pm-ants.

## State (2026-11-26T01:25Z)
- Main = 8792a29, 0.0.4. PR #6 open from `day5-next` (pack example, structural corpus, portfolio pin 467fd259/0.48.1 per pm-ants TASK). No version bump (pm-assist allocates). Change note `changelog.d/day5-pack-example-and-corpus.md`.
- Rulings: ONE PR per project per UTC day; own graph store; Equivalent-only alignment; never re-export semanticant_core::Provenance and AssertionSource under one name. NEVER path/git-depend.

## Next
1. #6 is READY (head ec49403, 8/8 pass); no pushes without telling pm-assist. Soft-stopped per pm-ants TASK (no row).
2. After merge: delete branch + worktree, fresh branch; ask pm-ants for a row, else soft-stop.

## Holds / WAITs
WAIT: #6 merged (gate) | check: gh pr view 6 -R ThinkersJournal/ontologicant --json state | then: delete branch day5-next (CLAUDE.md 3) + remove worktree, ask pm-ants for a row
