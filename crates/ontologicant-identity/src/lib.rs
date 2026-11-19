// SPDX-License-Identifier: MIT OR Apache-2.0
//! `ontologicant-identity`: which names and entities are the same thing.
//!
//! * [`IdentityIndex`] keeps aliases per entity and equivalence classes built by [`IdentityIndex::merge`] and
//!   [`IdentityIndex::split`]. Every change is an [`Op`] in an **append-only log**; the classes are a replay of the log.
//!   [`IdentityIndex::undo`] appends an undo op, it never deletes one, so history and provenance are kept.
//! * The canonical id of a class is its lowest [`EntityId`], so it is deterministic and survives replay.
//! * [`Matcher`] is a trait. [`ExactMatcher`] and [`NormalisedMatcher`] ship; fuzzy or model matchers can implement the
//!   trait elsewhere. [`IdentityIndex::propose_merges`] only proposes, with scores and reasons; applying a proposal is a
//!   separate logged step, [`IdentityIndex::apply`].
//!
//! `propose_merges` compares every pair of alias-holding entities (quadratic); it is the reference implementation,
//! not an index.
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub use ontologicant_core::{AssertionSource, EntityId, Origin, Reason};

/// Identifier of an [`Op`] in the log. Never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OpId(pub u64);

impl fmt::Display for OpId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "o{}", self.0)
    }
}

/// What an [`Op`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpKind {
    /// Put the classes of `a` and `b` together.
    Merge {
        /// First entity.
        a: EntityId,
        /// Second entity.
        b: EntityId,
    },
    /// Detach one entity from its class.
    Split {
        /// The entity detached.
        entity: EntityId,
    },
    /// Cancel an earlier merge or split.
    Undo {
        /// The op cancelled.
        target: OpId,
    },
}

/// One entry of the append-only log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Op {
    /// Its id (ids count up from 1 in log order).
    pub id: OpId,
    /// What it did.
    pub kind: OpKind,
    /// Why, in the author's words.
    pub why: String,
    /// Who or what did it.
    pub source: AssertionSource,
}

/// An alias of an entity: a name and where it came from.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Alias {
    /// The name, exactly as given.
    pub text: String,
    /// Where the alias came from.
    pub source: AssertionSource,
}

/// Why an identity operation or a text load failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentityError {
    /// The two entities are already in one class.
    AlreadySame(EntityId, EntityId),
    /// The entity is alone in its class, so there is nothing to split off.
    NotMerged(EntityId),
    /// No such op.
    UnknownOp(OpId),
    /// The op was already undone.
    AlreadyUndone(OpId),
    /// Undo ops themselves cannot be undone (undo the original op's effect by merging or splitting again).
    NotUndoable(OpId),
    /// The text form is malformed; says which line.
    Parse {
        /// 1-based line number.
        line: usize,
        /// What was wrong.
        problem: String,
    },
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityError::AlreadySame(a, b) => {
                write!(f, "{a} and {b} are already the same entity")
            }
            IdentityError::NotMerged(e) => write!(f, "{e} is not merged with anything"),
            IdentityError::UnknownOp(o) => write!(f, "no such operation {o}"),
            IdentityError::AlreadyUndone(o) => write!(f, "operation {o} was already undone"),
            IdentityError::NotUndoable(o) => {
                write!(f, "operation {o} is an undo and cannot be undone")
            }
            IdentityError::Parse { line, problem } => write!(f, "line {line}: {problem}"),
        }
    }
}
impl std::error::Error for IdentityError {}

/// A matcher's opinion that two names denote one thing.
#[derive(Clone, Debug, PartialEq)]
pub struct Match {
    /// Confidence in `(0, 1]`; `1.0` means the names are identical for this matcher.
    pub score: f64,
    /// What was compared and how.
    pub note: String,
}

/// Compares two names. Ship your own for fuzzy or model-based matching; the core stays deterministic and model-free.
pub trait Matcher {
    /// A short stable name, used in reasons.
    fn name(&self) -> &'static str;
    /// `Some` if the names match at all.
    fn compare(&self, a: &str, b: &str) -> Option<Match>;
}

/// Matches only identical text.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExactMatcher;

impl Matcher for ExactMatcher {
    fn name(&self) -> &'static str {
        "exact"
    }
    fn compare(&self, a: &str, b: &str) -> Option<Match> {
        (a == b).then(|| Match {
            score: 1.0,
            note: "identical text".into(),
        })
    }
}

/// Matches text that is equal after [`normalise`]: `1.0` if also identical, `0.9` if only the normal forms agree.
#[derive(Clone, Copy, Debug, Default)]
pub struct NormalisedMatcher;

impl Matcher for NormalisedMatcher {
    fn name(&self) -> &'static str {
        "normalised"
    }
    fn compare(&self, a: &str, b: &str) -> Option<Match> {
        if a == b {
            return Some(Match {
                score: 1.0,
                note: "identical text".into(),
            });
        }
        let (na, nb) = (normalise(a), normalise(b));
        (!na.is_empty() && na == nb).then(|| Match {
            score: 0.9,
            note: format!("equal after normalising to {na:?}"),
        })
    }
}

/// Lower-case, turn whitespace, `-` and `_` into single spaces, drop other ASCII punctuation, trim.
/// Non-ASCII characters are kept (no language-specific folding: that is data, not code).
pub fn normalise(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut gap = false;
    for c in s.chars().flat_map(char::to_lowercase) {
        if c.is_whitespace() || c == '-' || c == '_' {
            gap = true;
        } else if c.is_ascii_punctuation() {
            continue;
        } else {
            if gap && !out.is_empty() {
                out.push(' ');
            }
            gap = false;
            out.push(c);
        }
    }
    out
}

/// A suggested merge. Proposing changes nothing; see [`IdentityIndex::apply`].
#[derive(Clone, Debug, PartialEq)]
pub struct MergeProposal {
    /// The lower id of the pair.
    pub a: EntityId,
    /// The higher id of the pair.
    pub b: EntityId,
    /// The best score over all alias pairs.
    pub score: f64,
    /// The evidence: one reason for the best alias pair.
    pub reasons: Vec<Reason>,
}

/// Aliases, equivalence classes and the operation log behind them.
#[derive(Clone, Debug, Default)]
pub struct IdentityIndex {
    aliases: BTreeMap<EntityId, Vec<Alias>>,
    log: Vec<Op>,
    undone: BTreeSet<OpId>,
    /// Entity to its whole class, for entities in a class of two or more.
    classes: BTreeMap<EntityId, BTreeSet<EntityId>>,
}

impl IdentityIndex {
    /// An empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a name for an entity. Returns `false` (and changes nothing) if that exact name from that source exists.
    pub fn add_alias(
        &mut self,
        entity: EntityId,
        text: impl Into<String>,
        source: AssertionSource,
    ) -> bool {
        let alias = Alias {
            text: text.into(),
            source,
        };
        let list = self.aliases.entry(entity).or_default();
        if list.contains(&alias) {
            return false;
        }
        list.push(alias);
        true
    }

    /// The entity's own aliases, in the order added.
    pub fn aliases(&self, entity: EntityId) -> &[Alias] {
        self.aliases.get(&entity).map_or(&[], Vec::as_slice)
    }

    /// The aliases of every entity in the entity's class, in member id order.
    pub fn class_aliases(&self, entity: EntityId) -> Vec<&Alias> {
        self.class_of(entity)
            .into_iter()
            .flat_map(|m| self.aliases(m).iter())
            .collect()
    }

    /// Entities with an alias exactly equal to `text`, in id order.
    pub fn lookup(&self, text: &str) -> Vec<EntityId> {
        self.aliases
            .iter()
            .filter(|(_, l)| l.iter().any(|a| a.text == text))
            .map(|(e, _)| *e)
            .collect()
    }

    /// The class of an entity, sorted. An unmerged entity is alone.
    pub fn class_of(&self, entity: EntityId) -> Vec<EntityId> {
        match self.classes.get(&entity) {
            Some(c) => c.iter().copied().collect(),
            None => vec![entity],
        }
    }

    /// The canonical id: the lowest id in the class.
    pub fn canonical(&self, entity: EntityId) -> EntityId {
        self.classes
            .get(&entity)
            .and_then(|c| c.iter().next().copied())
            .unwrap_or(entity)
    }

    /// Are the two in one class?
    pub fn same(&self, a: EntityId, b: EntityId) -> bool {
        self.canonical(a) == self.canonical(b)
    }

    /// The whole log, oldest first, undone ops included.
    pub fn log(&self) -> &[Op] {
        &self.log
    }

    fn push(&mut self, kind: OpKind, why: &str, source: AssertionSource) -> OpId {
        let id = OpId(self.log.len() as u64 + 1);
        self.log.push(Op {
            id,
            kind,
            why: why.to_owned(),
            source,
        });
        self.replay();
        id
    }

    fn replay(&mut self) {
        let mut classes: BTreeMap<EntityId, BTreeSet<EntityId>> = BTreeMap::new();
        for op in self.log.iter().filter(|o| !self.undone.contains(&o.id)) {
            match op.kind {
                OpKind::Merge { a, b } => {
                    let mut set = classes
                        .get(&a)
                        .cloned()
                        .unwrap_or_else(|| BTreeSet::from([a]));
                    set.extend(
                        classes
                            .get(&b)
                            .cloned()
                            .unwrap_or_else(|| BTreeSet::from([b])),
                    );
                    for m in &set {
                        classes.insert(*m, set.clone());
                    }
                }
                OpKind::Split { entity } => {
                    if let Some(mut set) = classes.remove(&entity) {
                        set.remove(&entity);
                        if set.len() < 2 {
                            for m in &set {
                                classes.remove(m);
                            }
                        } else {
                            for m in &set {
                                classes.insert(*m, set.clone());
                            }
                        }
                    }
                }
                OpKind::Undo { .. } => {}
            }
        }
        self.classes = classes;
    }

    /// Declare `a` and `b` the same entity.
    pub fn merge(
        &mut self,
        a: EntityId,
        b: EntityId,
        why: &str,
        source: AssertionSource,
    ) -> Result<OpId, IdentityError> {
        if self.same(a, b) {
            return Err(IdentityError::AlreadySame(a, b));
        }
        Ok(self.push(OpKind::Merge { a, b }, why, source))
    }

    /// Detach `entity` from its class; the others stay together.
    pub fn split(
        &mut self,
        entity: EntityId,
        why: &str,
        source: AssertionSource,
    ) -> Result<OpId, IdentityError> {
        if !self.classes.contains_key(&entity) {
            return Err(IdentityError::NotMerged(entity));
        }
        Ok(self.push(OpKind::Split { entity }, why, source))
    }

    /// Cancel an earlier merge or split. The log keeps both; later ops are replayed on top.
    pub fn undo(
        &mut self,
        target: OpId,
        why: &str,
        source: AssertionSource,
    ) -> Result<OpId, IdentityError> {
        let op = self
            .log
            .get((target.0 as usize).wrapping_sub(1))
            .ok_or(IdentityError::UnknownOp(target))?;
        if matches!(op.kind, OpKind::Undo { .. }) {
            return Err(IdentityError::NotUndoable(target));
        }
        if self.undone.contains(&target) {
            return Err(IdentityError::AlreadyUndone(target));
        }
        self.undone.insert(target);
        Ok(self.push(OpKind::Undo { target }, why, source))
    }

    /// Suggest merges: pairs in different classes whose aliases the matcher accepts, sorted by `(a, b)`.
    pub fn propose_merges(&self, matcher: &dyn Matcher) -> Vec<MergeProposal> {
        let holders: Vec<EntityId> = self.aliases.keys().copied().collect();
        let mut out = Vec::new();
        for (i, &a) in holders.iter().enumerate() {
            for &b in &holders[i + 1..] {
                if self.same(a, b) {
                    continue;
                }
                let mut best: Option<(Match, &str, &str)> = None;
                for x in self.aliases(a) {
                    for y in self.aliases(b) {
                        if let Some(m) = matcher.compare(&x.text, &y.text)
                            && best.as_ref().is_none_or(|(bm, _, _)| m.score > bm.score)
                        {
                            best = Some((m, &x.text, &y.text));
                        }
                    }
                }
                if let Some((m, x, y)) = best {
                    out.push(MergeProposal {
                        a,
                        b,
                        score: m.score,
                        reasons: vec![Reason::new(
                            "identity.match",
                            format!("{}: {x:?} ~ {y:?} ({})", matcher.name(), m.note),
                        )],
                    });
                }
            }
        }
        out
    }

    /// Apply a proposal as a logged merge, with the proposal's reasons as the `why`.
    pub fn apply(
        &mut self,
        proposal: &MergeProposal,
        source: AssertionSource,
    ) -> Result<OpId, IdentityError> {
        let why = proposal
            .reasons
            .iter()
            .map(|r| r.note.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        self.merge(proposal.a, proposal.b, &why, source)
    }

    /// The line-oriented text form: aliases first (`alias`), then the log (`merge`, `split`, `undo`), tab separated,
    /// with `\\`, `\t` and `\n` escaped in free text. Loading replays the log.
    pub fn to_text(&self) -> String {
        let mut s = String::new();
        for (e, list) in &self.aliases {
            for a in list {
                s.push_str(&format!(
                    "alias\t{}\t{}\t{}\t{}\n",
                    e.0,
                    origin_str(a.source.origin),
                    esc(&a.source.source),
                    esc(&a.text)
                ));
            }
        }
        for op in &self.log {
            let head = match op.kind {
                OpKind::Merge { a, b } => format!("merge\t{}\t{}\t{}", op.id.0, a.0, b.0),
                OpKind::Split { entity } => format!("split\t{}\t{}", op.id.0, entity.0),
                OpKind::Undo { target } => format!("undo\t{}\t{}", op.id.0, target.0),
            };
            s.push_str(&format!(
                "{head}\t{}\t{}\t{}\n",
                origin_str(op.source.origin),
                esc(&op.source.source),
                esc(&op.why)
            ));
        }
        s
    }

    /// Read the text form written by [`IdentityIndex::to_text`], validating every op as it is replayed.
    pub fn from_text(text: &str) -> Result<IdentityIndex, IdentityError> {
        let mut idx = IdentityIndex::new();
        for (n, line) in text.lines().enumerate() {
            let perr = |problem: &str| IdentityError::Parse {
                line: n + 1,
                problem: problem.to_owned(),
            };
            if line.is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            let num = |i: usize| -> Result<u64, IdentityError> {
                f.get(i)
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| perr("bad number"))
            };
            let src_at = |i: usize| -> Result<AssertionSource, IdentityError> {
                let origin = match f.get(i).copied() {
                    Some("asserted") => Origin::Asserted,
                    Some("derived") => Origin::Derived,
                    _ => return Err(perr("bad origin")),
                };
                let source = unesc(f.get(i + 1).ok_or_else(|| perr("missing source"))?);
                Ok(AssertionSource { source, origin })
            };
            let want = |k: usize| {
                if f.len() == k {
                    Ok(())
                } else {
                    Err(perr("wrong number of fields"))
                }
            };
            match f[0] {
                "alias" => {
                    want(5)?;
                    idx.add_alias(EntityId(num(1)?), unesc(f[4]), src_at(2)?);
                }
                kind @ ("merge" | "split" | "undo") => {
                    let (nargs, k) = match kind {
                        "merge" => (2, 7),
                        _ => (1, 6),
                    };
                    want(k)?;
                    if num(1)? != idx.log.len() as u64 + 1 {
                        return Err(perr("op ids must count up from 1"));
                    }
                    let source = src_at(2 + nargs)?;
                    let why = unesc(f[4 + nargs]);
                    let r = match kind {
                        "merge" => idx.merge(EntityId(num(2)?), EntityId(num(3)?), &why, source),
                        "split" => idx.split(EntityId(num(2)?), &why, source),
                        _ => idx.undo(OpId(num(2)?), &why, source),
                    };
                    r.map_err(|e| perr(&e.to_string()))?;
                }
                _ => return Err(perr("unknown line kind")),
            }
        }
        Ok(idx)
    }
}

fn origin_str(o: Origin) -> &'static str {
    match o {
        Origin::Asserted => "asserted",
        Origin::Derived => "derived",
    }
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

fn unesc(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some(o) => out.push(o),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}
