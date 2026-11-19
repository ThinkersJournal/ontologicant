// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(missing_docs)]
use ontologicant_constraints::*;
use ontologicant_graph::{AssertionSource, EntityId, Graph, GraphStore, RelationType};
use ontologicant_kinds::{Property, Vocabulary};

fn t(s: &str) -> RelationType {
    RelationType::new(s)
}
fn link(g: &mut Graph, a: EntityId, ty: &str, b: EntityId) {
    g.add_relation(a, t(ty), b, AssertionSource::asserted("test"))
        .unwrap();
}
fn check(g: &Graph, cs: &[Constraint]) -> Report {
    let mut set = ConstraintSet::new();
    for c in cs {
        set.add(c.clone());
    }
    set.check(g, &t("is-a")).unwrap()
}

/// animal <- cat <- tabby ; animal <- dog ; thing (unrelated)
fn zoo() -> (Graph, [EntityId; 6]) {
    let mut g = Graph::new();
    let animal = g.add_entity("animal");
    let cat = g.add_entity("cat");
    let tabby = g.add_entity("tabby");
    let dog = g.add_entity("dog");
    let thing = g.add_entity("thing");
    let rex = g.add_entity("rex");
    link(&mut g, cat, "is-a", animal);
    link(&mut g, tabby, "is-a", cat);
    link(&mut g, dog, "is-a", animal);
    link(&mut g, rex, "is-a", dog);
    (g, [animal, cat, tabby, dog, thing, rex])
}

#[test]
fn empty_set_is_consistent() {
    let (g, _) = zoo();
    assert!(check(&g, &[]).is_consistent());
}

#[test]
fn disjoint_kinds_are_fine_when_no_entity_is_below_both() {
    let (g, [_, cat, _, dog, ..]) = zoo();
    assert!(check(&g, &[Constraint::Disjoint { a: cat, b: dog }]).is_consistent());
}

#[test]
fn disjoint_kinds_with_a_common_instance_are_reported_with_the_witness() {
    let (mut g, [_, cat, tabby, dog, ..]) = zoo();
    link(&mut g, tabby, "is-a", dog); // tabby is now below both
    let r = check(&g, &[Constraint::Disjoint { a: cat, b: dog }]);
    assert_eq!(r.violations.len(), 1);
    let v = &r.violations[0];
    assert_eq!(v.entities, vec![tabby, cat, dog]);
    assert!(!v.reasons.is_empty());
    assert_eq!(v.reasons[0].rule, "constraints.disjoint");
}

#[test]
fn disjoint_counts_a_kind_as_its_own_instance() {
    let (mut g, [_, cat, _, dog, ..]) = zoo();
    link(&mut g, cat, "is-a", dog); // cat is below dog and is cat
    let r = check(&g, &[Constraint::Disjoint { a: cat, b: dog }]);
    assert!(r.violations.iter().any(|v| v.entities[0] == cat));
}

#[test]
fn functional_flags_two_distinct_targets_but_not_a_repeated_one() {
    let (mut g, [animal, cat, _, dog, thing, _]) = zoo();
    link(&mut g, thing, "has-parent", cat);
    link(&mut g, thing, "has-parent", cat);
    assert!(check(&g, &[Constraint::Functional(t("has-parent"))]).is_consistent());
    link(&mut g, thing, "has-parent", dog);
    let r = check(&g, &[Constraint::Functional(t("has-parent"))]);
    assert_eq!(r.violations.len(), 1);
    assert_eq!(r.violations[0].entities, vec![thing, cat, dog]);
    let _ = animal;
}

#[test]
fn inverse_functional_flags_two_distinct_sources() {
    let (mut g, [animal, cat, _, dog, ..]) = zoo();
    link(&mut g, cat, "owns", animal);
    assert!(check(&g, &[Constraint::InverseFunctional(t("owns"))]).is_consistent());
    link(&mut g, dog, "owns", animal);
    let r = check(&g, &[Constraint::InverseFunctional(t("owns"))]);
    assert_eq!(r.violations.len(), 1);
    assert_eq!(r.violations[0].entities, vec![animal, cat, dog]);
}

#[test]
fn irreflexive_flags_self_loops() {
    let (mut g, [_, cat, _, dog, ..]) = zoo();
    link(&mut g, cat, "likes", dog);
    assert!(check(&g, &[Constraint::Irreflexive(t("likes"))]).is_consistent());
    link(&mut g, cat, "likes", cat);
    let r = check(&g, &[Constraint::Irreflexive(t("likes"))]);
    assert_eq!(r.violations.len(), 1);
    assert_eq!(r.violations[0].entities, vec![cat]);
}

#[test]
fn cardinality_min_and_max_count_relations_per_entity() {
    let (mut g, [animal, cat, tabby, dog, thing, rex]) = zoo();
    for x in [cat, dog, thing] {
        link(&mut g, x, "eats", animal);
    }
    link(&mut g, cat, "eats", tabby);
    link(&mut g, cat, "eats", rex);
    let max2 = Constraint::Cardinality {
        rtype: t("eats"),
        min: 0,
        max: Some(2),
        of: Some(cat),
    };
    let r = check(&g, &[max2]);
    assert_eq!(r.violations.len(), 1, "cat eats three things");
    assert_eq!(r.violations[0].entities, vec![cat]);
    let min1 = Constraint::Cardinality {
        rtype: t("eats"),
        min: 1,
        max: None,
        of: Some(thing),
    };
    assert!(check(&g, &[min1]).is_consistent(), "thing eats one");
}

#[test]
fn cardinality_of_a_kind_covers_its_instances_and_none_covers_all() {
    let (g, [animal, cat, tabby, dog, thing, rex]) = zoo();
    let c = Constraint::Cardinality {
        rtype: t("eats"),
        min: 1,
        max: None,
        of: Some(cat),
    };
    let r = check(&g, &[c]);
    let flagged: Vec<EntityId> = r.violations.iter().map(|v| v.entities[0]).collect();
    assert_eq!(flagged, vec![cat, tabby], "cat and tabby, not dog or rex");
    let all = Constraint::Cardinality {
        rtype: t("eats"),
        min: 1,
        max: None,
        of: None,
    };
    assert_eq!(check(&g, &[all]).violations.len(), 6);
    let _ = (animal, dog, thing, rex);
}

#[test]
fn domain_and_range_use_the_taxonomy() {
    let (mut g, [animal, cat, tabby, dog, thing, rex]) = zoo();
    link(&mut g, tabby, "chases", rex); // tabby is a cat, rex is a dog
    let dom = Constraint::Domain {
        rtype: t("chases"),
        kind: cat,
    };
    let ran = Constraint::Range {
        rtype: t("chases"),
        kind: dog,
    };
    assert!(check(&g, &[dom.clone(), ran.clone()]).is_consistent());
    link(&mut g, thing, "chases", rex);
    link(&mut g, cat, "chases", animal);
    let r = check(&g, &[dom, ran]);
    assert_eq!(r.violations.len(), 2);
    assert_eq!(
        r.violations[0].entities,
        vec![thing],
        "domain: thing is not a cat"
    );
    assert_eq!(
        r.violations[1].entities,
        vec![animal],
        "range: animal is not a dog"
    );
    assert_eq!(r.violations[0].reasons[0].rule, "constraints.domain");
    assert_eq!(r.violations[1].reasons[0].rule, "constraints.range");
}

#[test]
fn violations_are_ordered_by_constraint_then_entity() {
    let (mut g, [_, cat, _, dog, thing, _]) = zoo();
    link(&mut g, dog, "likes", dog);
    link(&mut g, cat, "likes", cat);
    link(&mut g, thing, "has-parent", cat);
    link(&mut g, thing, "has-parent", dog);
    let r = check(
        &g,
        &[
            Constraint::Functional(t("has-parent")),
            Constraint::Irreflexive(t("likes")),
        ],
    );
    let firsts: Vec<EntityId> = r.violations.iter().map(|v| v.entities[0]).collect();
    assert_eq!(firsts, vec![thing, cat, dog]);
    assert_eq!(
        r.violations[0].constraint,
        Constraint::Functional(t("has-parent"))
    );
}

#[test]
fn a_cyclic_taxonomy_is_an_error_not_a_report() {
    let (mut g, [animal, cat, ..]) = zoo();
    link(&mut g, animal, "is-a", cat);
    let mut set = ConstraintSet::new();
    set.add(Constraint::Irreflexive(t("x")));
    assert!(set.check(&g, &t("is-a")).is_err());
}

#[test]
fn from_vocabulary_takes_declared_functional_inverse_functional_and_irreflexive() {
    let mut v = Vocabulary::new();
    v.declare(t("a"), Property::Functional);
    v.declare(t("b"), Property::InverseFunctional);
    v.declare(t("c"), Property::Irreflexive);
    v.declare(t("d"), Property::Transitive);
    let set = ConstraintSet::from_vocabulary(&v);
    assert_eq!(
        set.constraints(),
        &[
            Constraint::Functional(t("a")),
            Constraint::InverseFunctional(t("b")),
            Constraint::Irreflexive(t("c")),
        ]
    );
}

#[test]
fn text_form_round_trips_every_constraint_kind() {
    let mut set = ConstraintSet::new();
    set.add(Constraint::Disjoint {
        a: EntityId(1),
        b: EntityId(2),
    });
    set.add(Constraint::Cardinality {
        rtype: t("has\tpart"),
        min: 1,
        max: Some(3),
        of: Some(EntityId(4)),
    });
    set.add(Constraint::Cardinality {
        rtype: t("x"),
        min: 0,
        max: None,
        of: None,
    });
    set.add(Constraint::Domain {
        rtype: t("d"),
        kind: EntityId(5),
    });
    set.add(Constraint::Range {
        rtype: t("r"),
        kind: EntityId(6),
    });
    set.add(Constraint::Functional(t("f")));
    set.add(Constraint::InverseFunctional(t("i")));
    set.add(Constraint::Irreflexive(t("z")));
    let text = set.to_text();
    assert_eq!(text.lines().count(), 8);
    let back = ConstraintSet::from_text(&text).unwrap();
    assert_eq!(back.constraints(), set.constraints());
    assert_eq!(back.to_text(), text);
}

#[test]
fn from_text_rejects_garbage() {
    assert!(ConstraintSet::from_text("nonsense\tx").is_err());
    assert!(ConstraintSet::from_text("disjoint\te1").is_err());
    assert!(ConstraintSet::from_text("cardinality\tr\tx\t*\t*").is_err());
    assert!(ConstraintSet::from_text("domain\tr\tnotanid").is_err());
}

#[test]
fn cardinality_bounds_are_inclusive() {
    let (mut g, [animal, cat, tabby, _, thing, _]) = zoo();
    link(&mut g, thing, "eats", animal);
    link(&mut g, thing, "eats", cat);
    let ok = Constraint::Cardinality {
        rtype: t("eats"),
        min: 2,
        max: Some(2),
        of: Some(thing),
    };
    assert!(
        check(&g, &[ok]).is_consistent(),
        "exactly 2 is within 2..=2"
    );
    let _ = tabby;
}

#[test]
fn violations_within_a_constraint_are_in_entity_order_not_relation_order() {
    let (mut g, [_, cat, _, _, thing, rex]) = zoo();
    link(&mut g, rex, "chases", cat); // added first, but higher id
    link(&mut g, thing, "chases", cat);
    let r = check(
        &g,
        &[Constraint::Domain {
            rtype: t("chases"),
            kind: cat,
        }],
    );
    let order: Vec<EntityId> = r.violations.iter().map(|v| v.entities[0]).collect();
    assert_eq!(order, vec![thing, rex]);
}
