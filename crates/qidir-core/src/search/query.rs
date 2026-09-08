//! Query language parser and tantivy query builder.
//!
//! Syntax (whitespace separated, case-insensitive):
//!
//! | example                 | meaning                                                   |
//! |-------------------------|-----------------------------------------------------------|
//! | `договор аренды`        | both words (any form in Smart mode) anywhere in the text   |
//! | `"договор аренды"`      | exact phrase (word forms still normalized in Smart mode)   |
//! | `договор OR контракт`   | either word                                                |
//! | `договор -аренды`       | exclude documents containing the second word               |
//! | `NOT аренды`            | same as `-аренды`                                          |
//! | `догов*`                | prefix                                                     |
//! | `д*говор`, `дог?вор`    | wildcard                                                   |
//! | `договор~`, `договор~2` | fuzzy (typo tolerant, Levenshtein distance 1 or 2)         |
//! | `name:отчет`            | file name contains the word                                |
//! | `ext:pdf`               | file extension                                             |
//! | `path:2023`             | folder path contains the word                              |
//! | `content:слово`         | only in the text, not in the file name                     |

use std::collections::HashSet;
use std::ops::Bound;

use tantivy::query::{
    BooleanQuery, BoostQuery, FuzzyTermQuery, Occur, PhraseQuery, Query, RangeQuery, RegexQuery,
    TermQuery,
};
use tantivy::schema::{Field, IndexRecordOption};
use tantivy::Term;

use super::highlight::Matcher;
use super::SearchMode;
use crate::analysis::{analyze, AnalysisMode};
use crate::error::{Error, Result};
use crate::index::Fields;

/// Which fields a clause targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldSel {
    /// Content + file name (default).
    Default,
    Content,
    Name,
    Path,
    Ext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClauseKind {
    Term,
    Phrase,
    Prefix,
    Wildcard,
    Fuzzy(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause {
    pub text: String,
    pub field: FieldSel,
    pub kind: ClauseKind,
}

/// Parsed query: a conjunction of disjunction groups plus exclusions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedQuery {
    pub groups: Vec<Vec<Clause>>,
    pub must_not: Vec<Clause>,
}

impl ParsedQuery {
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty() && self.must_not.is_empty()
    }

    /// Human-readable description shown in the UI.
    pub fn describe(&self) -> String {
        let fmt = |c: &Clause| -> String {
            let field = match c.field {
                FieldSel::Default => "",
                FieldSel::Content => "content:",
                FieldSel::Name => "name:",
                FieldSel::Path => "path:",
                FieldSel::Ext => "ext:",
            };
            match &c.kind {
                ClauseKind::Term => format!("{field}{}", c.text),
                ClauseKind::Phrase => format!("{field}\"{}\"", c.text),
                ClauseKind::Prefix => format!("{field}{}*", c.text),
                ClauseKind::Wildcard => format!("{field}{}", c.text),
                ClauseKind::Fuzzy(d) => format!("{field}{}~{d}", c.text),
            }
        };
        let mut parts: Vec<String> = self
            .groups
            .iter()
            .map(|g| {
                let alts: Vec<String> = g.iter().map(fmt).collect();
                if alts.len() > 1 {
                    format!("({})", alts.join(" OR "))
                } else {
                    alts.join("")
                }
            })
            .collect();
        for c in &self.must_not {
            parts.push(format!("-{}", fmt(c)));
        }
        parts.join(" AND ")
    }
}

/// Split the raw query into tokens, honouring double quotes.
fn split_tokens(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    for c in input.chars() {
        match c {
            '"' | '«' | '»' | '“' | '”' => {
                if in_quotes {
                    cur.push('"');
                    out.push(std::mem::take(&mut cur));
                    in_quotes = false;
                } else {
                    if !cur.is_empty()
                        && !cur.ends_with(':')
                        && !cur.ends_with('-')
                        && !cur.ends_with('!')
                    {
                        out.push(std::mem::take(&mut cur));
                    }
                    cur.push('"');
                    in_quotes = true;
                }
            }
            c if c.is_whitespace() && !in_quotes => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        if in_quotes {
            cur.push('"');
        }
        out.push(cur);
    }
    out
}

fn parse_clause(raw: &str) -> Option<Clause> {
    let mut s = raw;
    let mut field = FieldSel::Default;
    // field prefix
    if let Some(idx) = s.find(':') {
        if !s.starts_with('"') {
            let (f, rest) = s.split_at(idx);
            let sel = match f.to_ascii_lowercase().as_str() {
                "name" | "имя" | "файл" | "file" | "атау" => Some(FieldSel::Name),
                "ext" | "расш" | "тип" | "type" => Some(FieldSel::Ext),
                "path" | "путь" | "folder" | "папка" | "жол" => Some(FieldSel::Path),
                "content" | "text" | "текст" | "мәтін" | "body" => {
                    Some(FieldSel::Content)
                }
                _ => None,
            };
            if let Some(sel) = sel {
                field = sel;
                s = &rest[1..];
            }
        }
    }
    if s.is_empty() || !s.chars().any(|c| c.is_alphanumeric()) {
        return None;
    }
    if s.starts_with('"') {
        let inner = s.trim_matches('"').trim();
        if inner.is_empty() {
            return None;
        }
        return Some(Clause {
            text: inner.to_string(),
            field,
            kind: ClauseKind::Phrase,
        });
    }
    // fuzzy suffix
    if let Some(pos) = s.rfind('~') {
        let (base, dist) = s.split_at(pos);
        let dist = &dist[1..];
        let d: Option<u8> = if dist.is_empty() {
            Some(1)
        } else {
            dist.parse().ok()
        };
        if let (false, Some(d)) = (base.is_empty(), d) {
            return Some(Clause {
                text: base.to_string(),
                field,
                kind: ClauseKind::Fuzzy(d.clamp(1, 2)),
            });
        }
    }
    let trimmed = s.trim_end_matches('*');
    if s.ends_with('*') && !trimmed.contains(['*', '?']) {
        if trimmed.is_empty() {
            return None;
        }
        return Some(Clause {
            text: trimmed.to_string(),
            field,
            kind: ClauseKind::Prefix,
        });
    }
    if s.contains(['*', '?']) {
        return Some(Clause {
            text: s.to_string(),
            field,
            kind: ClauseKind::Wildcard,
        });
    }
    Some(Clause {
        text: s.to_string(),
        field,
        kind: ClauseKind::Term,
    })
}

/// Parse user input into a [`ParsedQuery`]. Never fails: unparseable pieces
/// are treated as plain words.
pub fn parse(input: &str) -> ParsedQuery {
    let tokens = split_tokens(input.trim());
    let mut q = ParsedQuery::default();
    let mut negate_next = false;
    let mut or_pending = false;

    for tok in tokens {
        let upper = tok.to_ascii_uppercase();
        if upper == "OR" || tok == "|" || upper == "ИЛИ" || upper == "НЕМЕСЕ" {
            or_pending = !q.groups.is_empty();
            continue;
        }
        if upper == "AND" || tok == "&&" || upper == "И" || upper == "ЖӘНЕ" {
            continue;
        }
        if upper == "NOT" || tok == "!" || upper == "НЕ" || upper == "ЕМЕС" {
            negate_next = true;
            continue;
        }
        let mut raw = tok.as_str();
        let mut negated = negate_next;
        negate_next = false;
        if (raw.starts_with('-') || raw.starts_with('!')) && raw.len() > 1 {
            negated = true;
            raw = &raw[1..];
        }
        let Some(clause) = parse_clause(raw) else {
            or_pending = false;
            continue;
        };
        if negated {
            q.must_not.push(clause);
            or_pending = false;
        } else if or_pending {
            q.groups.last_mut().unwrap().push(clause);
            or_pending = false;
        } else {
            q.groups.push(vec![clause]);
        }
    }
    q
}

/// Builds tantivy queries from a [`ParsedQuery`].
pub struct QueryBuilder {
    fields: Fields,
    mode: SearchMode,
}

const NAME_BOOST: f32 = 3.0;

impl QueryBuilder {
    pub fn new(fields: Fields, mode: SearchMode) -> Self {
        Self { fields, mode }
    }

    pub fn parse(&self, input: &str) -> ParsedQuery {
        parse(input)
    }

    fn content_field(&self) -> Field {
        match self.mode {
            SearchMode::Smart => self.fields.content,
            SearchMode::Exact => self.fields.content_exact,
        }
    }

    /// Fields + analysis mode targeted by a clause. Prefix/wildcard/fuzzy
    /// always operate on exact word forms.
    fn targets(&self, clause: &Clause) -> Vec<(Field, AnalysisMode, f32)> {
        let exactish = !matches!(clause.kind, ClauseKind::Term | ClauseKind::Phrase);
        let content = if exactish {
            (self.fields.content_exact, AnalysisMode::Exact, 1.0)
        } else {
            (self.content_field(), self.mode.analysis(), 1.0)
        };
        match clause.field {
            FieldSel::Default => vec![content, (self.fields.name, AnalysisMode::Exact, NAME_BOOST)],
            FieldSel::Content => vec![content],
            FieldSel::Name => vec![(self.fields.name, AnalysisMode::Exact, 1.0)],
            FieldSel::Path => vec![(self.fields.path_text, AnalysisMode::Exact, 1.0)],
            FieldSel::Ext => vec![(self.fields.ext, AnalysisMode::Exact, 1.0)],
        }
    }

    fn clause_query(&self, clause: &Clause) -> Result<Option<Box<dyn Query>>> {
        if clause.field == FieldSel::Ext {
            let ext = clause.text.trim_start_matches('.').to_lowercase();
            if ext.is_empty() {
                return Ok(None);
            }
            return Ok(Some(Box::new(TermQuery::new(
                Term::from_field_text(self.fields.ext, &ext),
                IndexRecordOption::Basic,
            ))));
        }
        let mut alts: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for (field, amode, boost) in self.targets(clause) {
            let q: Option<Box<dyn Query>> = match &clause.kind {
                ClauseKind::Term | ClauseKind::Phrase => {
                    let positions = group_by_position(&clause.text, amode);
                    positional_query(field, &positions)
                }
                ClauseKind::Prefix => {
                    let toks = analyze(&clause.text, AnalysisMode::Exact);
                    toks.last().map(|t| {
                        let pattern = format!("{}.*", regex::escape(&t.term));
                        let q: Box<dyn Query> = match RegexQuery::from_pattern(&pattern, field) {
                            Ok(q) => Box::new(q),
                            Err(_) => Box::new(TermQuery::new(
                                Term::from_field_text(field, &t.term),
                                IndexRecordOption::Basic,
                            )),
                        };
                        q
                    })
                }
                ClauseKind::Wildcard => {
                    let pattern = wildcard_to_regex(&clause.text);
                    match RegexQuery::from_pattern(&pattern, field) {
                        Ok(q) => Some(Box::new(q) as Box<dyn Query>),
                        Err(e) => {
                            return Err(Error::Query(format!(
                                "bad wildcard `{}`: {e}",
                                clause.text
                            )))
                        }
                    }
                }
                ClauseKind::Fuzzy(d) => {
                    let toks = analyze(&clause.text, AnalysisMode::Exact);
                    toks.first().map(|t| {
                        Box::new(FuzzyTermQuery::new(
                            Term::from_field_text(field, &t.term),
                            *d,
                            true,
                        )) as Box<dyn Query>
                    })
                }
            };
            if let Some(q) = q {
                let boosted: Box<dyn Query> = if (boost - 1.0).abs() > f32::EPSILON {
                    Box::new(BoostQuery::new(q, boost))
                } else {
                    q
                };
                alts.push((Occur::Should, boosted));
            }
        }
        Ok(match alts.len() {
            0 => None,
            1 => Some(alts.pop().unwrap().1),
            _ => Some(Box::new(BooleanQuery::new(alts))),
        })
    }

    /// Build the text part of the query. `None` when the query has no
    /// positive clauses (caller decides: match-all or nothing).
    pub fn build(&self, parsed: &ParsedQuery) -> Result<Option<Box<dyn Query>>> {
        let mut clauses: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for group in &parsed.groups {
            let mut alts: Vec<(Occur, Box<dyn Query>)> = Vec::new();
            for c in group {
                if let Some(q) = self.clause_query(c)? {
                    alts.push((Occur::Should, q));
                }
            }
            match alts.len() {
                0 => {}
                1 => clauses.push((Occur::Must, alts.pop().unwrap().1)),
                _ => clauses.push((Occur::Must, Box::new(BooleanQuery::new(alts)))),
            }
        }
        let has_positive = !clauses.is_empty();
        for c in &parsed.must_not {
            if let Some(q) = self.clause_query(c)? {
                clauses.push((Occur::MustNot, q));
            }
        }
        if clauses.is_empty() {
            return Ok(None);
        }
        if !has_positive {
            // Pure exclusion: match everything except.
            clauses.push((Occur::Must, Box::new(tantivy::query::AllQuery)));
        }
        Ok(Some(Box::new(BooleanQuery::new(clauses))))
    }

    /// Build the highlighter for positive clauses that target content.
    pub fn matcher(&self, parsed: &ParsedQuery) -> Matcher {
        let mut m = Matcher::default();
        let amode = self.mode.analysis();
        for clause in parsed.groups.iter().flatten() {
            if !matches!(clause.field, FieldSel::Default | FieldSel::Content) {
                continue;
            }
            match &clause.kind {
                ClauseKind::Term | ClauseKind::Phrase => {
                    for t in analyze(&clause.text, amode) {
                        m.terms.insert(t.term);
                    }
                }
                ClauseKind::Prefix => {
                    if let Some(t) = analyze(&clause.text, AnalysisMode::Exact).last() {
                        m.prefixes.push(t.term.clone());
                    }
                }
                ClauseKind::Wildcard => {
                    if let Ok(re) =
                        regex::Regex::new(&format!("^{}$", wildcard_to_regex(&clause.text)))
                    {
                        m.regexes.push(re);
                    }
                }
                ClauseKind::Fuzzy(d) => {
                    if let Some(t) = analyze(&clause.text, AnalysisMode::Exact).first() {
                        m.fuzzy.push((t.term.clone(), *d));
                    }
                }
            }
        }
        m
    }
}

/// Analyze text and group the resulting terms by position (stem + exact form
/// share a position in Stem mode).
fn group_by_position(text: &str, mode: AnalysisMode) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut last = usize::MAX;
    for t in analyze(text, mode) {
        if t.position != last {
            out.push(Vec::new());
            last = t.position;
        }
        out.last_mut().unwrap().push(t.term);
    }
    out
}

/// Maximum number of phrase variants generated from per-position alternatives.
const MAX_PHRASE_VARIANTS: usize = 64;

/// Build a query for a sequence of positions, each with alternative terms.
///
/// * one position → `Should` of term queries (or a single term query);
/// * several positions → the cartesian product of alternatives as phrase
///   queries combined with `Should` (capped at [`MAX_PHRASE_VARIANTS`]; beyond
///   the cap only the primary-stem and the exact-form phrases are used).
fn positional_query(field: Field, positions: &[Vec<String>]) -> Option<Box<dyn Query>> {
    match positions.len() {
        0 => None,
        1 => {
            let alts: Vec<(Occur, Box<dyn Query>)> = positions[0]
                .iter()
                .map(|t| {
                    let q: Box<dyn Query> = Box::new(TermQuery::new(
                        Term::from_field_text(field, t),
                        IndexRecordOption::WithFreqsAndPositions,
                    ));
                    (Occur::Should, q)
                })
                .collect();
            if alts.len() == 1 {
                Some(alts.into_iter().next().unwrap().1)
            } else {
                Some(Box::new(BooleanQuery::new(alts)))
            }
        }
        _ => {
            let total: usize = positions.iter().map(|p| p.len().max(1)).product();
            let combos: Vec<Vec<&str>> = if total > MAX_PHRASE_VARIANTS {
                // Too many alternatives: primary stems + exact forms only.
                let stems: Vec<&str> = positions.iter().map(|p| p[0].as_str()).collect();
                let exact: Vec<&str> = positions
                    .iter()
                    .map(|p| p.last().unwrap().as_str())
                    .collect();
                if stems == exact {
                    vec![stems]
                } else {
                    vec![stems, exact]
                }
            } else {
                let mut combos: Vec<Vec<&str>> = vec![Vec::new()];
                for alts in positions {
                    let mut next = Vec::with_capacity(combos.len() * alts.len());
                    for c in &combos {
                        for a in alts {
                            let mut c2 = c.clone();
                            c2.push(a.as_str());
                            next.push(c2);
                        }
                    }
                    combos = next;
                }
                combos
            };
            let alts: Vec<(Occur, Box<dyn Query>)> = combos
                .into_iter()
                .map(|c| {
                    let terms: Vec<Term> =
                        c.iter().map(|t| Term::from_field_text(field, t)).collect();
                    let q: Box<dyn Query> = Box::new(PhraseQuery::new(terms));
                    (Occur::Should, q)
                })
                .collect();
            if alts.len() == 1 {
                Some(alts.into_iter().next().unwrap().1)
            } else {
                Some(Box::new(BooleanQuery::new(alts)))
            }
        }
    }
}

/// Convert `*`/`?` wildcards to a regex over a normalized (exact-mode) term.
fn wildcard_to_regex(pattern: &str) -> String {
    let lower = pattern.to_lowercase();
    let folded = crate::analysis::fold::fold_token(&lower);
    let mut out = String::new();
    for c in folded.chars() {
        match c {
            '*' => out.push_str(".*"),
            '?' => out.push('.'),
            c => out.push_str(&regex::escape(&c.to_string())),
        }
    }
    out
}

pub(crate) fn u64_range(field: Field, min: Option<u64>, max: Option<u64>) -> Box<dyn Query> {
    let lower = match min {
        Some(v) => Bound::Included(Term::from_field_u64(field, v)),
        None => Bound::Unbounded,
    };
    let upper = match max {
        Some(v) => Bound::Included(Term::from_field_u64(field, v)),
        None => Bound::Unbounded,
    };
    Box::new(RangeQuery::new(lower, upper))
}

pub(crate) fn i64_range(field: Field, min: Option<i64>, max: Option<i64>) -> Box<dyn Query> {
    let lower = match min {
        Some(v) => Bound::Included(Term::from_field_i64(field, v)),
        None => Bound::Unbounded,
    };
    let upper = match max {
        Some(v) => Bound::Included(Term::from_field_i64(field, v)),
        None => Bound::Unbounded,
    };
    Box::new(RangeQuery::new(lower, upper))
}

/// Set of distinct analyzed terms in a query (for tests and diagnostics).
pub fn query_terms(input: &str, mode: AnalysisMode) -> HashSet<String> {
    parse(input)
        .groups
        .iter()
        .flatten()
        .flat_map(|c| analyze(&c.text, mode))
        .map(|t| t.term)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(text: &str, kind: ClauseKind) -> Clause {
        Clause {
            text: text.into(),
            field: FieldSel::Default,
            kind,
        }
    }

    #[test]
    fn parses_words_phrases_and_operators() {
        let q = parse(r#"договор "аренды помещения" -расторжение OR контракт ext:pdf name:отчет*"#);
        assert_eq!(q.groups.len(), 4);
        assert_eq!(q.groups[0], vec![c("договор", ClauseKind::Term)]);
        // `-расторжение OR контракт`: the negated word goes to must_not, OR then
        // attaches `контракт` to the previous positive group (the phrase).
        assert_eq!(
            q.groups[1],
            vec![
                c("аренды помещения", ClauseKind::Phrase),
                c("контракт", ClauseKind::Term)
            ]
        );
        assert_eq!(q.must_not, vec![c("расторжение", ClauseKind::Term)]);
        assert_eq!(q.groups[2][0].field, FieldSel::Ext);
        assert_eq!(
            q.groups[3],
            vec![Clause {
                text: "отчет".into(),
                field: FieldSel::Name,
                kind: ClauseKind::Prefix
            }]
        );
    }

    #[test]
    fn or_groups() {
        let q = parse("яблоко OR груша слива");
        assert_eq!(q.groups.len(), 2);
        assert_eq!(q.groups[0].len(), 2);
        assert_eq!(q.describe(), "(яблоко OR груша) AND слива");
    }

    #[test]
    fn fuzzy_and_wildcards() {
        let q = parse("договор~ дог?вор д*р~2");
        assert_eq!(q.groups[0][0].kind, ClauseKind::Fuzzy(1));
        assert_eq!(q.groups[1][0].kind, ClauseKind::Wildcard);
        assert_eq!(q.groups[2][0].kind, ClauseKind::Fuzzy(2));
        assert_eq!(wildcard_to_regex("Дог?вор*"), "дог.вор.*");
    }

    #[test]
    fn russian_and_kazakh_operators() {
        let q = parse("алма НЕМЕСЕ алмұрт ЕМЕС банан");
        assert_eq!(q.groups.len(), 1);
        assert_eq!(q.groups[0].len(), 2);
        assert_eq!(q.must_not.len(), 1);
    }

    #[test]
    fn quotes_variants() {
        let q = parse("«красное яблоко»");
        assert_eq!(q.groups[0][0].kind, ClauseKind::Phrase);
        assert_eq!(q.groups[0][0].text, "красное яблоко");
    }

    #[test]
    fn empty_and_garbage() {
        assert!(parse("").is_empty());
        assert!(parse("   OR NOT - \"\" ").is_empty());
    }
}
