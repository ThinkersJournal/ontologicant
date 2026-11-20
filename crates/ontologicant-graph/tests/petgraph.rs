// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(missing_docs)]
#![cfg(feature = "petgraph")]
use ontologicant_graph::*;
use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::EdgeRef;

fn sample() -> (Graph, [EntityId; 4]) {
    let mut g = Graph::new();
    let a = g.add_entity("a");
    let b = g.add_entity("b");
    let gone = g.add_entity("gone");
    let c = g.add_entity("c");
    g.add_relation(
        a,
        RelationType::new("is-a"),
        b,
        AssertionSource::asserted("s"),
    )
    .unwrap();
    g.add_relation(
        a,
        RelationType::new("is-a"),
        b,
        AssertionSource::derived("t"),
    )
    .unwrap(); // parallel
    g.add_relation(
        b,
        RelationType::new("is-a"),
        c,
        AssertionSource::asserted("s"),
    )
    .unwrap();
    g.remove_entity(gone).unwrap();
    (g, [a, b, gone, c])
}

#[test]
fn export_keeps_entities_relations_and_parallel_edges() {
    let (g, _) = sample();
    let p = g.to_petgraph();
    assert_eq!(p.graph.node_count(), 3, "removed entity is not exported");
    assert_eq!(
        p.graph.edge_count(),
        3,
        "parallel relations stay separate edges"
    );
}

#[test]
fn node_weights_and_edge_weights_carry_ids_labels_and_assertions() {
    let (g, [a, b, _, _]) = sample();
    let p = g.to_petgraph();
    let na = p.node_of(a).unwrap();
    assert_eq!(p.graph[na], (a, "a".to_owned()));
    let e: Vec<_> = p.graph.edges(na).collect();
    assert_eq!(e.len(), 2);
    assert!(e.iter().all(|x| p.graph[x.target()].0 == b));
    let rel = e[0].weight();
    assert_eq!(rel.rtype, RelationType::new("is-a"));
    assert!(
        p.node_of(EntityId(2)).is_none(),
        "the removed id has no node"
    );
}

#[test]
fn petgraph_algorithms_run_on_the_export_and_map_back_to_our_ids() {
    let (g, [a, b, _, c]) = sample();
    let p = g.to_petgraph();
    assert!(!is_cyclic_directed(&p.graph));
    let order: Vec<EntityId> = toposort(&p.graph, None)
        .unwrap()
        .into_iter()
        .map(|n| p.graph[n].0)
        .collect();
    let pos = |x| order.iter().position(|&e| e == x).unwrap();
    assert!(pos(a) < pos(b) && pos(b) < pos(c));
}

#[test]
fn import_builds_a_graph_with_fresh_stable_ids_in_node_order() {
    let mut p: StableDiGraph<String, (RelationType, AssertionSource)> = StableDiGraph::new();
    let x = p.add_node("x".into());
    let y = p.add_node("y".into());
    p.add_edge(
        x,
        y,
        (RelationType::new("r"), AssertionSource::asserted("s")),
    );
    p.add_edge(
        x,
        y,
        (RelationType::new("r"), AssertionSource::asserted("s")),
    );
    let g = Graph::from_petgraph(&p);
    assert_eq!(g.label(EntityId(0)).unwrap(), "x");
    assert_eq!(g.label(EntityId(1)).unwrap(), "y");
    assert_eq!(g.relation_count(), 2);
    assert_eq!(g.neighbours(EntityId(0), Direction::Out).unwrap().len(), 2);
}

#[test]
fn import_skips_holes_left_by_removed_petgraph_nodes() {
    let mut p: StableDiGraph<String, (RelationType, AssertionSource)> = StableDiGraph::new();
    let x = p.add_node("x".into());
    let hole = p.add_node("hole".into());
    let z = p.add_node("z".into());
    p.add_edge(
        x,
        z,
        (RelationType::new("r"), AssertionSource::asserted("s")),
    );
    p.remove_node(hole);
    let g = Graph::from_petgraph(&p);
    assert_eq!(g.entity_count(), 2);
    let labels: Vec<&str> = g.entities().map(|(_, l)| l).collect();
    assert_eq!(labels, vec!["x", "z"]);
    assert_eq!(g.relation_count(), 1);
}

#[test]
fn export_then_import_round_trips_content() {
    let (g, _) = sample();
    let p = g.to_petgraph();
    let weights: StableDiGraph<String, (RelationType, AssertionSource)> = p.graph.map(
        |_, n| n.1.clone(),
        |_, r| (r.rtype.clone(), r.assertion.clone()),
    );
    let back = Graph::from_petgraph(&weights);
    assert_eq!(back.entity_count(), g.entity_count());
    assert_eq!(back.relation_count(), g.relation_count());
    let sig = |g: &Graph| {
        let mut v: Vec<String> = g
            .relations()
            .map(|r| {
                format!(
                    "{}->{}:{}:{:?}",
                    g.label(r.from).unwrap(),
                    g.label(r.to).unwrap(),
                    r.rtype.as_str(),
                    r.assertion
                )
            })
            .collect();
        v.sort();
        v
    };
    assert_eq!(sig(&back), sig(&g));
}
