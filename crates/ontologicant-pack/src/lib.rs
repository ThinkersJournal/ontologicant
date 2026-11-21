// SPDX-License-Identifier: MIT OR Apache-2.0
//! `ontologicant-pack`: read OntologicAnt data from a SemanticAnt data pack.
//!
//! The pack format and its verification (licence, source and credit required; every listed file's SHA-256 checked;
//! symlinks refused) belong to `semanticant-pack`; this crate only consumes a [`Pack`] that has already passed that
//! and turns one listed file into one of our types. Three pack kinds are understood:
//!
//! | manifest `kind` | file text form | result |
//! |---|---|---|
//! | `constraints` | `ConstraintSet::to_text` | [`ConstraintSet`] |
//! | `graph` | `Graph::to_text` | [`Graph`] |
//! | `identity` | `IdentityIndex::to_text` | [`IdentityIndex`] |
//!
//! Each result comes back in a [`Loaded`] with an [`AssertionSource`] naming the pack (`pack:<name>@<version>`), so
//! callers can stamp what they assert from it. The pack's licence and credit stay on the pack's manifest
//! (`semanticant_core::Provenance`); they are deliberately not folded into `AssertionSource`.

use ontologicant_constraints::{ConstraintSet, ParseError};
use ontologicant_core::AssertionSource;
use ontologicant_graph::{Graph, GraphError};
use ontologicant_identity::{IdentityError, IdentityIndex};
pub use semanticant_pack::{Pack, PackError};
use std::fmt;
use std::path::Path;

/// Manifest `kind` of a constraint-set pack.
pub const KIND_CONSTRAINTS: &str = "constraints";
/// Manifest `kind` of a graph pack.
pub const KIND_GRAPH: &str = "graph";
/// Manifest `kind` of an identity-log pack.
pub const KIND_IDENTITY: &str = "identity";

/// A value read from a pack, with where it came from.
#[derive(Clone, Debug)]
pub struct Loaded<T> {
    /// The value.
    pub value: T,
    /// `pack:<name>@<version>`, as an asserted source.
    pub source: AssertionSource,
}

/// Why loading from a pack failed.
#[derive(Debug)]
pub enum LoadError {
    /// The pack itself could not be opened or verified.
    Pack(PackError),
    /// The manifest `kind` is not the one this loader reads.
    WrongKind {
        /// The kind required.
        expected: &'static str,
        /// The kind the manifest declares.
        found: String,
    },
    /// The named file is not listed in the manifest.
    NotListed(String),
    /// The named file is not valid UTF-8 text.
    NotText(String),
    /// The constraint text did not parse.
    Constraints(ParseError),
    /// The graph text did not parse.
    Graph(GraphError),
    /// The identity text did not replay.
    Identity(IdentityError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pack(e) => write!(f, "pack: {e}"),
            Self::WrongKind { expected, found } => {
                write!(f, "pack kind is `{found}`, expected `{expected}`")
            }
            Self::NotListed(n) => write!(f, "file `{n}` is not listed in the pack manifest"),
            Self::NotText(n) => write!(f, "file `{n}` is not valid UTF-8"),
            Self::Constraints(e) => write!(f, "constraints: {e}"),
            Self::Graph(e) => write!(f, "graph: {e}"),
            Self::Identity(e) => write!(f, "identity: {e}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<PackError> for LoadError {
    fn from(e: PackError) -> Self {
        Self::Pack(e)
    }
}

fn text<'a>(pack: &'a Pack, kind: &'static str, file: &str) -> Result<&'a str, LoadError> {
    let found = &pack.manifest().kind;
    if found != kind {
        return Err(LoadError::WrongKind {
            expected: kind,
            found: found.clone(),
        });
    }
    if pack.file(file).is_none() {
        return Err(LoadError::NotListed(file.to_owned()));
    }
    pack.text(file)
        .ok_or_else(|| LoadError::NotText(file.to_owned()))
}

fn source_of(pack: &Pack) -> AssertionSource {
    let m = pack.manifest();
    AssertionSource::asserted(format!("pack:{}@{}", m.name, m.version))
}

/// Read a [`ConstraintSet`] from `file` of a `constraints` pack.
pub fn constraints(pack: &Pack, file: &str) -> Result<Loaded<ConstraintSet>, LoadError> {
    let t = text(pack, KIND_CONSTRAINTS, file)?;
    Ok(Loaded {
        value: ConstraintSet::from_text(t).map_err(LoadError::Constraints)?,
        source: source_of(pack),
    })
}

/// Read a [`Graph`] from `file` of a `graph` pack.
pub fn graph(pack: &Pack, file: &str) -> Result<Loaded<Graph>, LoadError> {
    let t = text(pack, KIND_GRAPH, file)?;
    Ok(Loaded {
        value: Graph::from_text(t).map_err(LoadError::Graph)?,
        source: source_of(pack),
    })
}

/// Read an [`IdentityIndex`] from `file` of an `identity` pack.
pub fn identity(pack: &Pack, file: &str) -> Result<Loaded<IdentityIndex>, LoadError> {
    let t = text(pack, KIND_IDENTITY, file)?;
    Ok(Loaded {
        value: IdentityIndex::from_text(t).map_err(LoadError::Identity)?,
        source: source_of(pack),
    })
}

/// Open and verify the pack in `dir`, then read a [`ConstraintSet`] from `file`.
pub fn open_constraints(dir: &Path, file: &str) -> Result<Loaded<ConstraintSet>, LoadError> {
    constraints(&Pack::open(dir)?, file)
}

/// Open and verify the pack in `dir`, then read a [`Graph`] from `file`.
pub fn open_graph(dir: &Path, file: &str) -> Result<Loaded<Graph>, LoadError> {
    graph(&Pack::open(dir)?, file)
}

/// Open and verify the pack in `dir`, then read an [`IdentityIndex`] from `file`.
pub fn open_identity(dir: &Path, file: &str) -> Result<Loaded<IdentityIndex>, LoadError> {
    identity(&Pack::open(dir)?, file)
}
