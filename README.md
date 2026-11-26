# OntologicAnt

Comprehensive tool libraries covering the whole field of ontology and ontological analysis.

**Status: early.** Seven crates are built and tested, none published yet (checked on crates.io 2026-11-20: only the `ontologicant` 0.0.0 placeholder exists):

- `ontologicant-core`: stable ids, relation types, assertion sources.
- `ontologicant-graph`: a labelled directed multigraph with a documented text form; optional `petgraph` interop.
- `ontologicant-kinds`: taxonomy and mereology with user-selected axioms, declared relation properties and their closures, every verdict with reasons.
- `ontologicant-identity`: aliases, canonical ids, merge / split / undo over an append-only log, matchers.
- `ontologicant-constraints`: disjointness, cardinality, domain/range, functional checks, each violation with reasons.
- `ontologicant-pack`: reads constraint sets, graphs and identity logs from a verified SemanticAnt data pack (`semanticant-pack` from crates.io).
- `ontologicant-alignment`: `Equivalent`-only correspondences between named graphs (name matching, and shape-only structural proposals given anchors), accepted as a retractable overlay.

The spec is `docs/SPEC-STEP1.md`. The core, graph, identity, constraints and alignment crates have an optional `json` feature (`to_json` / `from_json`) that round-trips and validates like the text forms; kinds and pack have none. The graph, kinds, identity, constraints, pack and alignment crates have a crate-level example that runs as a doc test; the code sketches in the spec are design pseudo-code, not compiled. The crate name `ontologicant` is reserved on crates.io at 0.0.0.

Part of the field-library layer of the AnalysAnt family: these libraries are toolsets that AnalysAnt's lenses (and other projects, such as
SageAnt) use as dependencies. The standing rule (CireSnave, 2026-11-18): if a project needs a tool that falls in this field, the tool is
built here and used as a dependency by the project that needs it.

Licence: MIT OR Apache-2.0.
