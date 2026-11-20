# Step 2 design note: alignment (a note, not a spec)

Status: draft by the ontologicant lane, 2026-11-19, against branch `day3-identity` (identity, constraints, json and petgraph features built).
Nothing here is implemented. Items marked **[unverified]** are guesses to confirm; the SageAnt "X4 and X5" text named in
`SPEC-STEP1.md` section 4 does not exist yet (see 3a), so cross-graph addressing stays the biggest unknown.

## 1. What alignment means here

Identity (`ontologicant-identity`) answers "are these two entities in one graph the same?". Alignment answers the same question
**across two graphs or vocabularies**, plus a second one identity never asks: "does relation type R in graph A correspond to relation
type S in graph B, and kind K to kind L?" (Euzenat and Shvaiko, *Ontology Matching*, call the result an alignment: a set of
correspondences, each with a relation and a confidence.)

## 2. Proposed shape (built on what exists)

- `Correspondence { left: Ref, right: Ref, relation: Equivalent | Narrower | Broader | Disjoint, score: f64, reasons: Vec<Reason> }`.
  Same discipline as `MergeProposal`: **a correspondence is a proposal with reasons; accepting one is a separate, logged step.**
- `Ref` names a thing in a named graph: `(GraphName, EntityId)` for entities, `(GraphName, RelationType)` for relation types.
  Entity ids are only unique within one graph, so a cross-graph reference needs the graph name. **[unverified]** whether SageAnt's
  X4/X5 already fixes such a scheme; if so, adopt it rather than invent one.
- Matching reuses the `Matcher` trait from the identity crate unchanged (exact, normalised; fuzzy or model matchers stay optional
  escalations). Alignment adds *structural* matchers on top, which need only the graph and taxonomy we already have:
  shared neighbours, shared ancestors in the taxonomy, equal relation-type usage.
- An accepted alignment is data (a text form and the `json` feature, like the other crates), with an append-only log and undo, as in
  `IdentityIndex`. **[unverified]** whether it should reuse that log type or stay separate; reuse is attractive but couples two crates.
- Consistency: an alignment can be checked against both graphs' constraints (`ConstraintSet::check`), e.g. `Equivalent` between
  two kinds that one graph declares `Disjoint` with an ancestor of the other is a violation with a reason trace.

## 3. Open questions

1. Does anything downstream need `Narrower`/`Broader`/`Disjoint`, or only `Equivalent`? (AnalysAnt's lens text names "how they relate"
   within one graph; it does not mention cross-graph alignment.) If only `Equivalent` is needed, alignment is mostly identity with a
   graph name on each side, and Step 2 should be that small.
2. Where does the graph-name scheme live? Core (all tools share it), or a new `-alignment` crate that owns it?
3. Does the result of accepting a correspondence feed back into identity (merge across graphs) or stay a separate overlay? Overlay is
   safer: it never mutates either graph, and it can be undone by deleting the overlay.

## 3a. Answer received: SageAnt X4/X5 (2026-11-20)

Source: the pm-ants lane relaying SageAnt, 2026-11-20 (SageAnt ROADMAP rows X4/X5; `docs/PROPOSAL-expansion-2026-11-18.md` item 3).

- **No X4/X5 design text exists.** Both rows are Phase 2, planned and not started; a design note follows only after X1-X3.
- X4: a link target is another graph plus a chunk, addressed by (graph id, chunk id, version, content hash); an untrusted stub.
- X5: a chunk version referenced from a shared collection under existing grants.
- SageAnt specifies **no alignment**. Its reading, stated as not a decision: only "same chunk in two graphs" (equal content hash at a version) means `Equivalent`, at most.

Consequence: question 1 below is answered for now as **`Equivalent` only**, with the graph-name scheme swappable until X4 fixes an address.

## 4. Not proposed

No model or learned component in core, no SPARQL, no OWL reasoner (as in `SPEC-STEP1.md` section 2 non-goals). `oxrdf` and `fastobo`
are named there as adapter candidates for importing existing ontologies; they are not needed to design alignment itself.

## 5. Recommendation

Wait for answers to question 1 and for the X4/X5 text before writing code. The smallest useful first step, if a consumer asks, is
`Equivalent`-only alignment over `(GraphName, EntityId)` pairs reusing `Matcher`, `MergeProposal`-style proposals and the text/json forms.
