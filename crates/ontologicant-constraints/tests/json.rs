// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(missing_docs)]
#![cfg(feature = "json")]
use ontologicant_constraints::*;
use ontologicant_graph::{EntityId, RelationType};

fn sample() -> ConstraintSet {
    let mut s = ConstraintSet::new();
    s.add(Constraint::Disjoint {
        a: EntityId(1),
        b: EntityId(2),
    });
    s.add(Constraint::Cardinality {
        rtype: RelationType::new("has\tpart"),
        min: 1,
        max: None,
        of: Some(EntityId(4)),
    });
    s.add(Constraint::Domain {
        rtype: RelationType::new("d"),
        kind: EntityId(5),
    });
    s.add(Constraint::Functional(RelationType::new("f")));
    s.add(Constraint::Irreflexive(RelationType::new("z")));
    s
}

#[test]
fn json_round_trips_and_agrees_with_the_text_form() {
    let c = sample();
    let back = ConstraintSet::from_json(&c.to_json()).unwrap();
    assert_eq!(back.constraints(), c.constraints());
    assert_eq!(back.to_text(), c.to_text());
}

#[test]
fn json_has_a_documented_shape() {
    let v: serde_json::Value = serde_json::from_str(&sample().to_json()).unwrap();
    assert_eq!(v["version"], 1);
    assert_eq!(v["constraints"][0]["disjoint"]["a"], 1);
    assert_eq!(
        v["constraints"][1]["cardinality"]["max"],
        serde_json::Value::Null
    );
    assert_eq!(v["constraints"][3]["functional"], "f");
}

#[test]
fn from_json_rejects_bad_input() {
    assert!(ConstraintSet::from_json("{}").is_err());
    assert!(ConstraintSet::from_json(r#"{"version":2,"constraints":[]}"#).is_err());
    assert!(ConstraintSet::from_json(r#"{"version":1,"constraints":[{"nonsense":1}]}"#).is_err());
    assert!(ConstraintSet::from_json(r#"{"version":1,"constraints":[]}"#).is_ok());
}
