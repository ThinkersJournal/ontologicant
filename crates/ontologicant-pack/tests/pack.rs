// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(missing_docs)]
use ontologicant_constraints::{Constraint, ConstraintSet};
use ontologicant_core::{AssertionSource, Origin, RelationType};
use ontologicant_graph::{Graph, GraphStore};
use ontologicant_identity::IdentityIndex;
use ontologicant_pack::*;
use semanticant_core::Provenance;
use semanticant_core::pack::{FileEntry, Manifest, Sha256Hex};
use semanticant_pack::sha256_hex;
use std::collections::HashMap;

fn pack(kind: &str, name: &str, body: &str) -> Pack {
    let manifest = Manifest {
        name: "demo".into(),
        version: "1.2.3".into(),
        kind: kind.into(),
        schema: 1,
        lang: None,
        provenance: Provenance {
            source: "hand-written".into(),
            licence: "MIT OR Apache-2.0".into(),
            credit: "tests".into(),
            url: None,
            retrieved: None,
        },
        files: vec![FileEntry {
            name: name.into(),
            sha256: Sha256Hex::parse(&sha256_hex(body.as_bytes())).unwrap(),
        }],
    };
    let files = HashMap::from([(name.to_owned(), body.as_bytes().to_vec())]);
    Pack::from_parts(manifest, files).unwrap()
}

fn sample_graph() -> Graph {
    let mut g = Graph::new();
    let cat = g.add_entity("cat");
    let animal = g.add_entity("animal");
    g.add_relation(
        cat,
        RelationType::new("is-a"),
        animal,
        AssertionSource::asserted("test"),
    )
    .unwrap();
    g
}

#[test]
fn constraints_round_trip_with_pack_source() {
    let mut set = ConstraintSet::new();
    set.add(Constraint::Functional(RelationType::new("born-in")));
    set.add(Constraint::Irreflexive(RelationType::new("part-of")));
    let l = constraints(&pack(KIND_CONSTRAINTS, "c.txt", &set.to_text()), "c.txt").unwrap();
    assert_eq!(l.value, set);
    assert_eq!(l.source.source, "pack:demo@1.2.3");
    assert_eq!(l.source.origin, Origin::Asserted);
}

#[test]
fn graph_round_trip() {
    let g = sample_graph();
    let l = graph(&pack(KIND_GRAPH, "g.txt", &g.to_text()), "g.txt").unwrap();
    assert_eq!(l.value.to_text(), g.to_text());
    assert_eq!(l.value.entities().count(), 2);
}

#[test]
fn identity_round_trip() {
    let mut ix = IdentityIndex::new();
    let g = sample_graph();
    let (cat, _) = g.entities().next().unwrap();
    ix.add_alias(cat, "Felis catus", AssertionSource::asserted("pack-x"));
    let l = identity(&pack(KIND_IDENTITY, "i.txt", &ix.to_text()), "i.txt").unwrap();
    assert_eq!(l.value.lookup("Felis catus"), vec![cat]);
}

#[test]
fn wrong_kind_is_refused_before_parsing() {
    let set = ConstraintSet::new();
    let p = pack(KIND_GRAPH, "c.txt", &set.to_text());
    match constraints(&p, "c.txt") {
        Err(LoadError::WrongKind { expected, found }) => {
            assert_eq!((expected, found.as_str()), (KIND_CONSTRAINTS, KIND_GRAPH));
        }
        other => panic!("expected WrongKind, got {other:?}"),
    }
}

#[test]
fn unlisted_file_is_refused() {
    let p = pack(KIND_GRAPH, "g.txt", &sample_graph().to_text());
    assert!(matches!(
        graph(&p, "other.txt"),
        Err(LoadError::NotListed(n)) if n == "other.txt"
    ));
}

#[test]
fn non_utf8_is_refused() {
    let body = [0xff_u8, 0xfe, 0x00];
    let manifest = Manifest {
        files: vec![FileEntry {
            name: "g.txt".into(),
            sha256: Sha256Hex::parse(&sha256_hex(&body)).unwrap(),
        }],
        ..pack(KIND_GRAPH, "x", "x").manifest().clone()
    };
    let files = HashMap::from([("g.txt".to_owned(), body.to_vec())]);
    let p = Pack::from_parts(manifest, files).unwrap();
    assert!(matches!(graph(&p, "g.txt"), Err(LoadError::NotText(_))));
}

#[test]
fn malformed_text_is_a_typed_error_not_a_panic() {
    let p = pack(KIND_CONSTRAINTS, "c.txt", "this is not a constraint\n");
    assert!(matches!(
        constraints(&p, "c.txt"),
        Err(LoadError::Constraints(_))
    ));
    let p = pack(KIND_GRAPH, "g.txt", "garbage line\n");
    assert!(matches!(graph(&p, "g.txt"), Err(LoadError::Graph(_))));
    let p = pack(KIND_IDENTITY, "i.txt", "garbage line\n");
    assert!(matches!(identity(&p, "i.txt"), Err(LoadError::Identity(_))));
}

#[test]
fn tampered_bytes_never_reach_us() {
    let m = pack(KIND_GRAPH, "g.txt", "x").manifest().clone();
    let files = HashMap::from([("g.txt".to_owned(), b"tampered".to_vec())]);
    let e = Pack::from_parts(m, files).unwrap_err();
    assert!(matches!(e, PackError::ChecksumMismatch { .. }));
    assert!(matches!(LoadError::from(e), LoadError::Pack(_)));
}

#[test]
fn open_from_disk_verifies_then_loads() {
    let dir = std::env::temp_dir().join(format!("ontologicant-pack-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let body = sample_graph().to_text();
    std::fs::write(dir.join("g.txt"), &body).unwrap();
    let toml = format!(
        "schema = 1\nname = \"disk\"\nversion = \"0.1.0\"\nkind = \"graph\"\n\n[provenance]\nsource = \"s\"\nlicence = \"MIT\"\ncredit = \"c\"\n\n[[files]]\nname = \"g.txt\"\nsha256 = \"{}\"\n",
        sha256_hex(body.as_bytes())
    );
    std::fs::write(dir.join("pack.toml"), toml).unwrap();
    let l = open_graph(&dir, "g.txt").unwrap();
    assert_eq!(l.source.source, "pack:disk@0.1.0");
    // Tamper after the manifest was written: opening must now fail.
    std::fs::write(dir.join("g.txt"), "tampered\n").unwrap();
    assert!(matches!(
        open_graph(&dir, "g.txt"),
        Err(LoadError::Pack(PackError::ChecksumMismatch { .. }))
    ));
    std::fs::remove_dir_all(&dir).unwrap();
}
