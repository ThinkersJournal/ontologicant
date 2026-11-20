// SPDX-License-Identifier: MIT OR Apache-2.0
//! `ontologicant-alignment`: when are entities in two different graphs the same thing?
//!
//! Step 2 of the plan, deliberately small (see `docs/STEP2-ALIGNMENT-NOTE.md`): **`Equivalent` only**, over
//! `(GraphName, EntityId)` pairs. SageAnt's reading of its own cross-graph addressing is that "the same chunk in two
//! graphs" is `Equivalent` at most, and nothing downstream has asked for `Narrower`, `Broader` or `Disjoint`.
//! [`Relation`] is `#[non_exhaustive]` so they can be added without breaking callers.
//!
//! * [`propose`] compares the names (label and aliases) of two graphs with an `ontologicant_identity::Matcher` and
//!   returns [`Correspondence`] proposals with reasons. Proposing changes nothing.
//! * [`Alignment`] is an overlay: accepting a proposal is a separate, logged, undoable step, and neither graph is ever
//!   mutated. Equivalence is transitive over live (accepted, not retracted) correspondences.
//!
//! The graph-name scheme is a plain validated string here, because the addressing SageAnt will eventually fix
//! (graph id, chunk id, version, content hash) does not exist yet. [`Ref`] is the one place to change.

use ontologicant_core::{AssertionSource, EntityId, Origin, Reason};
use ontologicant_graph::Graph;
use ontologicant_identity::{IdentityIndex, Matcher};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

/// The name of a graph in an alignment. Non-empty, no control characters.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphName(String);

impl GraphName {
    /// Validate and wrap a name.
    pub fn new(name: impl Into<String>) -> Result<Self, AlignError> {
        let name = name.into();
        if name.is_empty() || name.chars().any(char::is_control) {
            return Err(AlignError::BadGraphName(name));
        }
        Ok(GraphName(name))
    }
    /// The name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for GraphName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// An entity in a named graph. Entity ids are only unique within one graph.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ref {
    /// The graph.
    pub graph: GraphName,
    /// The entity in that graph.
    pub entity: EntityId,
}

/// How two entities correspond.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Relation {
    /// The two are the same thing.
    Equivalent,
}

/// A suggested correspondence. Proposing changes nothing; see [`Alignment::accept`].
#[derive(Clone, Debug, PartialEq)]
pub struct Correspondence {
    /// The entity in the left graph.
    pub left: Ref,
    /// The entity in the right graph.
    pub right: Ref,
    /// How they correspond.
    pub relation: Relation,
    /// Best matcher score over all name pairs, in `(0, 1]`.
    pub score: f64,
    /// The evidence.
    pub reasons: Vec<Reason>,
}

/// One side of a comparison: a named graph, plus optional aliases for its entities.
#[derive(Clone, Copy, Debug)]
pub struct Side<'a> {
    /// The graph's name.
    pub name: &'a GraphName,
    /// The graph.
    pub graph: &'a Graph,
    /// Extra names for its entities, if any.
    pub identity: Option<&'a IdentityIndex>,
}

fn names<'a>(side: &Side<'a>, label: &'a str, e: EntityId) -> Vec<&'a str> {
    let mut v = vec![label];
    if let Some(ix) = side.identity {
        v.extend(ix.aliases(e).iter().map(|a| a.text.as_str()));
    }
    v
}

/// Propose `Equivalent` correspondences between the entities of two graphs, best name pair each. Both graphs'
/// names must differ (within one graph, use `ontologicant-identity`). Ordered by descending score, then by ids.
pub fn propose(
    left: Side<'_>,
    right: Side<'_>,
    matcher: &dyn Matcher,
) -> Result<Vec<Correspondence>, AlignError> {
    if left.name == right.name {
        return Err(AlignError::SameGraph(left.name.clone()));
    }
    let mut out = Vec::new();
    for (le, ll) in left.graph.entities() {
        let ln = names(&left, ll, le);
        for (re, rl) in right.graph.entities() {
            let rn = names(&right, rl, re);
            let mut best: Option<(f64, String)> = None;
            for x in &ln {
                for y in &rn {
                    if let Some(m) = matcher.compare(x, y)
                        && best.as_ref().is_none_or(|(s, _)| m.score > *s)
                    {
                        best = Some((
                            m.score,
                            format!("{}: {x:?} ~ {y:?} ({})", matcher.name(), m.note),
                        ));
                    }
                }
            }
            if let Some((score, note)) = best {
                out.push(Correspondence {
                    left: Ref {
                        graph: left.name.clone(),
                        entity: le,
                    },
                    right: Ref {
                        graph: right.name.clone(),
                        entity: re,
                    },
                    relation: Relation::Equivalent,
                    score,
                    reasons: vec![Reason::new("alignment.match", note)],
                });
            }
        }
    }
    out.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| (&a.left, &a.right).cmp(&(&b.left, &b.right)))
    });
    Ok(out)
}

/// Identifier of an accepted correspondence in the log. Never reused. Displays as `c<n>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CorrespondenceId(pub u64);

impl fmt::Display for CorrespondenceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "c{}", self.0)
    }
}

/// Why an alignment operation or a text load failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlignError {
    /// A graph name was empty or had control characters.
    BadGraphName(String),
    /// Both sides are the same graph; that is identity's job, not alignment's.
    SameGraph(GraphName),
    /// That live correspondence already exists (either direction).
    AlreadyAccepted(CorrespondenceId),
    /// No such correspondence.
    Unknown(CorrespondenceId),
    /// Already retracted.
    AlreadyRetracted(CorrespondenceId),
    /// A score was not finite or not in `(0, 1]`.
    BadScore,
    /// The text form is malformed.
    Parse {
        /// 1-based line number.
        line: usize,
        /// What was wrong.
        problem: String,
    },
}

impl fmt::Display for AlignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadGraphName(n) => write!(f, "bad graph name {n:?}"),
            Self::SameGraph(g) => {
                write!(f, "both sides are graph {g}; use identity within a graph")
            }
            Self::AlreadyAccepted(c) => write!(f, "already accepted as {c}"),
            Self::Unknown(c) => write!(f, "no such correspondence {c}"),
            Self::AlreadyRetracted(c) => write!(f, "{c} was already retracted"),
            Self::BadScore => f.write_str("score must be finite and in (0, 1]"),
            Self::Parse { line, problem } => write!(f, "line {line}: {problem}"),
        }
    }
}

impl std::error::Error for AlignError {}

/// An accepted correspondence in the log.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// Its id.
    pub id: CorrespondenceId,
    /// What was accepted.
    pub left: Ref,
    /// What was accepted.
    pub right: Ref,
    /// The score it was proposed with.
    pub score: f64,
    /// Why it was accepted (the proposal's reasons, joined).
    pub why: String,
    /// Who accepted it.
    pub source: AssertionSource,
    /// Whether it has been retracted.
    pub retracted: bool,
}

/// Accepted correspondences between graphs: an append-only log with retraction. An overlay: it never mutates a graph.
#[derive(Clone, Debug, Default)]
pub struct Alignment {
    log: Vec<Entry>,
}

impl Alignment {
    /// An empty alignment.
    pub fn new() -> Self {
        Self::default()
    }

    /// Every entry ever accepted, in order, retracted ones included.
    pub fn log(&self) -> &[Entry] {
        &self.log
    }

    fn live(&self) -> impl Iterator<Item = &Entry> {
        self.log.iter().filter(|e| !e.retracted)
    }

    /// Accept a proposal as a logged correspondence, with the proposal's reasons as the `why`.
    pub fn accept(
        &mut self,
        c: &Correspondence,
        source: AssertionSource,
    ) -> Result<CorrespondenceId, AlignError> {
        if c.left.graph == c.right.graph {
            return Err(AlignError::SameGraph(c.left.graph.clone()));
        }
        if !(c.score.is_finite() && c.score > 0.0 && c.score <= 1.0) {
            return Err(AlignError::BadScore);
        }
        if let Some(e) = self.live().find(|e| {
            (e.left == c.left && e.right == c.right) || (e.left == c.right && e.right == c.left)
        }) {
            return Err(AlignError::AlreadyAccepted(e.id));
        }
        let why = c
            .reasons
            .iter()
            .map(|r| format!("{}: {}", r.rule, r.note))
            .collect::<Vec<_>>()
            .join("; ");
        Ok(self.push(c.left.clone(), c.right.clone(), c.score, why, source))
    }

    fn push(
        &mut self,
        left: Ref,
        right: Ref,
        score: f64,
        why: String,
        source: AssertionSource,
    ) -> CorrespondenceId {
        let id = CorrespondenceId(self.log.len() as u64);
        self.log.push(Entry {
            id,
            left,
            right,
            score,
            why,
            source,
            retracted: false,
        });
        id
    }

    /// Retract an accepted correspondence. The log keeps it, marked retracted.
    pub fn retract(&mut self, id: CorrespondenceId) -> Result<(), AlignError> {
        let e = self
            .log
            .get_mut(id.0 as usize)
            .ok_or(AlignError::Unknown(id))?;
        if e.retracted {
            return Err(AlignError::AlreadyRetracted(id));
        }
        e.retracted = true;
        Ok(())
    }

    /// Everything equivalent to `r` through live correspondences (transitively), excluding `r` itself, sorted.
    pub fn equivalents(&self, r: &Ref) -> Vec<Ref> {
        let mut adj: BTreeMap<&Ref, Vec<&Ref>> = BTreeMap::new();
        for e in self.live() {
            adj.entry(&e.left).or_default().push(&e.right);
            adj.entry(&e.right).or_default().push(&e.left);
        }
        let mut seen: BTreeSet<&Ref> = BTreeSet::from([r]);
        let mut q = VecDeque::from([r]);
        while let Some(x) = q.pop_front() {
            for &n in adj.get(x).into_iter().flatten() {
                if seen.insert(n) {
                    q.push_back(n);
                }
            }
        }
        seen.into_iter().filter(|x| *x != r).cloned().collect()
    }

    /// Whether `a` and `b` are the same thing through live correspondences.
    pub fn equivalent(&self, a: &Ref, b: &Ref) -> bool {
        a == b || self.equivalents(a).contains(b)
    }

    /// The line-oriented text form, one `accept` line per log entry in log order (tab separated, `\\`, `\t`, `\n`
    /// escaped in free text), ending in `live` or `retracted`. Loading replays it.
    pub fn to_text(&self) -> String {
        let mut s = String::new();
        for e in &self.log {
            s.push_str(&format!(
                "accept\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                e.id.0,
                esc(e.left.graph.as_str()),
                e.left.entity.0,
                esc(e.right.graph.as_str()),
                e.right.entity.0,
                e.score,
                if e.source.origin == Origin::Asserted {
                    "asserted"
                } else {
                    "derived"
                },
                esc(&e.source.source),
                esc(&e.why),
                if e.retracted { "retracted" } else { "live" },
            ));
        }
        s
    }

    /// Read the text form written by [`Alignment::to_text`], validating as it replays.
    pub fn from_text(text: &str) -> Result<Alignment, AlignError> {
        let mut out = Alignment::new();
        for (n, line) in text.lines().enumerate() {
            let perr = |p: &str| AlignError::Parse {
                line: n + 1,
                problem: p.to_owned(),
            };
            if line.is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            match f.as_slice() {
                [
                    "accept",
                    id,
                    lg,
                    le,
                    rg,
                    re,
                    score,
                    origin,
                    source,
                    why,
                    state,
                ] => {
                    let id: u64 = id.parse().map_err(|_| perr("bad id"))?;
                    if id != out.log.len() as u64 {
                        return Err(perr("ids must be consecutive from 0"));
                    }
                    let ent = |s: &str| {
                        s.parse::<u64>()
                            .map(EntityId)
                            .map_err(|_| perr("bad entity id"))
                    };
                    let left = Ref {
                        graph: GraphName::new(unesc(lg))?,
                        entity: ent(le)?,
                    };
                    let right = Ref {
                        graph: GraphName::new(unesc(rg))?,
                        entity: ent(re)?,
                    };
                    let score: f64 = score.parse().map_err(|_| perr("bad score"))?;
                    let source = match *origin {
                        "asserted" => AssertionSource::asserted(unesc(source)),
                        "derived" => AssertionSource::derived(unesc(source)),
                        _ => return Err(perr("origin must be asserted or derived")),
                    };
                    let c = Correspondence {
                        left,
                        right,
                        relation: Relation::Equivalent,
                        score,
                        reasons: vec![],
                    };
                    let retracted = match *state {
                        "live" => false,
                        "retracted" => true,
                        _ => return Err(perr("state must be live or retracted")),
                    };
                    // A retracted entry is history, so it may repeat a pair; a live one is checked like `accept`.
                    let got = if retracted {
                        if c.left.graph == c.right.graph {
                            return Err(AlignError::SameGraph(c.left.graph));
                        }
                        if !(c.score.is_finite() && c.score > 0.0 && c.score <= 1.0) {
                            return Err(AlignError::BadScore);
                        }
                        let id = out.push(c.left, c.right, c.score, unesc(why), source);
                        out.retract(id)?;
                        id
                    } else {
                        let id = out.accept(&c, source)?;
                        out.log[id.0 as usize].why = unesc(why);
                        id
                    };
                    debug_assert_eq!(got.0, out.log.len() as u64 - 1);
                }
                _ => return Err(perr("unrecognised line")),
            }
        }
        Ok(out)
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
