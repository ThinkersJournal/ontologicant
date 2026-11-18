// SPDX-License-Identifier: MIT OR Apache-2.0
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use ontologicant_graph::{EntityId, Graph, RelationType};

/// A cycle in a relation that must be acyclic. `path` starts and ends at the same entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cycle {
    /// The entities around the cycle, first equal to last.
    pub path: Vec<EntityId>,
}

impl fmt::Display for Cycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<String> = self.path.iter().map(|e| e.to_string()).collect();
        write!(f, "cycle: {}", names.join(" -> "))
    }
}
impl std::error::Error for Cycle {}

/// A transitive, acyclic relation read from the edges of one relation type: an edge `a -> b` means "a is below b"
/// (a is-a b, a is part of b). `up` goes towards b, `down` towards a.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Order {
    up: BTreeMap<EntityId, BTreeSet<EntityId>>,
    down: BTreeMap<EntityId, BTreeSet<EntityId>>,
}

impl Order {
    /// Read the relations of `rtype` from `graph`. Fails with the first cycle found, in id order.
    pub fn from_graph(graph: &Graph, rtype: &RelationType) -> Result<Order, Cycle> {
        let mut o = Order::default();
        for r in graph.relations_of_type(rtype) {
            o.up.entry(r.from).or_default().insert(r.to);
            o.down.entry(r.to).or_default().insert(r.from);
            o.up.entry(r.to).or_default();
            o.down.entry(r.from).or_default();
        }
        o.find_cycle().map_or(Ok(o), Err)
    }

    fn find_cycle(&self) -> Option<Cycle> {
        // 0 = unseen, 1 = on the stack, 2 = done
        let mut state: BTreeMap<EntityId, u8> = BTreeMap::new();
        for &start in self.up.keys() {
            if state.get(&start).copied().unwrap_or(0) != 0 {
                continue;
            }
            let mut stack: Vec<(EntityId, Vec<EntityId>)> =
                vec![(start, self.up[&start].iter().rev().copied().collect())];
            let mut path = vec![start];
            state.insert(start, 1);
            while let Some((_, todo)) = stack.last_mut() {
                match todo.pop() {
                    Some(next) => match state.get(&next).copied().unwrap_or(0) {
                        1 => {
                            let i = path.iter().position(|p| *p == next).expect("on the stack");
                            let mut cyc = path[i..].to_vec();
                            cyc.push(next);
                            return Some(Cycle { path: cyc });
                        }
                        0 => {
                            state.insert(next, 1);
                            path.push(next);
                            stack.push((next, self.up[&next].iter().rev().copied().collect()));
                        }
                        _ => {}
                    },
                    None => {
                        let (done, _) = stack.pop().expect("non-empty");
                        state.insert(done, 2);
                        path.pop();
                    }
                }
            }
        }
        None
    }

    /// Entities that take part in the relation, in id order.
    pub fn nodes(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.up.keys().copied()
    }

    /// Direct upper neighbours (parents, wholes).
    pub fn parents(&self, e: EntityId) -> BTreeSet<EntityId> {
        self.up.get(&e).cloned().unwrap_or_default()
    }

    /// Direct lower neighbours (children, parts).
    pub fn children(&self, e: EntityId) -> BTreeSet<EntityId> {
        self.down.get(&e).cloned().unwrap_or_default()
    }

    /// Everything above `e`, not including `e`.
    pub fn ancestors(&self, e: EntityId) -> BTreeSet<EntityId> {
        reach(&self.up, e)
    }

    /// Everything below `e`, not including `e`.
    pub fn descendants(&self, e: EntityId) -> BTreeSet<EntityId> {
        reach(&self.down, e)
    }

    /// Participating entities with nothing above them.
    pub fn roots(&self) -> Vec<EntityId> {
        self.up
            .iter()
            .filter(|(_, u)| u.is_empty())
            .map(|(e, _)| *e)
            .collect()
    }

    /// A shortest chain `from, ..., to` going upwards, ties broken by lowest id; `None` if `to` is not above `from`.
    pub fn path(&self, from: EntityId, to: EntityId) -> Option<Vec<EntityId>> {
        let mut prev: BTreeMap<EntityId, EntityId> = BTreeMap::new();
        let mut q = VecDeque::from([from]);
        while let Some(x) = q.pop_front() {
            for &n in self.up.get(&x).into_iter().flatten() {
                if n != from && !prev.contains_key(&n) {
                    prev.insert(n, x);
                    if n == to {
                        let mut p = vec![to];
                        while let Some(&b) = prev.get(p.last().expect("non-empty")) {
                            p.push(b);
                            if b == from {
                                break;
                            }
                        }
                        p.reverse();
                        return Some(p);
                    }
                    q.push_back(n);
                }
            }
        }
        None
    }
}

fn reach(adj: &BTreeMap<EntityId, BTreeSet<EntityId>>, e: EntityId) -> BTreeSet<EntityId> {
    let mut seen = BTreeSet::new();
    let mut stack: Vec<EntityId> = adj.get(&e).into_iter().flatten().copied().collect();
    while let Some(x) = stack.pop() {
        if seen.insert(x) {
            stack.extend(adj.get(&x).into_iter().flatten().copied());
        }
    }
    seen
}
