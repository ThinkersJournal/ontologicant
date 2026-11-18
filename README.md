# OntologicAnt

Comprehensive tool libraries covering the whole field of ontology and ontological analysis.

**Status: early.** Three crates are built and tested, none published yet: `ontologicant-core` (stable ids, relation types, provenance),
`ontologicant-graph` (a labelled directed multigraph with a documented text form) and `ontologicant-kinds` (taxonomy and mereology with
user-selected axioms, declared relation properties and their closures, every verdict with reasons). The spec is `docs/SPEC-STEP1.md`;
identity and constraints are not built. The crate name `ontologicant` is reserved on crates.io at 0.0.0.

Part of the field-library layer of the AnalysAnt family: these libraries are toolsets that AnalysAnt's lenses (and other projects, such as
SageAnt) use as dependencies. The standing rule (CireSnave, 2026-11-18): if a project needs a tool that falls in this field, the tool is
built here and used as a dependency by the project that needs it.

Licence: MIT OR Apache-2.0.
