// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(missing_docs)]
use std::collections::BTreeSet;

use ontologicant_graph::{AssertionSource, EntityId, Graph, GraphStore, RelationType};
use ontologicant_kinds::*;

fn t(s: &str) -> RelationType {
    RelationType::new(s)
}
fn p() -> AssertionSource {
    AssertionSource::asserted("test")
}
fn ents(g: &mut Graph, n: usize) -> Vec<EntityId> {
    (0..n).map(|i| g.add_entity(&format!("n{i}"))).collect()
}
fn link(g: &mut Graph, a: EntityId, ty: &str, b: EntityId) {
    g.add_relation(a, t(ty), b, p()).unwrap();
}
fn set(v: &[EntityId]) -> BTreeSet<EntityId> {
    v.iter().copied().collect()
}

// ---- Order / Taxonomy

#[test]
fn taxonomy_ancestors_and_descendants_are_transitive() {
    let mut g = Graph::new();
    let e = ents(&mut g, 4);
    link(&mut g, e[0], "is-a", e[1]);
    link(&mut g, e[1], "is-a", e[2]);
    link(&mut g, e[3], "is-a", e[2]);
    let tax = Taxonomy::from_graph(&g, &t("is-a")).unwrap();
    assert_eq!(tax.ancestors(e[0]), set(&[e[1], e[2]]));
    assert_eq!(tax.descendants(e[2]), set(&[e[0], e[1], e[3]]));
    assert_eq!(tax.order().roots(), vec![e[2]]);
}

#[test]
fn only_the_named_relation_type_counts() {
    let mut g = Graph::new();
    let e = ents(&mut g, 2);
    link(&mut g, e[0], "part-of", e[1]);
    let tax = Taxonomy::from_graph(&g, &t("is-a")).unwrap();
    assert!(tax.ancestors(e[0]).is_empty());
}

#[test]
fn cycle_is_refused_and_named() {
    let mut g = Graph::new();
    let e = ents(&mut g, 4);
    link(&mut g, e[0], "is-a", e[1]);
    link(&mut g, e[1], "is-a", e[2]);
    link(&mut g, e[2], "is-a", e[1]);
    link(&mut g, e[2], "is-a", e[3]);
    let err = Taxonomy::from_graph(&g, &t("is-a")).unwrap_err();
    assert_eq!(err.path, vec![e[1], e[2], e[1]]);
    assert!(err.to_string().contains("e1 -> e2 -> e1"));
}

#[test]
fn self_loop_is_a_cycle() {
    let mut g = Graph::new();
    let e = ents(&mut g, 1);
    link(&mut g, e[0], "is-a", e[0]);
    assert_eq!(
        Taxonomy::from_graph(&g, &t("is-a")).unwrap_err().path,
        vec![e[0], e[0]]
    );
}

#[test]
fn a_diamond_is_not_a_cycle() {
    let mut g = Graph::new();
    let e = ents(&mut g, 4);
    link(&mut g, e[0], "is-a", e[1]);
    link(&mut g, e[0], "is-a", e[2]);
    link(&mut g, e[1], "is-a", e[3]);
    link(&mut g, e[2], "is-a", e[3]);
    assert!(Taxonomy::from_graph(&g, &t("is-a")).is_ok());
}

#[test]
fn is_a_gives_the_shortest_chain_as_reason_and_a_reason_when_false() {
    let mut g = Graph::new();
    let e = ents(&mut g, 4);
    link(&mut g, e[0], "is-a", e[1]);
    link(&mut g, e[1], "is-a", e[2]);
    link(&mut g, e[0], "is-a", e[2]); // shortcut
    let tax = Taxonomy::from_graph(&g, &t("is-a")).unwrap();
    let yes = tax.is_a(e[0], e[2]);
    assert!(yes.value);
    assert!(
        yes.reasons[0].note.contains("e0 -> e2"),
        "{:?}",
        yes.reasons
    );
    let no = tax.is_a(e[2], e[0]);
    assert!(!no.value && !no.reasons.is_empty());
    assert!(!tax.is_a(e[0], e[0]).value, "is-a is strict");
}

#[test]
fn lowest_common_ancestors_include_self_and_can_be_several() {
    let mut g = Graph::new();
    let e = ents(&mut g, 6);
    // e0,e1 both under e2 and e3; e2,e3 under e4; e5 under e4
    link(&mut g, e[0], "is-a", e[2]);
    link(&mut g, e[0], "is-a", e[3]);
    link(&mut g, e[1], "is-a", e[2]);
    link(&mut g, e[1], "is-a", e[3]);
    link(&mut g, e[2], "is-a", e[4]);
    link(&mut g, e[3], "is-a", e[4]);
    link(&mut g, e[5], "is-a", e[4]);
    let tax = Taxonomy::from_graph(&g, &t("is-a")).unwrap();
    assert_eq!(tax.lowest_common_ancestors(e[0], e[1]), set(&[e[2], e[3]]));
    assert_eq!(tax.lowest_common_ancestors(e[0], e[5]), set(&[e[4]]));
    assert_eq!(tax.lowest_common_ancestors(e[0], e[2]), set(&[e[2]]));
}

// ---- Mereology

fn whole_with_parts(g: &mut Graph, parts: usize) -> (EntityId, Vec<EntityId>) {
    let w = g.add_entity("whole");
    let ps: Vec<EntityId> = (0..parts)
        .map(|i| g.add_entity(&format!("part{i}")))
        .collect();
    for &x in &ps {
        link(g, x, "part-of", w);
    }
    (w, ps)
}

#[test]
fn mereology_parts_wholes_overlap() {
    let mut g = Graph::new();
    let e = ents(&mut g, 4);
    link(&mut g, e[0], "part-of", e[1]);
    link(&mut g, e[1], "part-of", e[2]);
    link(&mut g, e[3], "part-of", e[2]);
    let m = Mereology::from_graph(&g, &t("part-of")).unwrap();
    assert_eq!(m.parts(e[2]), set(&[e[0], e[1], e[3]]));
    assert_eq!(m.wholes(e[0]), set(&[e[1], e[2]]));
    assert!(m.overlap(e[1], e[2]) && m.overlap(e[0], e[0]) && m.overlap(e[0], e[1]));
    assert!(!m.overlap(e[0], e[3]));
}

#[test]
fn part_of_cycle_is_refused() {
    let mut g = Graph::new();
    let e = ents(&mut g, 2);
    link(&mut g, e[0], "part-of", e[1]);
    link(&mut g, e[1], "part-of", e[0]);
    assert!(Mereology::from_graph(&g, &t("part-of")).is_err());
}

#[test]
fn no_axioms_selected_means_nothing_is_checked() {
    let mut g = Graph::new();
    whole_with_parts(&mut g, 1);
    let m = Mereology::from_graph(&g, &t("part-of")).unwrap();
    assert!(m.violations(&Axioms::none()).is_empty());
}

#[test]
fn weak_supplementation_fails_for_a_single_part_and_passes_for_two_disjoint_parts() {
    let mut g = Graph::new();
    let (w, ps) = whole_with_parts(&mut g, 1);
    let m = Mereology::from_graph(&g, &t("part-of")).unwrap();
    let v = m.violations(&Axioms::none().with(Axiom::WeakSupplementation));
    assert_eq!(v.len(), 1);
    assert_eq!(
        (v[0].axiom, v[0].entities.clone()),
        (Axiom::WeakSupplementation, vec![ps[0], w])
    );
    assert!(v[0].reason.contains("no proper part that does not overlap"));

    let mut g = Graph::new();
    whole_with_parts(&mut g, 2);
    let m = Mereology::from_graph(&g, &t("part-of")).unwrap();
    assert!(
        m.violations(&Axioms::none().with(Axiom::WeakSupplementation))
            .is_empty()
    );
}

#[test]
fn weak_supplementation_fails_when_the_other_part_overlaps() {
    // whole has parts a and b; both have the same part c: a and b overlap, so neither supplements the other
    let mut g = Graph::new();
    let e = ents(&mut g, 4); // 0 whole, 1 a, 2 b, 3 c
    link(&mut g, e[1], "part-of", e[0]);
    link(&mut g, e[2], "part-of", e[0]);
    link(&mut g, e[3], "part-of", e[1]);
    link(&mut g, e[3], "part-of", e[2]);
    let m = Mereology::from_graph(&g, &t("part-of")).unwrap();
    let v = m.violations(&Axioms::none().with(Axiom::WeakSupplementation));
    assert!(v.iter().any(|x| x.entities == vec![e[1], e[0]]), "{v:?}");
}

#[test]
fn extensionality_fails_for_two_wholes_with_the_same_parts() {
    let mut g = Graph::new();
    let e = ents(&mut g, 4);
    for w in [e[0], e[1]] {
        link(&mut g, e[2], "part-of", w);
        link(&mut g, e[3], "part-of", w);
    }
    let m = Mereology::from_graph(&g, &t("part-of")).unwrap();
    let v = m.violations(&Axioms::none().with(Axiom::Extensionality));
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].entities, vec![e[0], e[1]]);
}

#[test]
fn extensionality_ignores_atoms() {
    let mut g = Graph::new();
    let e = ents(&mut g, 2);
    link(&mut g, e[0], "part-of", e[1]);
    let m = Mereology::from_graph(&g, &t("part-of")).unwrap();
    assert!(
        m.violations(&Axioms::none().with(Axiom::Extensionality))
            .is_empty()
    );
}

// ---- Vocabulary / Closure

fn pairs(c: &Closure) -> Vec<(EntityId, EntityId)> {
    c.pairs().collect()
}

#[test]
fn undeclared_relation_closes_to_exactly_its_edges() {
    let mut g = Graph::new();
    let e = ents(&mut g, 3);
    link(&mut g, e[0], "r", e[1]);
    link(&mut g, e[1], "r", e[2]);
    let c = Vocabulary::new().closure(&g, &t("r"));
    assert_eq!(pairs(&c), vec![(e[0], e[1]), (e[1], e[2])]);
}

#[test]
fn transitive_closure_gives_the_chain_as_derivation() {
    let mut g = Graph::new();
    let e = ents(&mut g, 3);
    link(&mut g, e[0], "r", e[1]);
    link(&mut g, e[1], "r", e[2]);
    let mut v = Vocabulary::new();
    v.declare(t("r"), Property::Transitive);
    let c = v.closure(&g, &t("r"));
    assert_eq!(pairs(&c), vec![(e[0], e[1]), (e[0], e[2]), (e[1], e[2])]);
    let d = c.derivation(e[0], e[2]).unwrap();
    assert_eq!(d.len(), 2);
    assert_eq!(
        (d[0].from, d[0].to, d[1].from, d[1].to),
        (e[0], e[1], e[1], e[2])
    );
    assert!(c.explain(e[0], e[2]).value);
    let no = c.explain(e[2], e[0]);
    assert!(!no.value && no.reasons[0].rule == "kinds.closure.no-derivation");
}

#[test]
fn symmetric_adds_the_reverse_with_a_symmetric_step() {
    let mut g = Graph::new();
    let e = ents(&mut g, 2);
    link(&mut g, e[0], "r", e[1]);
    let mut v = Vocabulary::new();
    v.declare(t("r"), Property::Symmetric);
    let c = v.closure(&g, &t("r"));
    assert_eq!(pairs(&c), vec![(e[0], e[1]), (e[1], e[0])]);
    assert!(matches!(
        c.derivation(e[1], e[0]).unwrap()[0].step,
        Step::Symmetric(_)
    ));
    assert!(matches!(
        c.derivation(e[0], e[1]).unwrap()[0].step,
        Step::Direct(_)
    ));
}

#[test]
fn inverse_is_declared_both_ways_and_reads_the_other_type_backwards() {
    let mut g = Graph::new();
    let e = ents(&mut g, 2);
    link(&mut g, e[0], "parent-of", e[1]);
    let mut v = Vocabulary::new();
    v.declare_inverse(t("child-of"), t("parent-of"));
    assert_eq!(
        v.decl(&t("parent-of")).unwrap().inverse,
        Some(t("child-of"))
    );
    let c = v.closure(&g, &t("child-of"));
    assert_eq!(pairs(&c), vec![(e[1], e[0])]);
    assert!(matches!(
        c.derivation(e[1], e[0]).unwrap()[0].step,
        Step::Inverse(_)
    ));
}

#[test]
fn reflexive_relates_every_entity_to_itself() {
    let mut g = Graph::new();
    let e = ents(&mut g, 2);
    let mut v = Vocabulary::new();
    v.declare(t("r"), Property::Reflexive);
    let c = v.closure(&g, &t("r"));
    assert_eq!(pairs(&c), vec![(e[0], e[0]), (e[1], e[1])]);
    assert_eq!(c.derivation(e[0], e[0]).unwrap()[0].step, Step::Reflexive);
}

#[test]
fn symmetric_and_transitive_makes_an_equivalence_with_a_self_pair() {
    let mut g = Graph::new();
    let e = ents(&mut g, 3);
    link(&mut g, e[0], "same", e[1]);
    link(&mut g, e[1], "same", e[2]);
    let mut v = Vocabulary::new();
    v.declare(t("same"), Property::Symmetric)
        .declare(t("same"), Property::Transitive);
    let c = v.closure(&g, &t("same"));
    assert_eq!(c.pairs().count(), 9);
    assert!(c.derivation(e[2], e[0]).is_some() && c.derivation(e[0], e[0]).is_some());
}

#[test]
fn closure_is_deterministic() {
    let build = || {
        let mut g = Graph::new();
        let e = ents(&mut g, 4);
        link(&mut g, e[0], "r", e[1]);
        link(&mut g, e[0], "r", e[2]);
        link(&mut g, e[1], "r", e[3]);
        link(&mut g, e[2], "r", e[3]);
        let mut v = Vocabulary::new();
        v.declare(t("r"), Property::Transitive);
        format!("{:?}", v.closure(&g, &t("r")))
    };
    assert_eq!(build(), build());
}

#[test]
fn two_atoms_are_not_an_extensionality_violation() {
    let mut g = Graph::new();
    let e = ents(&mut g, 3);
    link(&mut g, e[0], "part-of", e[2]);
    link(&mut g, e[1], "part-of", e[2]);
    let m = Mereology::from_graph(&g, &t("part-of")).unwrap();
    assert!(
        m.violations(&Axioms::none().with(Axiom::Extensionality))
            .is_empty()
    );
}

#[test]
fn the_chain_given_is_the_shortest_not_the_first_found() {
    let mut g = Graph::new();
    let e = ents(&mut g, 6);
    link(&mut g, e[0], "is-a", e[1]);
    link(&mut g, e[1], "is-a", e[3]);
    link(&mut g, e[0], "is-a", e[2]);
    link(&mut g, e[2], "is-a", e[5]);
    link(&mut g, e[5], "is-a", e[3]);
    let o = Order::from_graph(&g, &t("is-a")).unwrap();
    assert_eq!(o.path(e[0], e[3]), Some(vec![e[0], e[1], e[3]]));
    assert_eq!(o.path(e[3], e[0]), None);
}

#[test]
fn when_a_pair_is_both_direct_and_symmetric_direct_wins() {
    let mut g = Graph::new();
    let e = ents(&mut g, 2);
    link(&mut g, e[0], "r", e[1]);
    link(&mut g, e[1], "r", e[0]);
    let mut v = Vocabulary::new();
    v.declare(t("r"), Property::Symmetric);
    let c = v.closure(&g, &t("r"));
    assert!(matches!(
        c.derivation(e[0], e[1]).unwrap()[0].step,
        Step::Direct(_)
    ));
    assert!(matches!(
        c.derivation(e[1], e[0]).unwrap()[0].step,
        Step::Direct(_)
    ));
}
