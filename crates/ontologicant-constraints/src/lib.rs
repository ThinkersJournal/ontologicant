// SPDX-License-Identifier: MIT OR Apache-2.0
//! `ontologicant-constraints`: what must hold of a graph, checked with reasons.
//!
//! A [`ConstraintSet`] is **data**: a list of [`Constraint`]s that can be built in code, taken from a kinds
//! [`Vocabulary`] ([`ConstraintSet::from_vocabulary`]) or read from a line-oriented text form. [`ConstraintSet::check`]
//! runs them over a [`Graph`] and its is-a [`Taxonomy`] and returns a [`Report`]; every [`Violation`] names the
//! constraint, the entities and the reasons. Nothing is repaired and nothing is checked that was not asked for.
//!
//! "Of kind `k`" means: the entity is `k` or is below `k` in the taxonomy. Cardinalities count relations, so two parallel
//! relations count twice; functional and inverse-functional count *distinct* ends, so a repeated assertion is not a clash.
#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

use ontologicant_graph::{EntityId, Graph, Relation, RelationType};
use ontologicant_kinds::{Cycle, Property, Taxonomy, Vocabulary};

pub use ontologicant_core::Reason;

/// One declared requirement.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "json", serde(rename_all = "kebab-case"))]
pub enum Constraint {
    /// No entity is of both kinds `a` and `b`.
    Disjoint {
        /// First kind.
        a: EntityId,
        /// Second kind.
        b: EntityId,
    },
    /// Each entity (of kind `of`, or every entity if `None`) has between `min` and `max` (inclusive; `None` is unbounded)
    /// outgoing relations of `rtype`.
    Cardinality {
        /// The relation type counted.
        rtype: RelationType,
        /// Fewest allowed.
        min: u32,
        /// Most allowed, if bounded.
        max: Option<u32>,
        /// Restrict to instances of this kind.
        of: Option<EntityId>,
    },
    /// Every relation of `rtype` starts at an entity of kind `kind`.
    Domain {
        /// The relation type.
        rtype: RelationType,
        /// The required kind of the source.
        kind: EntityId,
    },
    /// Every relation of `rtype` ends at an entity of kind `kind`.
    Range {
        /// The relation type.
        rtype: RelationType,
        /// The required kind of the target.
        kind: EntityId,
    },
    /// Each entity has at most one distinct target of `rtype`.
    Functional(RelationType),
    /// Each entity has at most one distinct source of `rtype`.
    InverseFunctional(RelationType),
    /// No entity is related to itself by `rtype`.
    Irreflexive(RelationType),
}

/// A constraint that does not hold, with the evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct Violation {
    /// The constraint broken.
    pub constraint: Constraint,
    /// The entities involved, the offender first.
    pub entities: Vec<EntityId>,
    /// Why, in `constraints.*` rules.
    pub reasons: Vec<Reason>,
}

/// The result of a check.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Report {
    /// Violations ordered by constraint (in the order added), then by entity id.
    pub violations: Vec<Violation>,
}

impl Report {
    /// True when nothing was violated.
    pub fn is_consistent(&self) -> bool {
        self.violations.is_empty()
    }
}

/// A text form could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// 1-based line number.
    pub line: usize,
    /// What was wrong.
    pub problem: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.problem)
    }
}
impl std::error::Error for ParseError {}

/// A list of constraints.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConstraintSet {
    constraints: Vec<Constraint>,
}

type Found = Vec<(Vec<EntityId>, Reason)>;

impl ConstraintSet {
    /// An empty list (everything is consistent).
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one constraint.
    pub fn add(&mut self, c: Constraint) -> &mut Self {
        self.constraints.push(c);
        self
    }

    /// The constraints, in the order added.
    pub fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }

    /// The constraints a kinds [`Vocabulary`] declares: functional, inverse-functional and irreflexive relation types.
    /// (Transitive, symmetric and reflexive are closures, not constraints.)
    pub fn from_vocabulary(vocab: &Vocabulary) -> Self {
        let mut out = Self::new();
        for (rt, decl) in vocab.decls() {
            for p in &decl.properties {
                match p {
                    Property::Functional => {
                        out.add(Constraint::Functional(rt.clone()));
                    }
                    Property::InverseFunctional => {
                        out.add(Constraint::InverseFunctional(rt.clone()));
                    }
                    Property::Irreflexive => {
                        out.add(Constraint::Irreflexive(rt.clone()));
                    }
                    _ => {}
                }
            }
        }
        out
    }

    /// Check every constraint. `isa` names the taxonomy relation; a cyclic taxonomy is an error, not a report.
    pub fn check(&self, graph: &Graph, isa: &RelationType) -> Result<Report, Cycle> {
        let tax = Taxonomy::from_graph(graph, isa)?;
        let of_kind = |e: EntityId, k: EntityId| e == k || tax.ancestors(e).contains(&k);
        let mut violations = Vec::new();
        for c in &self.constraints {
            let mut found: Found = match c {
                Constraint::Disjoint { a, b } => disjoint(graph, &of_kind, *a, *b),
                Constraint::Cardinality {
                    rtype,
                    min,
                    max,
                    of,
                } => cardinality(graph, &of_kind, rtype, *min, *max, *of),
                Constraint::Domain { rtype, kind } => graph
                    .relations_of_type(rtype)
                    .filter(|r| !of_kind(r.from, *kind))
                    .map(|r| {
                        (
                            vec![r.from],
                            end_reason("constraints.domain", "source", r, r.from, *kind),
                        )
                    })
                    .collect(),
                Constraint::Range { rtype, kind } => graph
                    .relations_of_type(rtype)
                    .filter(|r| !of_kind(r.to, *kind))
                    .map(|r| {
                        (
                            vec![r.to],
                            end_reason("constraints.range", "target", r, r.to, *kind),
                        )
                    })
                    .collect(),
                Constraint::Functional(rtype) => distinct_ends(graph, rtype, true),
                Constraint::InverseFunctional(rtype) => distinct_ends(graph, rtype, false),
                Constraint::Irreflexive(rtype) => graph
                    .relations_of_type(rtype)
                    .filter(|r| r.from == r.to)
                    .map(|r| r.from)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .map(|e| {
                        (
                            vec![e],
                            Reason::new(
                                "constraints.irreflexive",
                                format!(
                                    "{e} is related to itself by irreflexive {:?}",
                                    rtype.as_str()
                                ),
                            ),
                        )
                    })
                    .collect(),
            };
            found.sort_by(|x, y| x.0.cmp(&y.0));
            for (entities, reason) in found {
                violations.push(Violation {
                    constraint: c.clone(),
                    entities,
                    reasons: vec![reason],
                });
            }
        }
        Ok(Report { violations })
    }

    /// The text form, one constraint per line, tab separated: `disjoint a b`, `cardinality rtype min max|* of|*`,
    /// `domain rtype kind`, `range rtype kind`, `functional rtype`, `inverse-functional rtype`, `irreflexive rtype`.
    /// Entities are written `e<n>`; backslash, tab and newline are escaped in relation names.
    pub fn to_text(&self) -> String {
        let mut s = String::new();
        for c in &self.constraints {
            let line = match c {
                Constraint::Disjoint { a, b } => format!("disjoint\t{a}\t{b}"),
                Constraint::Cardinality {
                    rtype,
                    min,
                    max,
                    of,
                } => format!(
                    "cardinality\t{}\t{min}\t{}\t{}",
                    esc(rtype.as_str()),
                    max.map_or("*".to_owned(), |m| m.to_string()),
                    of.map_or("*".to_owned(), |k| k.to_string())
                ),
                Constraint::Domain { rtype, kind } => {
                    format!("domain\t{}\t{kind}", esc(rtype.as_str()))
                }
                Constraint::Range { rtype, kind } => {
                    format!("range\t{}\t{kind}", esc(rtype.as_str()))
                }
                Constraint::Functional(r) => format!("functional\t{}", esc(r.as_str())),
                Constraint::InverseFunctional(r) => {
                    format!("inverse-functional\t{}", esc(r.as_str()))
                }
                Constraint::Irreflexive(r) => format!("irreflexive\t{}", esc(r.as_str())),
            };
            s.push_str(&line);
            s.push('\n');
        }
        s
    }

    /// Read the text form written by [`ConstraintSet::to_text`].
    pub fn from_text(text: &str) -> Result<ConstraintSet, ParseError> {
        let mut out = ConstraintSet::new();
        for (n, line) in text.lines().enumerate() {
            let perr = |p: &str| ParseError {
                line: n + 1,
                problem: p.to_owned(),
            };
            if line.is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            let want = |k: usize| {
                if f.len() == k {
                    Ok(())
                } else {
                    Err(perr("wrong number of fields"))
                }
            };
            let ent = |i: usize| -> Result<EntityId, ParseError> {
                f.get(i)
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| perr("bad entity id"))
            };
            let rt = |i: usize| RelationType::new(unesc(f.get(i).copied().unwrap_or("")));
            let c = match f[0] {
                "disjoint" => {
                    want(3)?;
                    Constraint::Disjoint {
                        a: ent(1)?,
                        b: ent(2)?,
                    }
                }
                "cardinality" => {
                    want(5)?;
                    let min = f[2].parse().map_err(|_| perr("bad minimum"))?;
                    let max = match f[3] {
                        "*" => None,
                        v => Some(v.parse().map_err(|_| perr("bad maximum"))?),
                    };
                    let of = if f[4] == "*" { None } else { Some(ent(4)?) };
                    Constraint::Cardinality {
                        rtype: rt(1),
                        min,
                        max,
                        of,
                    }
                }
                "domain" => {
                    want(3)?;
                    Constraint::Domain {
                        rtype: rt(1),
                        kind: ent(2)?,
                    }
                }
                "range" => {
                    want(3)?;
                    Constraint::Range {
                        rtype: rt(1),
                        kind: ent(2)?,
                    }
                }
                "functional" => {
                    want(2)?;
                    Constraint::Functional(rt(1))
                }
                "inverse-functional" => {
                    want(2)?;
                    Constraint::InverseFunctional(rt(1))
                }
                "irreflexive" => {
                    want(2)?;
                    Constraint::Irreflexive(rt(1))
                }
                _ => return Err(perr("unknown constraint")),
            };
            out.add(c);
        }
        Ok(out)
    }
}

#[cfg(feature = "json")]
#[derive(serde::Serialize, serde::Deserialize)]
struct JsonSet {
    version: u32,
    constraints: Vec<Constraint>,
}

#[cfg(feature = "json")]
impl ConstraintSet {
    /// Serialise to JSON: `{"version":1,"constraints":[...]}`, each constraint externally tagged by its kebab-case kind
    /// (`{"disjoint":{"a":1,"b":2}}`, `{"functional":"rel"}`, `{"cardinality":{"rtype","min","max","of"}}`).
    pub fn to_json(&self) -> String {
        let doc = JsonSet {
            version: 1,
            constraints: self.constraints.clone(),
        };
        serde_json::to_string(&doc).expect("constraints always serialise")
    }

    /// Read the JSON form written by [`ConstraintSet::to_json`].
    pub fn from_json(json: &str) -> Result<ConstraintSet, ParseError> {
        let bad = |m: String| ParseError {
            line: 0,
            problem: m,
        };
        let doc: JsonSet = serde_json::from_str(json).map_err(|e| bad(e.to_string()))?;
        if doc.version != 1 {
            return Err(bad("unsupported version".into()));
        }
        Ok(ConstraintSet {
            constraints: doc.constraints,
        })
    }
}

fn disjoint(
    graph: &Graph,
    of_kind: &dyn Fn(EntityId, EntityId) -> bool,
    a: EntityId,
    b: EntityId,
) -> Found {
    graph
        .entities()
        .map(|(e, _)| e)
        .filter(|&e| of_kind(e, a) && of_kind(e, b))
        .map(|e| {
            (
                vec![e, a, b],
                Reason::new(
                    "constraints.disjoint",
                    format!("{e} is of kind {a} and of kind {b}, which are declared disjoint"),
                ),
            )
        })
        .collect()
}

fn cardinality(
    graph: &Graph,
    of_kind: &dyn Fn(EntityId, EntityId) -> bool,
    rtype: &RelationType,
    min: u32,
    max: Option<u32>,
    of: Option<EntityId>,
) -> Found {
    let mut found = Vec::new();
    for (e, _) in graph.entities() {
        if of.is_some_and(|k| !of_kind(e, k)) {
            continue;
        }
        let n = graph
            .relations_of_type(rtype)
            .filter(|r| r.from == e)
            .count() as u64;
        if n < u64::from(min) || max.is_some_and(|m| n > u64::from(m)) {
            let want = match max {
                Some(m) => format!("{min}..={m}"),
                None => format!("{min}.."),
            };
            found.push((
                vec![e],
                Reason::new(
                    "constraints.cardinality",
                    format!(
                        "{e} has {n} {:?} relation(s), allowed {want}",
                        rtype.as_str()
                    ),
                ),
            ));
        }
    }
    found
}

/// Entities with more than one distinct target (`forward`) or source (`!forward`) for `rtype`.
fn distinct_ends(graph: &Graph, rtype: &RelationType, forward: bool) -> Found {
    let mut found = Vec::new();
    for (e, _) in graph.entities() {
        let ends: BTreeSet<EntityId> = graph
            .relations_of_type(rtype)
            .filter_map(|r| {
                if forward {
                    (r.from == e).then_some(r.to)
                } else {
                    (r.to == e).then_some(r.from)
                }
            })
            .collect();
        if ends.len() > 1 {
            let mut ents = vec![e];
            ents.extend(ends.iter().copied());
            let (rule, what) = if forward {
                ("constraints.functional", "targets for functional")
            } else {
                (
                    "constraints.inverse-functional",
                    "sources for inverse-functional",
                )
            };
            found.push((
                ents,
                Reason::new(
                    rule,
                    format!(
                        "{e} has {} distinct {what} {:?}",
                        ends.len(),
                        rtype.as_str()
                    ),
                ),
            ));
        }
    }
    found
}

fn end_reason(rule: &'static str, end: &str, r: &Relation, e: EntityId, kind: EntityId) -> Reason {
    Reason::new(
        rule,
        format!(
            "{e}, the {end} of {} ({:?}), is not of kind {kind}",
            r.id,
            r.rtype.as_str()
        ),
    )
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
