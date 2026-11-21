# HANDOFF: ontologicant (updated 2026-11-20)

You own `C:\Projects\ontologicant` (github.com/ThinkersJournal/ontologicant, public). Read `C:\Projects\CLAUDE.md`, then `docs/SPEC-STEP1.md`.
Sonnet only; never Opus. Primary checkout stays on `main`; branches in worktrees (`C:\Projects\ontologicant-wt-<topic>`). Cluster PM is pm-ants: [ASK]/[STATUS]/[FINDING] there, [READY] to pm-assist cc pm-ants.

## State (2026-11-21T00:10Z)
- Main = 6826e59 (0.0.3). PR #5 open from `day4-align` (worktree `C:\Projects\ontologicant-wt-align`), head 3752fea, 8/8 checks green, 0 threads; [READY] sent to pm-assist cc pm-ants. Carries pack loader, alignment (propose, structural, json), doc examples, README audit fixes, canonical LICENSE-APACHE (8 copies, pm-ants apache-text-fix), version 0.0.4. NEVER path/git-depend (semanticant 0.1.2 from crates.io).
- Rulings: ONE PR per project per UTC day; own graph store; Equivalent-only alignment; never re-export semanticant_core::Provenance and our AssertionSource under one name. Versions lane-carried. Primary checkout: `git checkout -- HANDOFF.md` before ff-merging.

## Next
1. After #5 merges: delete branch (local+remote, 404 read back with main control), `git worktree remove` + prune, fresh branch; ff primary. Then ask pm-ants for the next row (optional: pack-crate doc example, structural-matcher tuning). No new PR before 2026-11-22 UTC.

## Holds / WAITs
WAIT: PR #5 merged | check: gh pr view 5 --repo ThinkersJournal/ontologicant --json state --jq .state | wake: 2026-11-21T01:10:00Z
