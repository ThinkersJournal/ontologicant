// SPDX-License-Identifier: MIT OR Apache-2.0
use std::collections::BTreeSet;

use ontologicant_graph::{EntityId, Graph, RelationType};

use crate::order::{Cycle, Order};

/// A mereological axiom a user may ask to have checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Axiom {
    /// A whole with one proper part has another proper part that does not overlap it.
    WeakSupplementation,
    /// Two different wholes never have exactly the same proper parts.
    Extensionality,
}

/// The set of axioms to check. The default checks none: mereology is contested and the user selects.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Axioms(BTreeSet<Axiom>);

impl Axioms {
    /// No axiom.
    pub fn none() -> Self {
        Self::default()
    }
    /// Add an axiom.
    pub fn with(mut self, a: Axiom) -> Self {
        self.0.insert(a);
        self
    }
}

/// An axiom that does not hold, with the entities involved and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    /// The axiom.
    pub axiom: Axiom,
    /// The entities involved, in id order.
    pub entities: Vec<EntityId>,
    /// A human-readable reason.
    pub reason: String,
}

/// Proper parthood read from one relation type (part -> whole). A cycle is always an error: proper parthood is
/// irreflexive and asymmetric. For a reflexive part-of, declare it reflexive in a [`crate::Vocabulary`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mereology {
    order: Order,
}

impl Mereology {
    /// Read the relations of `rtype` (part -> whole) from `graph`.
    pub fn from_graph(graph: &Graph, rtype: &RelationType) -> Result<Mereology, Cycle> {
        Ok(Mereology {
            order: Order::from_graph(graph, rtype)?,
        })
    }

    /// All proper parts of `whole`, direct and indirect.
    pub fn parts(&self, whole: EntityId) -> BTreeSet<EntityId> {
        self.order.descendants(whole)
    }

    /// All wholes that `part` is a proper part of.
    pub fn wholes(&self, part: EntityId) -> BTreeSet<EntityId> {
        self.order.ancestors(part)
    }

    /// Do `a` and `b` overlap: are they equal or do they share a part?
    pub fn overlap(&self, a: EntityId, b: EntityId) -> bool {
        let ext = |e: EntityId| {
            let mut s = self.parts(e);
            s.insert(e);
            s
        };
        a == b || ext(a).intersection(&ext(b)).next().is_some()
    }

    /// Check the selected axioms; violations are in axiom, then id order.
    pub fn violations(&self, axioms: &Axioms) -> Vec<Violation> {
        let mut v = Vec::new();
        for axiom in &axioms.0 {
            match axiom {
                Axiom::WeakSupplementation => self.weak_supplementation(&mut v),
                Axiom::Extensionality => self.extensionality(&mut v),
            }
        }
        v
    }

    fn weak_supplementation(&self, out: &mut Vec<Violation>) {
        for y in self.order.nodes() {
            let parts = self.parts(y);
            for &x in &parts {
                if !parts.iter().any(|&z| !self.overlap(x, z)) {
                    out.push(Violation {
                        axiom: Axiom::WeakSupplementation,
                        entities: vec![x, y],
                        reason: format!("{x} is a proper part of {y}, but {y} has no proper part that does not overlap {x}"),
                    });
                }
            }
        }
    }

    fn extensionality(&self, out: &mut Vec<Violation>) {
        let nodes: Vec<EntityId> = self.order.nodes().collect();
        for (i, &a) in nodes.iter().enumerate() {
            let pa = self.parts(a);
            if pa.is_empty() {
                continue;
            }
            for &b in &nodes[i + 1..] {
                if self.parts(b) == pa {
                    out.push(Violation {
                        axiom: Axiom::Extensionality,
                        entities: vec![a, b],
                        reason: format!(
                            "{a} and {b} are different but have exactly the same proper parts"
                        ),
                    });
                }
            }
        }
    }
}
