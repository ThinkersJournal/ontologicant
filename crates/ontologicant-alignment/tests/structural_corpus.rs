// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(missing_docs)]
//! A fixed synthetic corpus for measuring `propose_structural`. NOT a real-world result: it measures how the scorer
//! behaves on graphs with a few high-degree hubs, which is the case the docs warn about.
//!
//! Corpus (deterministic, no RNG crate): per seed, a base graph of `N` entities in which `HUBS` hub entities attract
//! `HUB_SHARE` percent of `EDGES` edges. The right graph is the same entities in a shuffled insertion order (so ids
//! differ), each edge kept with `KEEP` percent and `NOISE` extra random edges added. Gold = same base entity. A random
//! `ANCHOR_PCT` percent of gold pairs are accepted as anchors; the rest are scored.
use ontologicant_alignment::*;
use ontologicant_core::{AssertionSource, EntityId, RelationType};
use ontologicant_graph::{Graph, GraphStore};
use std::collections::BTreeMap;

const N: usize = 200;
const HUBS: usize = 5;
const EDGES: usize = 600;
const HUB_SHARE: u64 = 40;
const KEEP: u64 = 80;
const NOISE: usize = 60;
const ANCHOR_PCT: u64 = 30;
const SEEDS: [u64; 5] = [1, 2, 3, 4, 5];

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

struct Case {
    left: Graph,
    right: Graph,
    /// gold: left entity -> right entity
    gold: BTreeMap<EntityId, EntityId>,
}

fn build(seed: u64) -> Case {
    let mut rng = Lcg(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for _ in 0..EDGES {
        let a = rng.below(N as u64) as usize;
        let b = if rng.below(100) < HUB_SHARE {
            rng.below(HUBS as u64) as usize
        } else {
            rng.below(N as u64) as usize
        };
        if a != b {
            edges.push((a, b));
        }
    }
    let src = AssertionSource::asserted("corpus");
    let rel = RelationType::new("rel");
    let mut left = Graph::new();
    let lid: Vec<EntityId> = (0..N).map(|i| left.add_entity(&format!("l{i}"))).collect();
    for &(a, b) in &edges {
        let _ = left.add_relation(lid[a], rel.clone(), lid[b], src.clone());
    }
    // Shuffled insertion order for the right graph (Fisher-Yates).
    let mut order: Vec<usize> = (0..N).collect();
    for i in (1..N).rev() {
        order.swap(i, rng.below(i as u64 + 1) as usize);
    }
    let mut right = Graph::new();
    let mut rid = vec![EntityId(0); N];
    for &i in &order {
        rid[i] = right.add_entity(&format!("r{i}"));
    }
    for &(a, b) in &edges {
        if rng.below(100) < KEEP {
            let _ = right.add_relation(rid[a], rel.clone(), rid[b], src.clone());
        }
    }
    for _ in 0..NOISE {
        let (a, b) = (rng.below(N as u64) as usize, rng.below(N as u64) as usize);
        if a != b {
            let _ = right.add_relation(rid[a], rel.clone(), rid[b], src.clone());
        }
    }
    let gold = (0..N).map(|i| (lid[i], rid[i])).collect();
    Case { left, right, gold }
}

#[derive(Debug, Clone, Copy)]
struct Metrics {
    /// Correct top-1 proposals / left entities that received any proposal.
    precision_at_1: f64,
    /// Correct top-1 proposals / unanchored gold pairs.
    recall_at_1: f64,
}

fn measure(propose: &dyn Fn(Side<'_>, Side<'_>, &Alignment) -> Vec<Correspondence>) -> Metrics {
    let (mut hit, mut proposed, mut total) = (0usize, 0usize, 0usize);
    for seed in SEEDS {
        let case = build(seed);
        let (nl, nr) = (
            GraphName::new("left").unwrap(),
            GraphName::new("right").unwrap(),
        );
        let mut rng = Lcg(seed ^ 0xABCD);
        let mut anchors = Alignment::new();
        let mut anchored = std::collections::BTreeSet::new();
        for (&l, &r) in &case.gold {
            if rng.below(100) < ANCHOR_PCT {
                let c = Correspondence {
                    left: Ref {
                        graph: nl.clone(),
                        entity: l,
                    },
                    right: Ref {
                        graph: nr.clone(),
                        entity: r,
                    },
                    relation: Relation::Equivalent,
                    score: 1.0,
                    reasons: vec![ontologicant_core::Reason::new("corpus.gold", "anchor")],
                };
                anchors
                    .accept(&c, AssertionSource::asserted("corpus"))
                    .unwrap();
                anchored.insert(l);
            }
        }
        let sa = Side {
            name: &nl,
            graph: &case.left,
            identity: None,
        };
        let sb = Side {
            name: &nr,
            graph: &case.right,
            identity: None,
        };
        let props = propose(sa, sb, &anchors);
        let mut top: BTreeMap<EntityId, EntityId> = BTreeMap::new();
        for c in props.iter().filter(|c| !anchored.contains(&c.left.entity)) {
            top.entry(c.left.entity).or_insert(c.right.entity); // already sorted best-first
        }
        total += case.gold.keys().filter(|l| !anchored.contains(l)).count();
        for (l, r) in &top {
            proposed += 1;
            if case.gold.get(l) == Some(r) {
                hit += 1;
            }
        }
    }
    Metrics {
        precision_at_1: hit as f64 / proposed as f64,
        recall_at_1: hit as f64 / total as f64,
    }
}

/// Measured at main 8792a29 on this corpus: precision@1 0.5503, recall@1 0.3935. The floors sit a little below, so a
/// scorer change that makes the proposals worse fails here; a change that is better should raise them, with the new
/// numbers in the commit message.
///
/// A hub-damped alternative (weight `ln 2 / ln(1 + degree)` per neighbour, weighted Dice) was measured on the same
/// corpus and was slightly WORSE (precision@1 0.5339, recall@1 0.3818), so it was not adopted.
#[test]
fn structural_scoring_holds_its_measured_floor() {
    let m = measure(&|a, b, al| propose_structural(a, b, al).unwrap());
    println!("CORPUS jaccard {m:?}");
    assert!(
        m.precision_at_1 >= 0.54,
        "precision@1 fell to {}",
        m.precision_at_1
    );
    assert!(m.recall_at_1 >= 0.385, "recall@1 fell to {}", m.recall_at_1);
}
