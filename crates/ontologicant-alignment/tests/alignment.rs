// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(missing_docs)]
use ontologicant_alignment::*;
use ontologicant_core::{AssertionSource, EntityId};
use ontologicant_graph::{Graph, GraphStore};
use ontologicant_identity::{ExactMatcher, IdentityIndex, NormalisedMatcher};

fn gn(s: &str) -> GraphName {
    GraphName::new(s).unwrap()
}
fn graph(labels: &[&str]) -> (Graph, Vec<EntityId>) {
    let mut g = Graph::new();
    let ids = labels.iter().map(|l| g.add_entity(l)).collect();
    (g, ids)
}
fn side<'a>(n: &'a GraphName, g: &'a Graph, i: Option<&'a IdentityIndex>) -> Side<'a> {
    Side {
        name: n,
        graph: g,
        identity: i,
    }
}
fn r(n: &str, e: EntityId) -> Ref {
    Ref {
        graph: gn(n),
        entity: e,
    }
}
fn src() -> AssertionSource {
    AssertionSource::asserted("test")
}

#[test]
fn graph_names_are_validated() {
    assert!(GraphName::new("").is_err());
    assert!(GraphName::new("a\tb").is_err());
    assert!(GraphName::new("a\nb").is_err());
    assert_eq!(gn("sage/1").as_str(), "sage/1");
}

#[test]
fn proposes_cross_graph_pairs_only_best_first() {
    let (a, ae) = graph(&["Cat", "dog"]);
    let (b, be) = graph(&["cat", "Cat", "fish"]);
    let (na, nb) = (gn("a"), gn("b"));
    let p = propose(side(&na, &a, None), side(&nb, &b, None), &NormalisedMatcher).unwrap();
    // "Cat"~"Cat" is exact (1.0); "Cat"~"cat" only normalised (0.9). dog and fish match nothing.
    assert_eq!(p.len(), 2);
    assert_eq!((p[0].left.entity, p[0].right.entity), (ae[0], be[1]));
    assert_eq!(p[0].score, 1.0);
    assert_eq!((p[1].left.entity, p[1].right.entity), (ae[0], be[0]));
    assert!(p[1].score < 1.0);
    assert!(p.iter().all(|c| c.relation == Relation::Equivalent));
    assert!(p[0].reasons[0].note.contains("normalised"));
}

#[test]
fn aliases_count_as_names() {
    let (a, ae) = graph(&["Felis catus"]);
    let (b, be) = graph(&["cat"]);
    let mut ix = IdentityIndex::new();
    ix.add_alias(ae[0], "cat", src());
    let (na, nb) = (gn("a"), gn("b"));
    assert!(
        propose(side(&na, &a, None), side(&nb, &b, None), &ExactMatcher)
            .unwrap()
            .is_empty()
    );
    let p = propose(side(&na, &a, Some(&ix)), side(&nb, &b, None), &ExactMatcher).unwrap();
    assert_eq!((p[0].left.entity, p[0].right.entity), (ae[0], be[0]));
}

#[test]
fn same_graph_is_refused() {
    let (a, _) = graph(&["x"]);
    let n = gn("a");
    assert_eq!(
        propose(side(&n, &a, None), side(&n, &a, None), &ExactMatcher),
        Err(AlignError::SameGraph(n.clone()))
    );
}

fn corr(l: Ref, rr: Ref) -> Correspondence {
    Correspondence {
        left: l,
        right: rr,
        relation: Relation::Equivalent,
        score: 0.9,
        reasons: vec![ontologicant_core::Reason::new("alignment.match", "test")],
    }
}

#[test]
fn accept_is_transitive_and_symmetric_and_retractable() {
    let (a, b, c) = (EntityId(1), EntityId(2), EntityId(3));
    let mut al = Alignment::new();
    let c1 = al.accept(&corr(r("a", a), r("b", b)), src()).unwrap();
    al.accept(&corr(r("c", c), r("b", b)), src()).unwrap();
    assert!(
        al.equivalent(&r("a", a), &r("c", c)),
        "transitive through b"
    );
    assert!(al.equivalent(&r("c", c), &r("a", a)), "symmetric");
    assert_eq!(al.equivalents(&r("b", b)), vec![r("a", a), r("c", c)]);
    al.retract(c1).unwrap();
    assert!(!al.equivalent(&r("a", a), &r("c", c)));
    assert!(al.equivalent(&r("b", b), &r("c", c)));
    assert_eq!(al.log().len(), 2, "log is append-only: retracted stays");
    assert_eq!(al.retract(c1), Err(AlignError::AlreadyRetracted(c1)));
    assert!(al.equivalent(&r("a", a), &r("a", a)));
}

#[test]
fn duplicates_and_bad_input_are_refused() {
    let (a, b) = (EntityId(1), EntityId(2));
    let mut al = Alignment::new();
    let id = al.accept(&corr(r("a", a), r("b", b)), src()).unwrap();
    assert_eq!(
        al.accept(&corr(r("b", b), r("a", a)), src()),
        Err(AlignError::AlreadyAccepted(id)),
        "reversed pair is the same pair"
    );
    assert!(matches!(
        al.accept(&corr(r("a", a), r("a", b)), src()),
        Err(AlignError::SameGraph(_))
    ));
    let mut bad = corr(r("a", a), r("c", b));
    bad.score = f64::NAN;
    assert_eq!(al.accept(&bad, src()), Err(AlignError::BadScore));
    bad.score = 1.5;
    assert_eq!(al.accept(&bad, src()), Err(AlignError::BadScore));
    assert_eq!(
        al.retract(CorrespondenceId(99)),
        Err(AlignError::Unknown(CorrespondenceId(99)))
    );
    // Once retracted, the pair may be accepted again.
    al.retract(id).unwrap();
    assert!(al.accept(&corr(r("a", a), r("b", b)), src()).is_ok());
}

#[test]
fn text_form_round_trips_including_retracted_and_reaccepted() {
    let (a, b, c) = (EntityId(1), EntityId(2), EntityId(3));
    let mut al = Alignment::new();
    let id = al.accept(&corr(r("a\\b", a), r("b", b)), src()).unwrap();
    al.retract(id).unwrap();
    al.accept(&corr(r("a\\b", a), r("b", b)), src()).unwrap();
    al.accept(
        &corr(r("c", c), r("b", b)),
        AssertionSource::derived("tab\there"),
    )
    .unwrap();
    let text = al.to_text();
    let back = Alignment::from_text(&text).unwrap();
    assert_eq!(back.log(), al.log());
    assert_eq!(back.to_text(), text);
    assert!(back.equivalent(&r("a\\b", a), &r("c", c)));
}

#[test]
fn malformed_text_is_a_typed_error() {
    for bad in [
        "garbage\n",
        "accept\t5\ta\t1\tb\t2\t0.9\tasserted\ts\twhy\tlive\n",
        "accept\t0\ta\t1\ta\t2\t0.9\tasserted\ts\twhy\tlive\n",
        "accept\t0\ta\t1\tb\t2\tNaN\tasserted\ts\twhy\tlive\n",
        "accept\t0\ta\t1\tb\t2\t0.9\tmaybe\ts\twhy\tlive\n",
        "accept\t0\ta\t1\tb\t2\t0.9\tasserted\ts\twhy\tdead\n",
        "accept\t0\t\t1\tb\t2\t0.9\tasserted\ts\twhy\tlive\n",
        "accept\t0\ta\tx\tb\t2\t0.9\tasserted\ts\twhy\tlive\n",
    ] {
        assert!(Alignment::from_text(bad).is_err(), "accepted: {bad:?}");
    }
    assert!(Alignment::from_text("").unwrap().log().is_empty());
}

fn link(g: &mut Graph, a: EntityId, b: EntityId) {
    g.add_relation(a, ontologicant_core::RelationType::new("rel"), b, src())
        .unwrap();
}

/// left: h1 - x - h2     right: H1 - X - H2 - Y     (h*/H* anchored; x~X shares two, nothing shares with Y but H2)
#[test]
fn structural_needs_anchors_and_counts_shared_neighbours() {
    let (mut a, ae) = graph(&["h1", "x", "h2"]);
    link(&mut a, ae[1], ae[0]);
    link(&mut a, ae[1], ae[2]);
    let (mut b, be) = graph(&["H1", "X", "H2", "Y"]);
    link(&mut b, be[1], be[0]);
    link(&mut b, be[1], be[2]);
    link(&mut b, be[3], be[2]);
    let (na, nb) = (gn("a"), gn("b"));
    let (sa, sb) = (side(&na, &a, None), side(&nb, &b, None));

    // No anchors: nothing, whatever the shape.
    assert!(
        propose_structural(sa, sb, &Alignment::new())
            .unwrap()
            .is_empty()
    );

    let mut al = Alignment::new();
    al.accept(&corr(r("a", ae[0]), r("b", be[0])), src())
        .unwrap();
    al.accept(&corr(r("a", ae[2]), r("b", be[2])), src())
        .unwrap();
    let p = propose_structural(sa, sb, &al).unwrap();
    let pair = |c: &Correspondence| (c.left.entity, c.right.entity);
    // x~X: both neighbours anchored: 2 / (2 + 2 - 2) = 1.0. x~Y: one shared: 1 / (2 + 1 - 1) = 0.5.
    assert_eq!(pair(&p[0]), (ae[1], be[1]));
    assert_eq!(p[0].score, 1.0);
    assert!(
        p.iter()
            .any(|c| pair(c) == (ae[1], be[3]) && c.score == 0.5)
    );
    // Anchored pairs are not re-proposed; a hub's own pairs with its anchors' neighbours still appear (shape only).
    assert!(p.iter().all(|c| pair(c) != (ae[0], be[0])));
    assert!(p[0].reasons[0].rule == "alignment.structural");
}

#[test]
fn structural_ignores_names_and_refuses_same_graph() {
    let (mut a, ae) = graph(&["same", "hub"]);
    link(&mut a, ae[0], ae[1]);
    let (mut b, be) = graph(&["different", "hub"]);
    link(&mut b, be[0], be[1]);
    let (na, nb) = (gn("a"), gn("b"));
    let mut al = Alignment::new();
    al.accept(&corr(r("a", ae[1]), r("b", be[1])), src())
        .unwrap();
    let p = propose_structural(side(&na, &a, None), side(&nb, &b, None), &al).unwrap();
    assert_eq!(p.len(), 1, "names differ yet the shape proposes it");
    assert!(matches!(
        propose_structural(side(&na, &a, None), side(&na, &a, None), &al),
        Err(AlignError::SameGraph(_))
    ));
}
