// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(missing_docs)]
use ontologicant_core::{AssertionSource, EntityId};
use ontologicant_identity::*;

fn e(n: u64) -> EntityId {
    EntityId(n)
}
fn src() -> AssertionSource {
    AssertionSource::asserted("test")
}

#[test]
fn unmerged_entity_is_its_own_canonical_id_and_class() {
    let id = IdentityIndex::new();
    assert_eq!(id.canonical(e(5)), e(5));
    assert_eq!(id.class_of(e(5)), vec![e(5)]);
}

#[test]
fn merge_makes_lowest_id_canonical_for_the_whole_class() {
    let mut id = IdentityIndex::new();
    id.merge(e(7), e(3), "same", src()).unwrap();
    id.merge(e(9), e(7), "same", src()).unwrap();
    for x in [3, 7, 9] {
        assert_eq!(id.canonical(e(x)), e(3));
    }
    assert_eq!(id.class_of(e(9)), vec![e(3), e(7), e(9)]);
    assert!(id.same(e(3), e(9)));
    assert!(!id.same(e(3), e(4)));
}

#[test]
fn merging_an_entity_with_itself_or_its_own_class_is_refused() {
    let mut id = IdentityIndex::new();
    assert_eq!(
        id.merge(e(1), e(1), "x", src()),
        Err(IdentityError::AlreadySame(e(1), e(1)))
    );
    id.merge(e(1), e(2), "x", src()).unwrap();
    assert_eq!(
        id.merge(e(2), e(1), "x", src()),
        Err(IdentityError::AlreadySame(e(2), e(1)))
    );
}

#[test]
fn split_detaches_one_entity_and_leaves_the_rest_merged() {
    let mut id = IdentityIndex::new();
    id.merge(e(1), e(2), "x", src()).unwrap();
    id.merge(e(2), e(3), "x", src()).unwrap();
    id.split(e(1), "wrong", src()).unwrap();
    assert_eq!(id.canonical(e(1)), e(1));
    assert_eq!(id.class_of(e(2)), vec![e(2), e(3)]);
    assert_eq!(id.canonical(e(3)), e(2));
}

#[test]
fn splitting_a_singleton_is_refused() {
    let mut id = IdentityIndex::new();
    assert_eq!(
        id.split(e(1), "x", src()),
        Err(IdentityError::NotMerged(e(1)))
    );
}

#[test]
fn undo_restores_the_state_before_a_merge_and_the_log_only_grows() {
    let mut id = IdentityIndex::new();
    let m = id.merge(e(1), e(2), "x", src()).unwrap();
    let before = id.log().len();
    let u = id.undo(m, "oops", src()).unwrap();
    assert_ne!(m, u);
    assert_eq!(id.canonical(e(2)), e(2));
    assert!(id.log().len() > before, "undo appends, it never removes");
    assert_eq!(id.log()[0].id, m, "the undone op is still in the log");
}

#[test]
fn undoing_an_early_merge_keeps_later_independent_ones() {
    let mut id = IdentityIndex::new();
    let m1 = id.merge(e(1), e(2), "x", src()).unwrap();
    id.merge(e(2), e(3), "x", src()).unwrap();
    id.undo(m1, "x", src()).unwrap();
    assert_eq!(id.class_of(e(2)), vec![e(2), e(3)]);
    assert_eq!(id.canonical(e(1)), e(1));
}

#[test]
fn undo_of_a_split_remerges() {
    let mut id = IdentityIndex::new();
    id.merge(e(1), e(2), "x", src()).unwrap();
    let s = id.split(e(2), "x", src()).unwrap();
    id.undo(s, "x", src()).unwrap();
    assert!(id.same(e(1), e(2)));
}

#[test]
fn undo_errors() {
    let mut id = IdentityIndex::new();
    let m = id.merge(e(1), e(2), "x", src()).unwrap();
    let u = id.undo(m, "x", src()).unwrap();
    assert_eq!(id.undo(m, "x", src()), Err(IdentityError::AlreadyUndone(m)));
    assert_eq!(id.undo(u, "x", src()), Err(IdentityError::NotUndoable(u)));
    assert_eq!(
        id.undo(OpId(99), "x", src()),
        Err(IdentityError::UnknownOp(OpId(99)))
    );
}

#[test]
fn ops_carry_why_and_source() {
    let mut id = IdentityIndex::new();
    let m = id
        .merge(e(1), e(2), "same doi", AssertionSource::asserted("pack-x"))
        .unwrap();
    let op = &id.log()[0];
    assert_eq!(op.id, m);
    assert_eq!(op.why, "same doi");
    assert_eq!(op.source, AssertionSource::asserted("pack-x"));
}

#[test]
fn aliases_are_kept_per_entity_with_source_and_follow_the_class() {
    let mut id = IdentityIndex::new();
    id.add_alias(e(2), "Felis catus", AssertionSource::asserted("pack-x"));
    id.add_alias(e(1), "cat", src());
    id.add_alias(e(1), "cat", src()); // duplicate (text, source): ignored
    assert_eq!(id.aliases(e(1)).len(), 1);
    id.merge(e(1), e(2), "x", src()).unwrap();
    let all: Vec<&str> = id
        .class_aliases(e(2))
        .iter()
        .map(|a| a.text.as_str())
        .collect();
    assert_eq!(all, vec!["cat", "Felis catus"]);
    assert_eq!(id.aliases(e(2)).len(), 1, "own aliases stay per entity");
}

#[test]
fn lookup_finds_entities_by_exact_alias() {
    let mut id = IdentityIndex::new();
    id.add_alias(e(2), "Felis catus", src());
    id.add_alias(e(1), "Felis catus", src());
    assert_eq!(id.lookup("Felis catus"), vec![e(1), e(2)]);
    assert!(id.lookup("felis catus").is_empty());
}

#[test]
fn normalise_folds_case_space_and_ascii_punctuation() {
    assert_eq!(normalise("  Felis\t CATUS. "), "felis catus");
    assert_eq!(normalise("U.S.A"), "usa");
    assert_eq!(normalise("a-b"), "a b", "hyphen separates words");
}

#[test]
fn exact_matcher_scores_only_identical_text() {
    let m = ExactMatcher;
    assert_eq!(m.compare("cat", "cat").unwrap().score, 1.0);
    assert!(m.compare("cat", "Cat").is_none());
}

#[test]
fn normalised_matcher_scores_identical_one_and_folded_lower() {
    let m = NormalisedMatcher;
    assert_eq!(m.compare("cat", "cat").unwrap().score, 1.0);
    let f = m.compare("Cat ", "cat").unwrap();
    assert!(f.score < 1.0 && f.score > 0.0);
    assert!(m.compare("cat", "dog").is_none());
}

#[test]
fn proposals_pair_entities_in_different_classes_with_score_and_reasons() {
    let mut id = IdentityIndex::new();
    id.add_alias(e(1), "Felis catus", src());
    id.add_alias(e(2), "felis  catus", src());
    id.add_alias(e(3), "Canis lupus", src());
    let p = id.propose_merges(&NormalisedMatcher);
    assert_eq!(p.len(), 1);
    assert_eq!((p[0].a, p[0].b), (e(1), e(2)));
    assert!(p[0].score > 0.0 && p[0].score < 1.0);
    assert!(!p[0].reasons.is_empty());
    assert!(id.propose_merges(&ExactMatcher).is_empty());
}

#[test]
fn proposals_skip_already_merged_pairs_and_applying_one_is_a_separate_logged_step() {
    let mut id = IdentityIndex::new();
    id.add_alias(e(1), "cat", src());
    id.add_alias(e(2), "cat", src());
    let p = id.propose_merges(&ExactMatcher);
    assert_eq!(p.len(), 1);
    assert_eq!(id.log().len(), 0, "proposing changes nothing");
    id.apply(&p[0], src()).unwrap();
    assert_eq!(id.log().len(), 1);
    assert!(id.propose_merges(&ExactMatcher).is_empty());
}

#[test]
fn proposal_takes_best_score_over_alias_pairs_and_is_sorted() {
    let mut id = IdentityIndex::new();
    id.add_alias(e(1), "Cat", src());
    id.add_alias(e(1), "kitty", src());
    id.add_alias(e(2), "cat", src());
    id.add_alias(e(2), "kitty", src());
    id.add_alias(e(5), "x", src());
    id.add_alias(e(4), "x", src());
    let p = id.propose_merges(&NormalisedMatcher);
    assert_eq!(p.len(), 2);
    assert_eq!((p[0].a, p[0].b), (e(1), e(2)));
    assert_eq!(p[0].score, 1.0, "kitty = kitty beats Cat ~ cat");
    assert_eq!((p[1].a, p[1].b), (e(4), e(5)), "a < b within a pair");
}

#[test]
fn text_form_round_trips_and_replays_to_the_same_state() {
    let mut id = IdentityIndex::new();
    id.add_alias(e(1), "a b", src());
    let m = id.merge(e(1), e(2), "why with | bar", src()).unwrap();
    id.merge(e(2), e(3), "y", AssertionSource::derived("t"))
        .unwrap();
    id.undo(m, "z", src()).unwrap();
    let text = id.to_text();
    let back = IdentityIndex::from_text(&text).unwrap();
    assert_eq!(back.to_text(), text);
    assert_eq!(back.class_of(e(2)), id.class_of(e(2)));
    assert_eq!(back.log(), id.log());
    assert_eq!(back.aliases(e(1)), id.aliases(e(1)));
}

#[test]
fn from_text_rejects_garbage() {
    assert!(IdentityIndex::from_text("bogus line").is_err());
    assert!(
        IdentityIndex::from_text("undo\t1\t0\tasserted\ts\twhy").is_err(),
        "undo of unknown op"
    );
}

#[test]
fn splitting_a_pair_leaves_two_singletons_neither_splittable() {
    let mut id = IdentityIndex::new();
    id.merge(e(1), e(2), "x", src()).unwrap();
    id.split(e(1), "x", src()).unwrap();
    assert_eq!(
        id.split(e(2), "x", src()),
        Err(IdentityError::NotMerged(e(2)))
    );
    assert_eq!(id.class_of(e(2)), vec![e(2)]);
}

#[test]
fn from_text_rejects_out_of_order_op_ids_and_unescapes_tabs() {
    assert!(IdentityIndex::from_text("merge\t5\t1\t2\tasserted\ts\twhy").is_err());
    let mut id = IdentityIndex::new();
    id.add_alias(e(1), "a\tb\nc\\d", src());
    let back = IdentityIndex::from_text(&id.to_text()).unwrap();
    assert_eq!(back.aliases(e(1))[0].text, "a\tb\nc\\d");
    assert_eq!(
        id.to_text().lines().count(),
        1,
        "tabs and newlines in text are escaped"
    );
}
