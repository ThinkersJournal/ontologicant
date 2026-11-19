// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(missing_docs)]
#![cfg(feature = "json")]
use ontologicant_graph::*;

fn sample() -> Graph {
    let mut g = Graph::new();
    let a = g.add_entity("tab\tby");
    let b = g.add_entity("cat");
    let c = g.add_entity("gone");
    g.add_relation(
        a,
        RelationType::new("is-a"),
        b,
        AssertionSource::asserted("s1"),
    )
    .unwrap();
    g.add_relation(b, RelationType::new("x"), c, AssertionSource::derived("t"))
        .unwrap();
    g.remove_entity(c).unwrap(); // leaves id gaps that must survive
    g
}

#[test]
fn json_round_trips_the_whole_graph_including_id_counters() {
    let g = sample();
    let back = Graph::from_json(&g.to_json()).unwrap();
    // (`==` would also compare empty adjacency entries a removal leaves behind, so compare the content)
    assert_eq!(back.to_text(), g.to_text());
    let mut back = back;
    let fresh = back.add_entity("new");
    assert_eq!(
        fresh,
        EntityId(3),
        "removed ids are not reused after a JSON round trip"
    );
}

#[test]
fn json_has_a_documented_shape() {
    let v: serde_json::Value = serde_json::from_str(&sample().to_json()).unwrap();
    assert_eq!(v["version"], 1);
    assert_eq!(v["next_entity"], 3);
    assert_eq!(v["entities"][0]["id"], 0);
    assert_eq!(v["entities"][0]["label"], "tab\tby");
    assert_eq!(v["relations"][0]["rtype"], "is-a");
    assert_eq!(v["relations"][0]["assertion"]["origin"], "Asserted");
}

#[test]
fn json_and_text_forms_agree() {
    let g = sample();
    assert_eq!(
        Graph::from_json(&g.to_json()).unwrap().to_text(),
        g.to_text()
    );
}

#[test]
fn from_json_rejects_what_from_text_rejects() {
    let good = sample().to_json();
    let bad = |edit: &dyn Fn(&mut serde_json::Value)| {
        let mut v: serde_json::Value = serde_json::from_str(&good).unwrap();
        edit(&mut v);
        assert!(Graph::from_json(&v.to_string()).is_err());
    };
    bad(&|v| v["version"] = 2.into());
    bad(&|v| v["next_entity"] = 1.into()); // an id is not below its counter
    bad(&|v| v["relations"][0]["to"] = 99.into()); // dangling endpoint
    bad(&|v| v["entities"][1]["id"] = 0.into()); // duplicate entity id
    assert!(Graph::from_json("not json").is_err());
    let mut two = Graph::new();
    let (a, b) = (two.add_entity("a"), two.add_entity("b"));
    for _ in 0..2 {
        two.add_relation(a, RelationType::new("r"), b, AssertionSource::asserted("s"))
            .unwrap();
    }
    let mut v: serde_json::Value = serde_json::from_str(&two.to_json()).unwrap();
    assert!(Graph::from_json(&v.to_string()).is_ok());
    v["relations"][1]["id"] = 0.into(); // duplicate relation id
    assert!(Graph::from_json(&v.to_string()).is_err());
}

#[test]
fn from_json_rejects_a_duplicate_entity_id_even_with_no_relations() {
    let mut g = Graph::new();
    g.add_entity("a");
    g.add_entity("b");
    let mut v: serde_json::Value = serde_json::from_str(&g.to_json()).unwrap();
    assert!(Graph::from_json(&v.to_string()).is_ok());
    v["entities"][1]["id"] = 0.into();
    assert!(Graph::from_json(&v.to_string()).is_err());
}
