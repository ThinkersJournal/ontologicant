# Changelog

## 0.0.3 (not published)

- Added `ontologicant-identity`: `IdentityIndex` (aliases, canonical ids, equivalence classes, merge / split / undo over an append-only
  operation log), the `Matcher` trait with `ExactMatcher` and `NormalisedMatcher`, `MergeProposal` (proposing changes nothing; `apply` is a
  separate logged step), and a line-oriented text form that replays the log.

## 0.0.2 (not published)

- Added `ontologicant-kinds`: `Order`, `Taxonomy`, `Mereology` (axioms only when selected), `Vocabulary` and `Closure`.
- **Breaking:** `ontologicant_core::Provenance` is renamed `AssertionSource`, and `Relation.provenance` is now `Relation.assertion`.
  `semanticant_core::Provenance` is a data-licence record; ours is an assertion's origin, so the two never share a name.
- `Reason` and `Verdict<T>` now come from `semanticant-core` 0.1.0 (replacing the private shim). Minimum Rust version is 1.88.

## 0.0.1 (not published)

- `ontologicant-core` and `ontologicant-graph`: stable ids, relation types, a labelled directed multigraph and its text format.
