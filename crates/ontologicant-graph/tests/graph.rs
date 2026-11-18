// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(missing_docs)]
use ontologicant_graph::*;

fn p() -> Provenance {
    Provenance::asserted("test")
}
fn t(s: &str) -> RelationType {
    RelationType::new(s)
}

#[test]
fn ids_are_not_reused_after_removal() {
    let mut g = Graph::new();
    let a = g.add_entity("a");
    g.remove_entity(a).unwrap();
    let b = g.add_entity("b");
    assert_ne!(a, b);
    assert_eq!(g.label(a), Err(GraphError::NoSuchEntity(a)));
}

#[test]
fn relation_ids_are_not_reused() {
    let mut g = Graph::new();
    let (a, b) = (g.add_entity("a"), g.add_entity("b"));
    let r1 = g.add_relation(a, t("x"), b, p()).unwrap();
    g.remove_relation(r1).unwrap();
    let r2 = g.add_relation(a, t("x"), b, p()).unwrap();
    assert_ne!(r1, r2);
}

#[test]
fn multigraph_keeps_parallel_edges() {
    let mut g = Graph::new();
    let (a, b) = (g.add_entity("a"), g.add_entity("b"));
    g.add_relation(a, t("x"), b, p()).unwrap();
    g.add_relation(a, t("x"), b, p()).unwrap();
    assert_eq!(g.neighbours(a, Direction::Out).unwrap().len(), 2);
}

#[test]
fn neighbours_are_in_relation_id_order_and_directional() {
    let mut g = Graph::new();
    let (a, b, c) = (g.add_entity("a"), g.add_entity("b"), g.add_entity("c"));
    let r0 = g.add_relation(c, t("x"), a, p()).unwrap();
    let r1 = g.add_relation(a, t("x"), b, p()).unwrap();
    assert_eq!(g.neighbours(a, Direction::Out).unwrap(), vec![(r1, b)]);
    assert_eq!(g.neighbours(a, Direction::In).unwrap(), vec![(r0, c)]);
    assert_eq!(
        g.neighbours(a, Direction::Both).unwrap(),
        vec![(r0, c), (r1, b)]
    );
}

#[test]
fn self_loop_in_both_appears_once_per_direction() {
    let mut g = Graph::new();
    let a = g.add_entity("a");
    let r = g.add_relation(a, t("x"), a, p()).unwrap();
    assert_eq!(
        g.neighbours(a, Direction::Both).unwrap(),
        vec![(r, a), (r, a)]
    );
}

#[test]
fn relation_to_missing_entity_is_refused() {
    let mut g = Graph::new();
    let a = g.add_entity("a");
    let ghost = EntityId(99);
    assert_eq!(
        g.add_relation(a, t("x"), ghost, p()),
        Err(GraphError::NoSuchEntity(ghost))
    );
    assert_eq!(g.relation_count(), 0);
}

#[test]
fn removing_an_entity_removes_its_relations_and_returns_them() {
    let mut g = Graph::new();
    let (a, b, c) = (g.add_entity("a"), g.add_entity("b"), g.add_entity("c"));
    let r0 = g.add_relation(a, t("x"), b, p()).unwrap();
    let r1 = g.add_relation(c, t("x"), a, p()).unwrap();
    let r2 = g.add_relation(b, t("x"), c, p()).unwrap();
    let removed: Vec<RelationId> = g.remove_entity(a).unwrap().iter().map(|r| r.id).collect();
    assert_eq!(removed, vec![r0, r1]);
    assert_eq!(g.relation_count(), 1);
    assert!(g.relation(r2).is_ok());
    assert_eq!(g.neighbours(b, Direction::In).unwrap(), vec![]);
}

#[test]
fn relations_of_type_filters() {
    let mut g = Graph::new();
    let (a, b) = (g.add_entity("a"), g.add_entity("b"));
    g.add_relation(a, t("is-a"), b, p()).unwrap();
    g.add_relation(a, t("part-of"), b, p()).unwrap();
    assert_eq!(g.relations_of_type(&t("part-of")).count(), 1);
}

#[test]
fn text_round_trip_is_exact_with_awkward_strings() {
    let mut g = Graph::new();
    let a = g.add_entity("tab\there\nnewline\\slash\r é 猫");
    let b = g.add_entity("");
    g.add_relation(a, t("is\ta"), b, Provenance::derived("src\\1\n"))
        .unwrap();
    let gone = g.add_entity("gone");
    g.remove_entity(gone).unwrap();
    let text = g.to_text();
    let h = Graph::from_text(&text).unwrap();
    assert_eq!(g, h);
    assert_eq!(h.to_text(), text);
}

#[test]
fn ids_stay_unused_across_a_round_trip() {
    let mut g = Graph::new();
    let a = g.add_entity("a");
    g.remove_entity(a).unwrap();
    let mut h = Graph::from_text(&g.to_text()).unwrap();
    assert_ne!(h.add_entity("b"), a);
}

#[test]
fn output_is_deterministic() {
    let build = || {
        let mut g = Graph::new();
        let (a, b) = (g.add_entity("a"), g.add_entity("b"));
        g.add_relation(b, t("x"), a, p()).unwrap();
        g.add_relation(a, t("y"), b, p()).unwrap();
        g.to_text()
    };
    assert_eq!(build(), build());
}

#[test]
fn malformed_text_is_refused_with_a_line() {
    for (text, line) in [
        ("", 1),
        ("wrong\n", 1),
        ("ontologicant-graph 1\nE\t0\ta\n", 2), // missing N record at end -> reported at 2
        ("ontologicant-graph 1\nN\t1\t0\nE\t0\ta\\q\n", 3), // bad escape
        ("ontologicant-graph 1\nN\t1\t0\nE\t0\ta\nE\t0\tb\n", 4), // duplicate id
        (
            "ontologicant-graph 1\nN\t1\t1\nE\t0\ta\nR\t0\t0\t5\tA\tx\ts\n",
            4,
        ), // undeclared entity
        ("ontologicant-graph 1\nN\t0\t0\nE\t0\ta\n", 2), // id not below counter
        ("ontologicant-graph 1\nN\t1\t0\nZ\n", 3),
    ] {
        match Graph::from_text(text) {
            Err(GraphError::Parse { line: l, .. }) => assert_eq!(l, line, "{text:?}"),
            other => panic!("{text:?} gave {other:?}"),
        }
    }
}

#[test]
fn ids_display_and_parse() {
    assert_eq!(EntityId(12).to_string(), "e12");
    assert_eq!("r3".parse::<RelationId>().unwrap(), RelationId(3));
    assert!("e3".parse::<RelationId>().is_err());
    assert!("e".parse::<EntityId>().is_err());
}
