# OntologicAnt: SPEC-STEP1 (lane draft of 2026-11-18, for CireSnave to read as it happens)

Expands `SPEC-SEED-2026-11-18.md`. Marks: **[said]** CireSnave, **[PM]** PM seed, **[lane]** this lane's proposal, **[unverified]** not checked.
Docs only; no code in this PR.

## 1. Survey: what was read and what was only seen

**Honest scope of the survey.** Done: crates.io search and metadata (name, version, licence, MSRV, repository, downloads, description) for the
candidates below, read through the crates.io API on 2026-11-18. **Not done:** reading any crate's source, and reading any paper. The
literature entries are cited from the lane's memory and are **[unverified]** until read. Nothing below is a quality judgement of a crate's code.

| Family | Candidate (version, licence) | Seen | Verdict **[lane]** |
|---|---|---|---|
| graph structure/algorithms | `petgraph` 0.8.3, MIT OR Apache-2.0, 625M downloads | metadata | Optional interop adapter behind a feature; not the core store (see 3.1) |
| union-find | `ena` 0.14.4, MIT OR Apache-2.0 | metadata | Not used: identity needs split, undo and provenance, which a plain union-find does not give |
| RDF terms | `oxrdf` 0.3.5, MIT OR Apache-2.0, MSRV 1.87 | metadata | Candidate for an RDF import/export adapter crate, not the core |
| OWL 2 RL reasoning | `reasonable` 0.5.0-a1 (BSD-3-Clause); `oxirs-rule` 0.4.1 (Apache-2.0); `purrdf-entail` 4.0.2 (MIT/Apache/MulanPSL, MSRV 1.98) | metadata | Out of the core (rules over RDF, not our model); revisit for an optional adapter |
| SHACL validation | `purrdf-shapes` 4.0.2 | metadata | Prior art for constraint reports; not a dependency |
| OBO format | `fastobo` 0.15.5, MIT | metadata | Candidate importer for biomedical ontologies |
| RDF framework | `rdf-model` 0.5.0 (Unlicense) | metadata | Not needed |
| string similarity | `strsim` 0.11.1, MIT | metadata | Candidate for alias matching (optional, deterministic) |
| Unicode text | `unicode-normalization` (MIT/Apache AND Unicode-3.0), `unicode-segmentation` | metadata | Candidates for alias normalisation; coordinate with SemanticAnt |

**Found nothing** for mereology, taxonomy-with-closure, ontology alignment or entity resolution as dedicated crates: crates.io searches for
"mereology" returned 0 results; "taxonomy", "entity resolution", "record linkage" and "ontology alignment" returned only unrelated crates. Positive control: the
same search method finds `petgraph`, `oxrdf` and `reasonable`, which exist. These searches are name/description matches only, so a crate that
does the job but is described differently would be missed **[unverified]**.

**Literature [unverified, from memory, to be read before the first release]:** mereology (Casati and Varzi, *Parts and Places*; Simons, *Parts*);
identity criteria and OntoClean (Guarino and Welty); ontology matching (Euzenat and Shvaiko, *Ontology Matching*); record linkage (Fellegi and Sunter 1969);
OWL 2 profiles and SKOS/SHACL as W3C references; description-logic classification (Baader et al., *The Description Logic Handbook*).

**Reuse rule applied:** reuse a good permissive crate where one exists, keeping licence and credit. The survey found no crate that fits the
core model, so the four foundation tools are written here, with `petgraph`, `oxrdf`, `fastobo` and `strsim` as adapter/optional candidates.

## 2. Scope and non-goals

In scope **[PM]**: entities, kinds, instances versus types, identity, parthood, relations and their properties, taxonomies, alignment,
consistency, entity resolution. Step 1 builds the four foundation tools (section 3); alignment and richer reasoning are Step 2.

Non-goals **[lane]**: no model or learned component in the core; no triple store or query language (SPARQL) of our own; no full OWL/DL
reasoner; no natural-language processing (that is SemanticAnt); no evidence or justification (EpistemologicAnt); not a lens (AnalysAnt).

## 3. The four foundation tools: confirmed, with corrections

The PM's list is confirmed as the right foundation **[lane]**. Corrections: relation properties belong with the graph's vocabulary, not only tool 2, and
reasons (tool 4) need a shared trace type used by all four. Crates: `ontologicant-core` (types, ids, provenance, reason trace) plus
`ontologicant-graph`, `-kinds`, `-identity`, `-constraints`. One version for all **[said, portfolio rule]**.

Every API below is a **sketch**, not a promise; names will change on contact with tests.

### 3.1 Relation graph (`ontologicant-graph`)
Labelled directed multigraph; stable `EntityId`/`RelationId` that are never reused; deterministic iteration order (sorted by id); documented
serialisation (a line-oriented text form plus JSON via a feature); in-memory reference store behind a `GraphStore` trait so other stores can exist.
Why not `petgraph` as the store: stable ids that survive removal, deterministic order and attached provenance are required, and `petgraph`'s
indices are not stable across removal in `Graph` (`StableGraph` exists) **[unverified, read the docs first]**. Decide when the code is written.
```rust
let mut g = Graph::new();
let a = g.add_entity("tabby");               // -> EntityId
let r = g.add_relation(a, RelationType::new("is-a"), b, Provenance::asserted("source-1"));
for e in g.neighbours(a, Direction::Out) { /* (RelationId, EntityId) in id order */ }
```

### 3.2 Kinds and relations (`ontologicant-kinds`)
Relation properties declared as data: `Transitive, Symmetric, Reflexive, Irreflexive, Functional, InverseFunctional, Inverse(of)`. Taxonomy (is-a) closure
and cycle detection; part-of with the standard mereological axioms as *selectable profiles* (none, weak supplementation, extensional), since mereology
is contested and the tool must serve every plausible use. Closures return the derivation path.
```rust
let tax = Taxonomy::from_graph(&g, RelationType::new("is-a"))?;   // Err(Cycle{path}) names the cycle
tax.ancestors(cat); tax.is_a(tabby, animal) -> Verdict { holds, reasons: Trace }
```

### 3.3 Identity (`ontologicant-identity`)
Alias tables, canonical id, equivalence classes with merge and split that keep provenance and can be undone (an append-only operation log;
undo replays inverse). Matching is a trait: exact and normalised-string matchers ship; fuzzy and model matchers are optional escalations behind the trait.
Merges are *proposals with scores and reasons*; applying one is a separate, logged step.
```rust
let mut id = IdentityIndex::new();
id.add_alias(e, "Felis catus", Source::new("pack-x"));
let p = id.propose_merges(&Matcher::normalised())?;   // Vec<MergeProposal{a,b,score,reasons}>
let op = id.merge(a, b, why)?; id.undo(op)?;
```

### 3.4 Constraints and consistency (`ontologicant-constraints`)
Disjointness, cardinality, domain/range, functional/inverse-functional checks over a graph plus its taxonomy. Returns `Report { violations }` where every
violation carries the constraint, the entities, and the reason trace. Constraint sets are data (loaded from the data-pack format), not code.

## 4. Seams and the shared substrate

- **Consumers.** AnalysAnt (`analysant-ontological`, today a placeholder crate with "no API yet") and SageAnt. Both consume the four tools through the
  traits above. A consumer's needs are test cases, never the shape of the API **[said]**.
- **What the first consumer needs, and when: [unverified].** AnalysAnt's requirements doc describes the ontological lens as "what entities the text is
  about and how they relate: the DAG, the ROADMAP, the lane, the hook; the knowledge-graph view" (analysant `docs/REQUIREMENTS-2026-11-16.md`, line 43, origin/main).
  That implies tools 3.1, 3.2 and 3.3 first. No date exists on either side; **[ASK]** below. SageAnt's "X4 and X5" (cross-graph addressing) text was not
  found on SageAnt's origin/main by grep; the lane has not seen it, so alignment is placed in Step 2 pending that text.
- **Shared substrate.** Needed by all three field projects: stable identifiers, provenance record, reason trace, data-pack format. Proposal **[lane]**: PM's order
  SemanticAnt <- OntologicAnt <- EpistemologicAnt is workable only if the *reason trace* and *provenance* types live in the lowest crate that needs them. Since
  OntologicAnt cannot depend on a later project, option A: define them in `ontologicant-core`, and the sibling that sits below must not need them;
  option B: a tiny fourth crate (`ant-trace`, versioned separately) that all three depend on. The lane recommends B, because A forces EpistemologicAnt to
  depend on a graph crate just to report a reason. The sibling specs were not yet written when this was drafted, so this is unreconciled.

## 5. Principles applied
Generic over use (section 3 APIs name no consumer); deterministic, no model in core; every verdict has reasons; `#![forbid(unsafe_code)]`; std-only
core with optional adapters; language as data (relation names and labels are plain strings, never English enums); permissive licences only (all candidates
above are MIT/Apache/BSD/Unlicense; `purrdf-*` MSRV 1.98 is noted); every README claim built and tested.

## 6. Build plan
Spec PR (this) -> foundation tools, one PR per day, red-first tests with sabotage, versions allocated by the PM at [READY]. Order: core + graph,
then kinds, then identity, then constraints. Anything beyond the four waits for the spec to be read **[PM]**.

## 7. Decisions that need CireSnave
- **[ASK]** Shared substrate: option A (in `ontologicant-core`) or B (separate `ant-trace` crate)? Recommend B. Blocks: any code in `-core`.
- **[ASK]** Mereology default profile: none (user selects) or extensional? Recommend none. Blocks: 3.2 API.
- **[ASK]** First consumer's date for tools 3.1-3.3 (AnalysAnt) and the X4/X5 text (SageAnt). Blocks: ordering of Step 2.
- **[ASK]** Own graph store versus `petgraph`-backed: recommend own core, `petgraph` adapter later. Blocks: 3.1.

## 8. Unverified
All literature (section 1); every crate's source and real quality; `petgraph` index stability; whether a crate exists that name searches missed; the first
consumer's actual needs and dates; the sibling specs; MSRV and edition to target (raise as needed per standing rule).

— lane: ontologicant
