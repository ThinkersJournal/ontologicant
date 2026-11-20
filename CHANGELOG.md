# Changelog

## 0.0.4 (not published)

- Added `ontologicant-pack`: reads a `ConstraintSet`, `Graph` or `IdentityIndex` from a data pack already opened and verified by
  `semanticant-pack` 0.1.2 (crates.io), checking the manifest `kind` first and returning each value with a `pack:<name>@<version>`
  `AssertionSource`. Typed `LoadError`; `open_*` helpers open from a directory. Consumes the pack loader, does not copy it.
- `ontologicant-core` now depends on `semanticant-core` 0.1.2 (was 0.1.0).
- Added `ontologicant-alignment` (Step 2, `Equivalent` only): `propose` compares two named graphs (labels plus identity aliases) with a `Matcher`
  and returns `Correspondence` proposals with reasons; `Alignment` is an append-only, retractable overlay (transitive, symmetric `equivalent`;
  text form that replays) and never mutates either graph. `Relation` is `non_exhaustive`; `GraphName` is a validated string until SageAnt fixes
  cross-graph addressing.

## 0.0.3 (not published)

- Added `ontologicant-identity`: `IdentityIndex` (aliases, canonical ids, equivalence classes, merge / split / undo over an append-only
  operation log), the `Matcher` trait with `ExactMatcher` and `NormalisedMatcher`, `MergeProposal` (proposing changes nothing; `apply` is a
  separate logged step), and a line-oriented text form that replays the log.
- Added `ontologicant-constraints`: `ConstraintSet` (disjoint, cardinality, domain, range, functional, inverse-functional, irreflexive) as
  data with a text form, `from_vocabulary`, and `check` returning a `Report` whose every `Violation` has entities and reasons.
  `ontologicant-kinds`: `Vocabulary::decls()` (additive).
- Feature `json` (off by default) on core, graph, identity and constraints: serde derives on the shared types and `to_json` / `from_json` on
  `Graph`, `IdentityIndex` and `ConstraintSet`, validated like the text forms (`from_json` replays the identity log). `GraphError::Json` and
  `IdentityError::Json` added. CI now also runs clippy and the tests with `--all-features`.
- Feature `petgraph` on `ontologicant-graph` (off by default): `Graph::to_petgraph` (a `StableGraph` whose weights keep our ids, labels and relations;
  parallel relations stay separate edges) and `Graph::from_petgraph`. The own store stays the core; this is interop only.

## 0.0.2 (not published)

- Added `ontologicant-kinds`: `Order`, `Taxonomy`, `Mereology` (axioms only when selected), `Vocabulary` and `Closure`.
- **Breaking:** `ontologicant_core::Provenance` is renamed `AssertionSource`, and `Relation.provenance` is now `Relation.assertion`.
  `semanticant_core::Provenance` is a data-licence record; ours is an assertion's origin, so the two never share a name.
- `Reason` and `Verdict<T>` now come from `semanticant-core` 0.1.0 (replacing the private shim). Minimum Rust version is 1.88.

## 0.0.1 (not published)

- `ontologicant-core` and `ontologicant-graph`: stable ids, relation types, a labelled directed multigraph and its text format.
