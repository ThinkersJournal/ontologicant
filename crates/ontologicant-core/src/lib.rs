// SPDX-License-Identifier: MIT OR Apache-2.0
//! `ontologicant-core`: the small types every OntologicAnt tool crate shares.
//!
//! * [`EntityId`] and [`RelationId`]: stable identifiers. An id is never reused for another thing,
//!   even after the thing it named is removed.
//! * [`RelationType`]: the name of a kind of relation. A plain string, never an English enum:
//!   relation vocabularies are data.
//! * [`Provenance`]: where an assertion came from, and whether it was asserted or derived.
//! * [`Reason`] and [`Verdict`]: re-exported from `semanticant-core`, where every project in the family finds them
//!   (PM ruling, 2026-11-18). Note that `semanticant_core::Provenance` is a different thing: the licence and credit of
//!   a data pack. [`Provenance`] here records where one assertion came from.
#![forbid(unsafe_code)]

use std::fmt;
use std::str::FromStr;

pub use semanticant_core::{Reason, Verdict};

macro_rules! id_type {
    ($(#[$m:meta])* $name:ident, $prefix:literal) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub u64);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}{}", $prefix, self.0)
            }
        }

        impl FromStr for $name {
            type Err = ParseIdError;
            fn from_str(s: &str) -> Result<Self, ParseIdError> {
                s.strip_prefix($prefix)
                    .and_then(|n| n.parse::<u64>().ok())
                    .map($name)
                    .ok_or_else(|| ParseIdError(s.to_owned()))
            }
        }
    };
}

id_type!(
    /// Stable identifier of an entity. Displays as `e<n>`.
    EntityId,
    "e"
);
id_type!(
    /// Stable identifier of a relation instance. Displays as `r<n>`.
    RelationId,
    "r"
);

/// An identifier string was not of the form `e<n>` / `r<n>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseIdError(String);

impl fmt::Display for ParseIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not a valid identifier: {:?}", self.0)
    }
}
impl std::error::Error for ParseIdError {}

/// The name of a kind of relation, for example `"is-a"` or `"part-of"`. Any string is allowed.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelationType(String);

impl RelationType {
    /// Make a relation type from any string.
    pub fn new(name: impl Into<String>) -> Self {
        RelationType(name.into())
    }
    /// The name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether an assertion was given or computed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Origin {
    /// Stated by a source.
    Asserted,
    /// Computed from other assertions.
    Derived,
}

/// Where an assertion came from.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Provenance {
    /// A free-form source name (a document, a data pack, a tool).
    pub source: String,
    /// Asserted or derived.
    pub origin: Origin,
}

impl Provenance {
    /// An assertion stated by `source`.
    pub fn asserted(source: impl Into<String>) -> Self {
        Provenance {
            source: source.into(),
            origin: Origin::Asserted,
        }
    }
    /// A conclusion computed by `source`.
    pub fn derived(source: impl Into<String>) -> Self {
        Provenance {
            source: source.into(),
            origin: Origin::Derived,
        }
    }
}
