// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(missing_docs)]
#![cfg(feature = "json")]
use ontologicant_alignment::*;
use ontologicant_core::{AssertionSource, EntityId, Reason};

fn corr(lg: &str, l: u64, rg: &str, r: u64) -> Correspondence {
    Correspondence {
        left: Ref {
            graph: GraphName::new(lg).unwrap(),
            entity: EntityId(l),
        },
        right: Ref {
            graph: GraphName::new(rg).unwrap(),
            entity: EntityId(r),
        },
        relation: Relation::Equivalent,
        score: 0.75,
        reasons: vec![Reason::new("alignment.match", "t\tab")],
    }
}

fn sample() -> Alignment {
    let mut al = Alignment::new();
    let id = al
        .accept(&corr("a\\", 1, "b", 2), AssertionSource::asserted("me"))
        .unwrap();
    al.retract(id).unwrap();
    al.accept(&corr("a\\", 1, "b", 2), AssertionSource::derived("x\ny"))
        .unwrap();
    al
}

#[test]
fn json_round_trips_and_agrees_with_the_text_form() {
    let al = sample();
    let back = Alignment::from_json(&al.to_json()).unwrap();
    assert_eq!(back.log(), al.log());
    assert_eq!(back.to_text(), al.to_text());
}

#[test]
fn json_has_a_documented_shape() {
    let v: serde_json::Value = serde_json::from_str(&sample().to_json()).unwrap();
    assert_eq!(v["version"], 1);
    assert_eq!(v["entries"][0]["left_graph"], "a\\");
    assert_eq!(v["entries"][0]["retracted"], true);
    assert_eq!(v["entries"][1]["retracted"], false);
}

#[test]
fn json_is_validated_like_the_text_form() {
    let good = sample().to_json();
    for bad in [
        "not json".to_owned(),
        good.replace("\"version\":1", "\"version\":2"),
        good.replace("\"left_graph\":\"a\\\\\"", "\"left_graph\":\"b\""),
        good.replace("\"origin\":\"derived\"", "\"origin\":\"maybe\""),
        good.replace("\"score\":0.75", "\"score\":7.5"),
        good.replace("\"score\":0.75", "\"score\":null"),
        // Two live entries for one pair.
        good.replace("\"retracted\":true", "\"retracted\":false"),
    ] {
        assert_ne!(bad, good, "mutation did not apply");
        assert!(Alignment::from_json(&bad).is_err(), "accepted: {bad}");
    }
    assert!(Alignment::from_json(&good).is_ok());
}
