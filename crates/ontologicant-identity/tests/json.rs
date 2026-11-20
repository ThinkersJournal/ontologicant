// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(missing_docs)]
#![cfg(feature = "json")]
use ontologicant_identity::*;

fn e(n: u64) -> EntityId {
    EntityId(n)
}
fn src() -> AssertionSource {
    AssertionSource::asserted("t")
}
fn sample() -> IdentityIndex {
    let mut id = IdentityIndex::new();
    id.add_alias(e(1), "a\tb", src());
    id.add_alias(e(2), "A B", AssertionSource::derived("pack"));
    let m = id.merge(e(1), e(2), "same", src()).unwrap();
    id.merge(e(2), e(3), "x", src()).unwrap();
    id.undo(m, "oops", src()).unwrap();
    id
}

#[test]
fn json_round_trips_aliases_log_and_state() {
    let id = sample();
    let back = IdentityIndex::from_json(&id.to_json()).unwrap();
    assert_eq!(back.log(), id.log());
    assert_eq!(back.aliases(e(2)), id.aliases(e(2)));
    assert_eq!(back.class_of(e(2)), id.class_of(e(2)));
    assert_eq!(back.to_text(), id.to_text(), "JSON and text forms agree");
}

#[test]
fn json_has_a_documented_shape() {
    let v: serde_json::Value = serde_json::from_str(&sample().to_json()).unwrap();
    assert_eq!(v["version"], 1);
    assert_eq!(v["aliases"][0]["entity"], 1);
    assert_eq!(v["aliases"][0]["alias"]["text"], "a\tb");
    assert_eq!(v["log"][0]["id"], 1);
    assert_eq!(v["log"][0]["kind"]["Merge"]["a"], 1);
    assert_eq!(v["log"][2]["kind"]["Undo"]["target"], 1);
}

#[test]
fn from_json_replays_the_log_through_the_same_validation() {
    let good = sample().to_json();
    let edit = |f: &dyn Fn(&mut serde_json::Value)| {
        let mut v: serde_json::Value = serde_json::from_str(&good).unwrap();
        f(&mut v);
        IdentityIndex::from_json(&v.to_string())
    };
    assert!(edit(&|v| v["version"] = 2.into()).is_err());
    assert!(
        edit(&|v| v["log"][1]["id"] = 7.into()).is_err(),
        "ids must count up from 1"
    );
    assert!(
        edit(&|v| v["log"][2]["kind"]["Undo"]["target"] = 9.into()).is_err(),
        "unknown op"
    );
    assert!(
        edit(&|v| v["log"][1]["kind"]["Merge"]["b"] = 1.into()).is_err(),
        "merge of one entity with itself"
    );
    assert!(IdentityIndex::from_json("[]").is_err());
}
