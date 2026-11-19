// SPDX-License-Identifier: MIT OR Apache-2.0
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ontologicant_core::{Reason, Verdict};
use ontologicant_graph::{EntityId, Graph, RelationId, RelationType};

/// A property a relation type can be declared to have.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Property {
    /// a R b and b R c imply a R c. Used by [`Closure`].
    Transitive,
    /// a R b implies b R a. Used by [`Closure`].
    Symmetric,
    /// a R a for every entity. Used by [`Closure`].
    Reflexive,
    /// Never a R a. Declared here, checked by the constraints crate.
    Irreflexive,
    /// Each a has at most one b with a R b. Declared here, checked by the constraints crate.
    Functional,
    /// Each b has at most one a with a R b. Declared here, checked by the constraints crate.
    InverseFunctional,
}

/// What is declared about one relation type.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelationDecl {
    /// Its properties.
    pub properties: BTreeSet<Property>,
    /// Its inverse relation type, if any.
    pub inverse: Option<RelationType>,
}

/// Declared relation properties, by relation type.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Vocabulary {
    decls: BTreeMap<RelationType, RelationDecl>,
}

impl Vocabulary {
    /// An empty vocabulary: every relation type is a plain labelled edge.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare a property of `rtype`.
    pub fn declare(&mut self, rtype: RelationType, p: Property) -> &mut Self {
        self.decls.entry(rtype).or_default().properties.insert(p);
        self
    }

    /// Declare `a` and `b` inverse to each other (both directions are recorded).
    pub fn declare_inverse(&mut self, a: RelationType, b: RelationType) -> &mut Self {
        self.decls.entry(a.clone()).or_default().inverse = Some(b.clone());
        self.decls.entry(b).or_default().inverse = Some(a);
        self
    }

    /// What is declared about `rtype`, if anything.
    pub fn decl(&self, rtype: &RelationType) -> Option<&RelationDecl> {
        self.decls.get(rtype)
    }

    /// Every pair that relates by `rtype` under the declared properties, each with the shortest chain of steps that derives it.
    pub fn closure(&self, graph: &Graph, rtype: &RelationType) -> Closure {
        let decl = self.decls.get(rtype).cloned().unwrap_or_default();
        let has = |p: Property| decl.properties.contains(&p);
        let mut adj: BTreeMap<EntityId, BTreeMap<EntityId, Edge>> = BTreeMap::new();
        let mut add = |e: Edge| {
            let slot = adj
                .entry(e.from)
                .or_default()
                .entry(e.to)
                .or_insert_with(|| e.clone());
            if e.step < slot.step {
                *slot = e;
            }
        };
        for r in graph.relations_of_type(rtype) {
            add(Edge {
                from: r.from,
                to: r.to,
                step: Step::Direct(r.id),
            });
            if has(Property::Symmetric) {
                add(Edge {
                    from: r.to,
                    to: r.from,
                    step: Step::Symmetric(r.id),
                });
            }
        }
        if let Some(inv) = &decl.inverse {
            for r in graph.relations_of_type(inv) {
                add(Edge {
                    from: r.to,
                    to: r.from,
                    step: Step::Inverse(r.id),
                });
                if has(Property::Symmetric) {
                    add(Edge {
                        from: r.from,
                        to: r.to,
                        step: Step::Inverse(r.id),
                    });
                }
            }
        }
        let mut pairs: BTreeMap<(EntityId, EntityId), Vec<Edge>> = BTreeMap::new();
        let sources: Vec<EntityId> = adj.keys().copied().collect();
        for s in sources {
            if has(Property::Transitive) {
                let mut prev: BTreeMap<EntityId, Edge> = BTreeMap::new();
                let mut q = VecDeque::from([s]);
                while let Some(x) = q.pop_front() {
                    for (&n, e) in adj.get(&x).into_iter().flatten() {
                        if let std::collections::btree_map::Entry::Vacant(slot) = prev.entry(n) {
                            slot.insert(e.clone());
                            q.push_back(n);
                        }
                    }
                }
                for &t in prev.keys() {
                    let mut chain = Vec::new();
                    let mut cur = t;
                    loop {
                        let e = prev[&cur].clone();
                        cur = e.from;
                        chain.push(e);
                        if cur == s {
                            break;
                        }
                    }
                    chain.reverse();
                    pairs.insert((s, t), chain);
                }
            } else {
                for (&t, e) in &adj[&s] {
                    pairs.insert((s, t), vec![e.clone()]);
                }
            }
        }
        if has(Property::Reflexive) {
            for (e, _) in graph.entities() {
                pairs.entry((e, e)).or_insert_with(|| {
                    vec![Edge {
                        from: e,
                        to: e,
                        step: Step::Reflexive,
                    }]
                });
            }
        }
        Closure { pairs }
    }
}

/// Why one link of a derivation holds. Variants are in priority order: when two ways give the same pair, the lower wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Step {
    /// Asserted by this relation.
    Direct(RelationId),
    /// Asserted by this relation of the declared inverse type, read backwards.
    Inverse(RelationId),
    /// This relation read backwards because the type is symmetric.
    Symmetric(RelationId),
    /// Every entity relates to itself because the type is reflexive.
    Reflexive,
}

/// One link in a derivation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    /// Where the link starts.
    pub from: EntityId,
    /// Where it ends.
    pub to: EntityId,
    /// Why it holds.
    pub step: Step,
}

/// The pairs a relation type implies, with derivations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Closure {
    pairs: BTreeMap<(EntityId, EntityId), Vec<Edge>>,
}

impl Closure {
    /// All pairs in id order.
    pub fn pairs(&self) -> impl Iterator<Item = (EntityId, EntityId)> + '_ {
        self.pairs.keys().copied()
    }

    /// The derivation of `(a, b)`: the chain of links, or `None` if the pair does not hold.
    pub fn derivation(&self, a: EntityId, b: EntityId) -> Option<&[Edge]> {
        self.pairs.get(&(a, b)).map(Vec::as_slice)
    }

    /// Does `a` relate to `b`, and why or why not.
    pub fn explain(&self, a: EntityId, b: EntityId) -> Verdict<bool> {
        match self.derivation(a, b) {
            Some(chain) => chain.iter().fold(Verdict::new(true), |v, e| {
                v.because(Reason::new(
                    "kinds.closure.step",
                    format!("{} -> {}: {:?}", e.from, e.to, e.step),
                ))
            }),
            None => Verdict::new(false).because(Reason::new(
                "kinds.closure.no-derivation",
                format!("nothing derives {a} -> {b}"),
            )),
        }
    }
}
