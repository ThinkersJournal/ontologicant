// SPDX-License-Identifier: MIT OR Apache-2.0
//! `ontologicant-kinds`: kinds and relations over an [`ontologicant_graph::Graph`].
//!
//! * [`Order`]: a transitive, acyclic relation (is-a, part-of, any strict order) read from the graph, with cycle detection
//!   that names the cycle, ancestors, descendants and shortest witness paths.
//! * [`Taxonomy`]: an [`Order`] over an is-a relation, with `is_a` verdicts and lowest common ancestors.
//! * [`Mereology`]: an [`Order`] over a part-of relation (proper parthood) whose axioms are **selected by the user**
//!   ([`Axioms`]); nothing is checked unless asked for, because mereology is contested.
//! * [`Vocabulary`] and [`Closure`]: declared relation properties (transitive, symmetric, reflexive, inverse) and the pairs
//!   they imply, each with the chain of steps that derives it. Functional and inverse-functional are declared here and
//!   enforced by the constraints crate.
//!
//! Relation names are plain strings: nothing here is tied to English or to a particular relation name.
#![forbid(unsafe_code)]

mod mereology;
mod order;
mod taxonomy;
mod vocab;

pub use mereology::{Axiom, Axioms, Mereology, Violation};
pub use order::{Cycle, Order};
pub use taxonomy::Taxonomy;
pub use vocab::{Closure, Edge, Property, RelationDecl, Step, Vocabulary};
