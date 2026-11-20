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
//!
//! # Example
//! ```
//! use ontologicant_graph::{AssertionSource, Graph, GraphStore, RelationType};
//! use ontologicant_kinds::Taxonomy;
//!
//! let mut g = Graph::new();
//! let (tabby, cat, animal) = (g.add_entity("tabby"), g.add_entity("cat"), g.add_entity("animal"));
//! let isa = RelationType::new("is-a");
//! let src = AssertionSource::asserted("example");
//! g.add_relation(tabby, isa.clone(), cat, src.clone()).unwrap();
//! g.add_relation(cat, isa.clone(), animal, src).unwrap();
//!
//! let t = Taxonomy::from_graph(&g, &isa).unwrap();
//! let verdict = t.is_a(tabby, animal);
//! assert!(verdict.value); // transitively, and the verdict says why
//! assert!(!verdict.reasons.is_empty());
//! assert!(t.ancestors(tabby).contains(&animal));
//! ```
#![forbid(unsafe_code)]

mod mereology;
mod order;
mod taxonomy;
mod vocab;

pub use mereology::{Axiom, Axioms, Mereology, Violation};
pub use order::{Cycle, Order};
pub use taxonomy::Taxonomy;
pub use vocab::{Closure, Edge, Property, RelationDecl, Step, Vocabulary};
