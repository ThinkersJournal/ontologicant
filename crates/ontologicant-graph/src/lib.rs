// SPDX-License-Identifier: MIT OR Apache-2.0
//! `ontologicant-graph`: a labelled, directed multigraph of entities with stable ids.
//!
//! * Ids are never reused, including across [`Graph::to_text`] / [`Graph::from_text`].
//! * Every listing is in id order, so output is deterministic.
//! * [`GraphStore`] is the trait other stores can implement; [`Graph`] is the in-memory reference store.
//!
//! # JSON (feature `json`)
//! [`Graph::to_json`] / [`Graph::from_json`] carry the same information as the text form, as
//! `{"version":1,"next_entity":n,"next_relation":n,"entities":[{"id":0,"label":".."}],"relations":[{"id","from","rtype","to","assertion":{"source","origin":"Asserted"|"Derived"}}]}`,
//! checked by the same invariants (unique ids, declared endpoints, ids below the counters).
//!
//! # Text format (version 1)
//! Line oriented, fields separated by a TAB, `\\`, `\t`, `\n` and `\r` escaped inside a field:
//! ```text
//! ontologicant-graph 1
//! N <next entity number> <next relation number>
//! E <entity id> <label>
//! R <relation id> <from> <to> <A|D> <relation type> <source>
//! ```
//! # Example
//! ```
//! use ontologicant_graph::{AssertionSource, Graph, GraphStore, RelationType};
//!
//! let mut g = Graph::new();
//! let cat = g.add_entity("cat");
//! let animal = g.add_entity("animal");
//! g.add_relation(cat, RelationType::new("is-a"), animal, AssertionSource::asserted("example"))
//!     .unwrap();
//! let back = Graph::from_text(&g.to_text()).unwrap();
//! assert_eq!(back.to_text(), g.to_text());
//! assert_eq!(back.entities().count(), 2);
//! ```
//!
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub use ontologicant_core::{AssertionSource, EntityId, Origin, RelationId, RelationType};

/// Which relations of an entity to follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Relations that start at the entity.
    Out,
    /// Relations that end at the entity.
    In,
    /// Both.
    Both,
}

/// One relation instance.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
pub struct Relation {
    /// Its id.
    pub id: RelationId,
    /// Source entity.
    pub from: EntityId,
    /// The kind of relation.
    pub rtype: RelationType,
    /// Target entity.
    pub to: EntityId,
    /// Where it came from.
    pub assertion: AssertionSource,
}

/// Errors from graph operations and parsing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphError {
    /// No entity with this id.
    NoSuchEntity(EntityId),
    /// No relation with this id.
    NoSuchRelation(RelationId),
    /// The text form is malformed.
    Parse {
        /// 1-based line number.
        line: usize,
        /// What is wrong.
        message: String,
    },
    /// The JSON form is malformed or breaks a graph invariant.
    Json(String),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::NoSuchEntity(id) => write!(f, "no such entity {id}"),
            GraphError::NoSuchRelation(id) => write!(f, "no such relation {id}"),
            GraphError::Parse { line, message } => write!(f, "line {line}: {message}"),
            GraphError::Json(m) => write!(f, "json: {m}"),
        }
    }
}
impl std::error::Error for GraphError {}

/// What a store of entities and relations must offer. Other stores can implement it.
pub trait GraphStore {
    /// Add an entity; its id is never reused.
    fn add_entity(&mut self, label: &str) -> EntityId;
    /// Add a relation between two existing entities.
    fn add_relation(
        &mut self,
        from: EntityId,
        rtype: RelationType,
        to: EntityId,
        assertion: AssertionSource,
    ) -> Result<RelationId, GraphError>;
    /// Relations of `id` in the given direction as (relation, other end), in relation-id order.
    /// A self-loop followed in `Both` appears once per direction it matches.
    fn neighbours(
        &self,
        id: EntityId,
        dir: Direction,
    ) -> Result<Vec<(RelationId, EntityId)>, GraphError>;
    /// Number of entities.
    fn entity_count(&self) -> usize;
    /// Number of relations.
    fn relation_count(&self) -> usize;
}

/// The in-memory reference store.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Graph {
    next_entity: u64,
    next_relation: u64,
    entities: BTreeMap<EntityId, String>,
    relations: BTreeMap<RelationId, Relation>,
    out: BTreeMap<EntityId, BTreeSet<RelationId>>,
    inc: BTreeMap<EntityId, BTreeSet<RelationId>>,
}

impl Graph {
    /// An empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// The label of an entity.
    pub fn label(&self, id: EntityId) -> Result<&str, GraphError> {
        self.entities
            .get(&id)
            .map(String::as_str)
            .ok_or(GraphError::NoSuchEntity(id))
    }

    /// A relation by id.
    pub fn relation(&self, id: RelationId) -> Result<&Relation, GraphError> {
        self.relations
            .get(&id)
            .ok_or(GraphError::NoSuchRelation(id))
    }

    /// All entities in id order.
    pub fn entities(&self) -> impl Iterator<Item = (EntityId, &str)> {
        self.entities.iter().map(|(id, l)| (*id, l.as_str()))
    }

    /// All relations in id order.
    pub fn relations(&self) -> impl Iterator<Item = &Relation> {
        self.relations.values()
    }

    /// All relations of one type, in id order.
    pub fn relations_of_type<'a>(
        &'a self,
        rtype: &'a RelationType,
    ) -> impl Iterator<Item = &'a Relation> {
        self.relations.values().filter(move |r| &r.rtype == rtype)
    }

    /// Remove one relation and return it. Its id is not reused.
    pub fn remove_relation(&mut self, id: RelationId) -> Result<Relation, GraphError> {
        let rel = self
            .relations
            .remove(&id)
            .ok_or(GraphError::NoSuchRelation(id))?;
        if let Some(s) = self.out.get_mut(&rel.from) {
            s.remove(&id);
        }
        if let Some(s) = self.inc.get_mut(&rel.to) {
            s.remove(&id);
        }
        Ok(rel)
    }

    /// Remove an entity and every relation touching it; return the removed relations in id order.
    /// The entity's id is not reused.
    pub fn remove_entity(&mut self, id: EntityId) -> Result<Vec<Relation>, GraphError> {
        if !self.entities.contains_key(&id) {
            return Err(GraphError::NoSuchEntity(id));
        }
        let mut touching: BTreeSet<RelationId> = BTreeSet::new();
        touching.extend(self.out.get(&id).into_iter().flatten());
        touching.extend(self.inc.get(&id).into_iter().flatten());
        let mut removed = Vec::new();
        for rid in touching {
            removed.push(self.remove_relation(rid)?);
        }
        self.entities.remove(&id);
        self.out.remove(&id);
        self.inc.remove(&id);
        Ok(removed)
    }

    /// Serialise to the version-1 text form (see the crate docs).
    pub fn to_text(&self) -> String {
        let mut s = String::from("ontologicant-graph 1\n");
        s.push_str(&format!(
            "N\t{}\t{}\n",
            self.next_entity, self.next_relation
        ));
        for (id, label) in &self.entities {
            s.push_str(&format!("E\t{}\t{}\n", id.0, esc(label)));
        }
        for r in self.relations.values() {
            let o = match r.assertion.origin {
                Origin::Asserted => "A",
                Origin::Derived => "D",
            };
            s.push_str(&format!(
                "R\t{}\t{}\t{}\t{}\t{}\t{}\n",
                r.id.0,
                r.from.0,
                r.to.0,
                o,
                esc(r.rtype.as_str()),
                esc(&r.assertion.source)
            ));
        }
        s
    }

    /// Parse the version-1 text form.
    pub fn from_text(text: &str) -> Result<Graph, GraphError> {
        let err = |line: usize, m: &str| GraphError::Parse {
            line,
            message: m.to_owned(),
        };
        let mut lines = text.lines().enumerate().map(|(i, l)| (i + 1, l));
        match lines.next() {
            Some((_, "ontologicant-graph 1")) => {}
            _ => return Err(err(1, "missing header `ontologicant-graph 1`")),
        }
        let mut g = Graph::new();
        let mut have_n = false;
        for (n, line) in lines {
            let f: Vec<&str> = line.split('\t').collect();
            let num = |s: &str| s.parse::<u64>().map_err(|_| err(n, "not a number"));
            match (f[0], f.len()) {
                ("N", 3) => {
                    g.next_entity = num(f[1])?;
                    g.next_relation = num(f[2])?;
                    have_n = true;
                }
                ("E", 3) => {
                    let id = EntityId(num(f[1])?);
                    if g.entities
                        .insert(id, unesc(f[2]).ok_or_else(|| err(n, "bad escape"))?)
                        .is_some()
                    {
                        return Err(err(n, "duplicate entity id"));
                    }
                }
                ("R", 7) => {
                    let (id, from, to) = (
                        RelationId(num(f[1])?),
                        EntityId(num(f[2])?),
                        EntityId(num(f[3])?),
                    );
                    let origin = match f[4] {
                        "A" => Origin::Asserted,
                        "D" => Origin::Derived,
                        _ => return Err(err(n, "origin must be A or D")),
                    };
                    for e in [from, to] {
                        if !g.entities.contains_key(&e) {
                            return Err(err(
                                n,
                                "relation names an entity that is not declared before it",
                            ));
                        }
                    }
                    let rtype = RelationType::new(unesc(f[5]).ok_or_else(|| err(n, "bad escape"))?);
                    let source = unesc(f[6]).ok_or_else(|| err(n, "bad escape"))?;
                    let assertion = AssertionSource { source, origin };
                    let rel = Relation {
                        id,
                        from,
                        rtype,
                        to,
                        assertion,
                    };
                    if g.relations.insert(id, rel).is_some() {
                        return Err(err(n, "duplicate relation id"));
                    }
                    g.out.entry(from).or_default().insert(id);
                    g.inc.entry(to).or_default().insert(id);
                }
                _ => return Err(err(n, "unknown or malformed record")),
            }
        }
        if !have_n {
            return Err(err(2, "missing `N` record"));
        }
        // Ids must stay below the counters, or a later add could reuse one.
        if g.entities.keys().any(|e| e.0 >= g.next_entity)
            || g.relations.keys().any(|r| r.0 >= g.next_relation)
        {
            return Err(err(2, "an id is not below its `N` counter"));
        }
        Ok(g)
    }
}

/// A [`Graph`] exported to petgraph (feature `petgraph`): nodes weigh `(EntityId, label)`, edges weigh the [`Relation`].
/// Our ids stay on the weights, so any petgraph algorithm result maps back by id.
#[cfg(feature = "petgraph")]
#[derive(Clone, Debug)]
pub struct PetGraph {
    /// The petgraph graph. A `StableGraph`, so its indices survive removal as ours do.
    pub graph: petgraph::stable_graph::StableDiGraph<(EntityId, String), Relation>,
    nodes: BTreeMap<EntityId, petgraph::stable_graph::NodeIndex>,
}

#[cfg(feature = "petgraph")]
impl PetGraph {
    /// The node that stands for `id`, if the entity was exported.
    pub fn node_of(&self, id: EntityId) -> Option<petgraph::stable_graph::NodeIndex> {
        self.nodes.get(&id).copied()
    }
}

#[cfg(feature = "petgraph")]
impl Graph {
    /// Export to petgraph. Nodes are added in entity-id order and edges in relation-id order, so the result is
    /// deterministic; parallel relations stay separate edges.
    pub fn to_petgraph(&self) -> PetGraph {
        let mut graph = petgraph::stable_graph::StableDiGraph::new();
        let nodes: BTreeMap<EntityId, _> = self
            .entities
            .iter()
            .map(|(id, label)| (*id, graph.add_node((*id, label.clone()))))
            .collect();
        for r in self.relations.values() {
            graph.add_edge(nodes[&r.from], nodes[&r.to], r.clone());
        }
        PetGraph { graph, nodes }
    }

    /// Import from petgraph. New entity ids are assigned in node-index order and new relation ids in edge-index order
    /// (holes left by removed petgraph nodes and edges are skipped); the ids of a previous export are not recovered.
    pub fn from_petgraph(
        p: &petgraph::stable_graph::StableDiGraph<String, (RelationType, AssertionSource)>,
    ) -> Graph {
        use petgraph::visit::{EdgeRef, IntoEdgeReferences};
        let mut g = Graph::new();
        let mut ids = BTreeMap::new();
        for n in p.node_indices() {
            ids.insert(n, g.add_entity(&p[n]));
        }
        for e in p.edge_references() {
            let (rtype, assertion) = e.weight().clone();
            g.add_relation(ids[&e.source()], rtype, ids[&e.target()], assertion)
                .expect("both ends were just added");
        }
        g
    }
}

#[cfg(feature = "json")]
#[derive(serde::Serialize, serde::Deserialize)]
struct JsonEntity {
    id: EntityId,
    label: String,
}

#[cfg(feature = "json")]
#[derive(serde::Serialize, serde::Deserialize)]
struct JsonGraph {
    version: u32,
    next_entity: u64,
    next_relation: u64,
    entities: Vec<JsonEntity>,
    relations: Vec<Relation>,
}

#[cfg(feature = "json")]
impl Graph {
    /// Serialise to JSON (see the crate docs). Entities and relations are in id order.
    pub fn to_json(&self) -> String {
        let doc = JsonGraph {
            version: 1,
            next_entity: self.next_entity,
            next_relation: self.next_relation,
            entities: self
                .entities
                .iter()
                .map(|(id, label)| JsonEntity {
                    id: *id,
                    label: label.clone(),
                })
                .collect(),
            relations: self.relations.values().cloned().collect(),
        };
        serde_json::to_string(&doc).expect("a graph always serialises")
    }

    /// Parse the JSON form, enforcing the same invariants as [`Graph::from_text`].
    pub fn from_json(json: &str) -> Result<Graph, GraphError> {
        let bad = |m: &str| GraphError::Json(m.to_owned());
        let doc: JsonGraph =
            serde_json::from_str(json).map_err(|e| GraphError::Json(e.to_string()))?;
        if doc.version != 1 {
            return Err(bad("unsupported version"));
        }
        let mut g = Graph::new();
        g.next_entity = doc.next_entity;
        g.next_relation = doc.next_relation;
        for e in doc.entities {
            if g.entities.insert(e.id, e.label).is_some() {
                return Err(bad("duplicate entity id"));
            }
        }
        for r in doc.relations {
            if !g.entities.contains_key(&r.from) || !g.entities.contains_key(&r.to) {
                return Err(bad("relation names an entity that is not declared"));
            }
            g.out.entry(r.from).or_default().insert(r.id);
            g.inc.entry(r.to).or_default().insert(r.id);
            if g.relations.insert(r.id, r).is_some() {
                return Err(bad("duplicate relation id"));
            }
        }
        if g.entities.keys().any(|e| e.0 >= g.next_entity)
            || g.relations.keys().any(|r| r.0 >= g.next_relation)
        {
            return Err(bad("an id is not below its counter"));
        }
        Ok(g)
    }
}

impl GraphStore for Graph {
    fn add_entity(&mut self, label: &str) -> EntityId {
        let id = EntityId(self.next_entity);
        self.next_entity += 1;
        self.entities.insert(id, label.to_owned());
        id
    }

    fn add_relation(
        &mut self,
        from: EntityId,
        rtype: RelationType,
        to: EntityId,
        assertion: AssertionSource,
    ) -> Result<RelationId, GraphError> {
        for e in [from, to] {
            if !self.entities.contains_key(&e) {
                return Err(GraphError::NoSuchEntity(e));
            }
        }
        let id = RelationId(self.next_relation);
        self.next_relation += 1;
        self.relations.insert(
            id,
            Relation {
                id,
                from,
                rtype,
                to,
                assertion,
            },
        );
        self.out.entry(from).or_default().insert(id);
        self.inc.entry(to).or_default().insert(id);
        Ok(id)
    }

    fn neighbours(
        &self,
        id: EntityId,
        dir: Direction,
    ) -> Result<Vec<(RelationId, EntityId)>, GraphError> {
        if !self.entities.contains_key(&id) {
            return Err(GraphError::NoSuchEntity(id));
        }
        let mut v: Vec<(RelationId, EntityId)> = Vec::new();
        if matches!(dir, Direction::Out | Direction::Both) {
            v.extend(
                self.out
                    .get(&id)
                    .into_iter()
                    .flatten()
                    .map(|r| (*r, self.relations[r].to)),
            );
        }
        if matches!(dir, Direction::In | Direction::Both) {
            v.extend(
                self.inc
                    .get(&id)
                    .into_iter()
                    .flatten()
                    .map(|r| (*r, self.relations[r].from)),
            );
        }
        v.sort();
        Ok(v)
    }

    fn entity_count(&self) -> usize {
        self.entities.len()
    }

    fn relation_count(&self) -> usize {
        self.relations.len()
    }
}

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '\t' => o.push_str("\\t"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            c => o.push(c),
        }
    }
    o
}

fn unesc(s: &str) -> Option<String> {
    let mut o = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            o.push(match it.next()? {
                '\\' => '\\',
                't' => '\t',
                'n' => '\n',
                'r' => '\r',
                _ => return None,
            });
        } else {
            o.push(c);
        }
    }
    Some(o)
}
