// SPDX-License-Identifier: MIT OR Apache-2.0
use std::collections::BTreeSet;

use ontologicant_core::{Reason, Verdict};
use ontologicant_graph::{EntityId, Graph, RelationType};

use crate::order::{Cycle, Order};

/// An is-a hierarchy. `is_a(a, b)` is strict: an entity is not its own kind unless a cycle says so, and cycles are refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Taxonomy {
    order: Order,
}

impl Taxonomy {
    /// Read the relations of `rtype` (child -> parent) from `graph`; a cycle is an error naming the cycle.
    pub fn from_graph(graph: &Graph, rtype: &RelationType) -> Result<Taxonomy, Cycle> {
        Ok(Taxonomy {
            order: Order::from_graph(graph, rtype)?,
        })
    }

    /// The underlying order.
    pub fn order(&self) -> &Order {
        &self.order
    }

    /// All kinds above `e`.
    pub fn ancestors(&self, e: EntityId) -> BTreeSet<EntityId> {
        self.order.ancestors(e)
    }

    /// All kinds below `e`.
    pub fn descendants(&self, e: EntityId) -> BTreeSet<EntityId> {
        self.order.descendants(e)
    }

    /// Does `a` fall under `b`? The reasons give the shortest chain, or say there is none.
    pub fn is_a(&self, a: EntityId, b: EntityId) -> Verdict<bool> {
        match self.order.path(a, b) {
            Some(p) => {
                let chain: Vec<String> = p.iter().map(|e| e.to_string()).collect();
                Verdict::new(true).because(Reason::new(
                    "kinds.taxonomy.is-a-chain",
                    format!("{a} is below {b} via {}", chain.join(" -> ")),
                ))
            }
            None => Verdict::new(false).because(Reason::new(
                "kinds.taxonomy.no-chain",
                format!("no chain of is-a edges leads from {a} up to {b}"),
            )),
        }
    }

    /// The lowest common ancestors of `a` and `b`, counting each as its own ancestor: the common ones with no
    /// common one below them. More than one is possible in a multiple-inheritance hierarchy.
    pub fn lowest_common_ancestors(&self, a: EntityId, b: EntityId) -> BTreeSet<EntityId> {
        let with_self = |e: EntityId| {
            let mut s = self.order.ancestors(e);
            s.insert(e);
            s
        };
        let common: BTreeSet<EntityId> =
            with_self(a).intersection(&with_self(b)).copied().collect();
        common
            .iter()
            .copied()
            .filter(|c| {
                !common
                    .iter()
                    .any(|d| d != c && self.order.ancestors(*d).contains(c))
            })
            .collect()
    }
}
