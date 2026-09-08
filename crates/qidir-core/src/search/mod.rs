//! Query execution: parsing, filtering, ranking, facets and snippets.

pub mod highlight;
pub mod query;

use std::collections::HashMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tantivy::collector::{Collector, Count, SegmentCollector, TopDocs};
use tantivy::columnar::StrColumn;
use tantivy::query::{AllQuery, BooleanQuery, Occur, Query, TermQuery};
use tantivy::schema::document::TantivyDocument;
use tantivy::schema::{IndexRecordOption, Value};
use tantivy::{DocAddress, DocId, Order, Score, Searcher, SegmentOrdinal, SegmentReader, Term};

use crate::analysis::AnalysisMode;
use crate::error::Result;
use crate::extract::FileCategory;
use crate::index::Fields;
pub use highlight::{Matcher, Snippet};
pub use query::{ParsedQuery, QueryBuilder};

/// Search mode chosen by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    /// Morphology-aware: `яблоко` finds `яблоки`, `кітап` finds `кітаптар`.
    #[default]
    Smart,
    /// Exact word forms (still case- and `ё/е`-insensitive).
    Exact,
}

impl SearchMode {
    pub fn analysis(self) -> AnalysisMode {
        match self {
            SearchMode::Smart => AnalysisMode::Stem,
            SearchMode::Exact => AnalysisMode::Exact,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    #[default]
    Relevance,
    ModifiedDesc,
    ModifiedAsc,
    SizeDesc,
    SizeAsc,
    NameAsc,
    NameDesc,
}

/// A search request. All filters are optional and combined with AND.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchRequest {
    pub query: String,
    pub mode: SearchMode,
    /// Restrict to these indexed roots (as stored, e.g. `D:\Docs`).
    pub roots: Vec<String>,
    pub categories: Vec<FileCategory>,
    /// Lower-case extensions without dot.
    pub extensions: Vec<String>,
    pub size_min: Option<u64>,
    pub size_max: Option<u64>,
    /// Unix seconds.
    pub modified_from: Option<i64>,
    pub modified_to: Option<i64>,
    /// Only files whose text was extracted.
    pub with_content_only: bool,
    pub sort: SortOrder,
    pub offset: usize,
    pub limit: usize,
    /// Approximate snippet length in characters.
    pub snippet_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub path: String,
    pub name: String,
    pub ext: String,
    pub category: FileCategory,
    pub root: String,
    pub size: u64,
    /// Unix seconds.
    pub modified: i64,
    pub score: f32,
    pub has_content: bool,
    /// HTML-escaped snippet with `<mark>` around matches; empty if none.
    pub snippet: String,
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FacetCount {
    pub value: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Facets {
    pub categories: Vec<FacetCount>,
    pub extensions: Vec<FacetCount>,
    pub roots: Vec<FacetCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub total: u64,
    pub hits: Vec<SearchHit>,
    pub facets: Facets,
    pub took_ms: u64,
    /// Human readable explanation of how the query was interpreted.
    pub interpretation: String,
    pub offset: usize,
    pub limit: usize,
}

/// Maximum results fetched for in-memory sorts (by name).
const NAME_SORT_WINDOW: usize = 5000;

/// Execute a search against `searcher`.
pub fn search(searcher: &Searcher, fields: &Fields, req: &SearchRequest) -> Result<SearchResponse> {
    let started = Instant::now();
    let limit = if req.limit == 0 { 50 } else { req.limit.min(1000) };
    let snippet_chars = if req.snippet_chars == 0 { 220 } else { req.snippet_chars.clamp(60, 2000) };

    let builder = QueryBuilder::new(*fields, req.mode);
    let parsed = builder.parse(&req.query);
    let text_query = builder.build(&parsed)?;
    let matcher = builder.matcher(&parsed);

    let mut clauses: Vec<(Occur, Box<dyn Query>)> = Vec::new();
    match text_query {
        Some(q) => clauses.push((Occur::Must, q)),
        None => clauses.push((Occur::Must, Box::new(AllQuery))),
    }
    add_filters(fields, req, &mut clauses);
    let query: Box<dyn Query> =
        if clauses.len() == 1 { clauses.pop().unwrap().1 } else { Box::new(BooleanQuery::new(clauses)) };

    let facet_collector = FacetCollector { fields: *fields };
    let addresses: Vec<(Score, DocAddress)>;
    let total: u64;
    let facets: Facets;

    let effective_sort = if parsed.is_empty() && req.sort == SortOrder::Relevance {
        SortOrder::ModifiedDesc
    } else {
        req.sort
    };

    match effective_sort {
        SortOrder::Relevance => {
            let top = TopDocs::with_limit(limit).and_offset(req.offset);
            let (t, docs, f) = searcher.search(&query, &(Count, top, facet_collector))?;
            total = t as u64;
            addresses = docs;
            facets = f;
        }
        SortOrder::ModifiedDesc | SortOrder::ModifiedAsc => {
            let order = if effective_sort == SortOrder::ModifiedDesc { Order::Desc } else { Order::Asc };
            let top = TopDocs::with_limit(limit)
                .and_offset(req.offset)
                .order_by_fast_field::<i64>("modified", order);
            let (t, docs, f) = searcher.search(&query, &(Count, top, facet_collector))?;
            total = t as u64;
            addresses = docs.into_iter().map(|(_, a)| (0.0, a)).collect();
            facets = f;
        }
        SortOrder::SizeDesc | SortOrder::SizeAsc => {
            let order = if effective_sort == SortOrder::SizeDesc { Order::Desc } else { Order::Asc };
            let top =
                TopDocs::with_limit(limit).and_offset(req.offset).order_by_fast_field::<u64>("size", order);
            let (t, docs, f) = searcher.search(&query, &(Count, top, facet_collector))?;
            total = t as u64;
            addresses = docs.into_iter().map(|(_, a)| (0.0, a)).collect();
            facets = f;
        }
        SortOrder::NameAsc | SortOrder::NameDesc => {
            let top = TopDocs::with_limit(NAME_SORT_WINDOW);
            let (t, docs, f) = searcher.search(&query, &(Count, top, facet_collector))?;
            total = t as u64;
            facets = f;
            let mut named: Vec<(String, Score, DocAddress)> = docs
                .into_iter()
                .map(|(s, a)| {
                    let name = searcher
                        .doc::<TantivyDocument>(a)
                        .ok()
                        .and_then(|d| {
                            d.get_first(fields.name).and_then(|v| v.as_str()).map(|s| s.to_lowercase())
                        })
                        .unwrap_or_default();
                    (name, s, a)
                })
                .collect();
            named.sort_by(|a, b| a.0.cmp(&b.0));
            if effective_sort == SortOrder::NameDesc {
                named.reverse();
            }
            addresses = named.into_iter().skip(req.offset).take(limit).map(|(_, s, a)| (s, a)).collect();
        }
    }

    let mut hits = Vec::with_capacity(addresses.len());
    for (score, addr) in addresses {
        let doc: TantivyDocument = searcher.doc(addr)?;
        hits.push(hit_from_doc(fields, &doc, score, &matcher, req.mode, snippet_chars));
    }

    Ok(SearchResponse {
        total,
        hits,
        facets,
        took_ms: started.elapsed().as_millis() as u64,
        interpretation: parsed.describe(),
        offset: req.offset,
        limit,
    })
}

fn add_filters(fields: &Fields, req: &SearchRequest, clauses: &mut Vec<(Occur, Box<dyn Query>)>) {
    if !req.roots.is_empty() {
        let alts: Vec<(Occur, Box<dyn Query>)> = req
            .roots
            .iter()
            .map(|r| {
                let q: Box<dyn Query> =
                    Box::new(TermQuery::new(Term::from_field_text(fields.root, r), IndexRecordOption::Basic));
                (Occur::Should, q)
            })
            .collect();
        clauses.push((Occur::Must, Box::new(BooleanQuery::new(alts))));
    }
    if !req.categories.is_empty() {
        let alts: Vec<(Occur, Box<dyn Query>)> = req
            .categories
            .iter()
            .map(|c| {
                let q: Box<dyn Query> = Box::new(TermQuery::new(
                    Term::from_field_text(fields.category, c.as_str()),
                    IndexRecordOption::Basic,
                ));
                (Occur::Should, q)
            })
            .collect();
        clauses.push((Occur::Must, Box::new(BooleanQuery::new(alts))));
    }
    if !req.extensions.is_empty() {
        let alts: Vec<(Occur, Box<dyn Query>)> = req
            .extensions
            .iter()
            .map(|e| {
                let e = e.trim().trim_start_matches('.').to_lowercase();
                let q: Box<dyn Query> =
                    Box::new(TermQuery::new(Term::from_field_text(fields.ext, &e), IndexRecordOption::Basic));
                (Occur::Should, q)
            })
            .collect();
        clauses.push((Occur::Must, Box::new(BooleanQuery::new(alts))));
    }
    if req.with_content_only {
        clauses.push((
            Occur::Must,
            Box::new(TermQuery::new(Term::from_field_u64(fields.has_content, 1), IndexRecordOption::Basic)),
        ));
    }
    if req.size_min.is_some() || req.size_max.is_some() {
        clauses.push((Occur::Must, query::u64_range(fields.size, req.size_min, req.size_max)));
    }
    if req.modified_from.is_some() || req.modified_to.is_some() {
        clauses.push((Occur::Must, query::i64_range(fields.modified, req.modified_from, req.modified_to)));
    }
}

fn hit_from_doc(
    fields: &Fields,
    doc: &TantivyDocument,
    score: Score,
    matcher: &Matcher,
    mode: SearchMode,
    snippet_chars: usize,
) -> SearchHit {
    let text = |f| doc.get_first(f).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let body = doc.get_first(fields.body).and_then(|v| v.as_str());
    let snippet = match body {
        Some(b) if !matcher.is_empty() => highlight::snippet(b, matcher, mode.analysis(), snippet_chars).html,
        Some(b) => highlight::leading_snippet(b, snippet_chars),
        None => String::new(),
    };
    let ext = text(fields.ext);
    let category = FileCategory::parse(&text(fields.category)).unwrap_or(FileCategory::Other);
    SearchHit {
        path: text(fields.path),
        name: text(fields.name),
        ext,
        category,
        root: text(fields.root),
        size: doc.get_first(fields.size).and_then(|v| v.as_u64()).unwrap_or(0),
        modified: doc.get_first(fields.modified).and_then(|v| v.as_i64()).unwrap_or(0),
        score,
        has_content: doc.get_first(fields.has_content).and_then(|v| v.as_u64()).unwrap_or(0) == 1,
        snippet,
        encoding: doc.get_first(fields.encoding).and_then(|v| v.as_str()).map(|s| s.to_string()),
    }
}

// ----------------------------------------------------------------------------
// Facet collector over string fast fields
// ----------------------------------------------------------------------------

struct FacetCollector {
    fields: Fields,
}

struct FacetSegmentCollector {
    ext: Option<StrColumn>,
    category: Option<StrColumn>,
    root: Option<StrColumn>,
    ext_counts: HashMap<u64, u64>,
    cat_counts: HashMap<u64, u64>,
    root_counts: HashMap<u64, u64>,
}

impl Collector for FacetCollector {
    type Fruit = Facets;
    type Child = FacetSegmentCollector;

    fn for_segment(
        &self,
        _segment_local_id: SegmentOrdinal,
        segment: &SegmentReader,
    ) -> tantivy::Result<Self::Child> {
        let ff = segment.fast_fields();
        let schema = segment.schema();
        Ok(FacetSegmentCollector {
            ext: ff.str(schema.get_field_name(self.fields.ext))?,
            category: ff.str(schema.get_field_name(self.fields.category))?,
            root: ff.str(schema.get_field_name(self.fields.root))?,
            ext_counts: HashMap::new(),
            cat_counts: HashMap::new(),
            root_counts: HashMap::new(),
        })
    }

    fn requires_scoring(&self) -> bool {
        false
    }

    fn merge_fruits(&self, segment_fruits: Vec<Facets>) -> tantivy::Result<Facets> {
        let mut cats: HashMap<String, u64> = HashMap::new();
        let mut exts: HashMap<String, u64> = HashMap::new();
        let mut roots: HashMap<String, u64> = HashMap::new();
        for f in segment_fruits {
            for c in f.categories {
                *cats.entry(c.value).or_default() += c.count;
            }
            for e in f.extensions {
                *exts.entry(e.value).or_default() += e.count;
            }
            for r in f.roots {
                *roots.entry(r.value).or_default() += r.count;
            }
        }
        Ok(Facets {
            categories: sorted_counts(cats),
            extensions: sorted_counts(exts),
            roots: sorted_counts(roots),
        })
    }
}

fn sorted_counts(m: HashMap<String, u64>) -> Vec<FacetCount> {
    let mut v: Vec<FacetCount> = m.into_iter().map(|(value, count)| FacetCount { value, count }).collect();
    v.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.value.cmp(&b.value)));
    v
}

fn count_ords(col: &Option<StrColumn>, doc: DocId, counts: &mut HashMap<u64, u64>) {
    if let Some(c) = col {
        for ord in c.term_ords(doc) {
            *counts.entry(ord).or_default() += 1;
        }
    }
}

fn resolve(col: &Option<StrColumn>, counts: HashMap<u64, u64>) -> Vec<FacetCount> {
    let mut out = Vec::with_capacity(counts.len());
    if let Some(c) = col {
        let mut buf = String::new();
        for (ord, count) in counts {
            buf.clear();
            if c.ord_to_str(ord, &mut buf).unwrap_or(false) {
                out.push(FacetCount { value: buf.clone(), count });
            }
        }
    }
    out
}

impl SegmentCollector for FacetSegmentCollector {
    type Fruit = Facets;

    fn collect(&mut self, doc: DocId, _score: Score) {
        count_ords(&self.ext, doc, &mut self.ext_counts);
        count_ords(&self.category, doc, &mut self.cat_counts);
        count_ords(&self.root, doc, &mut self.root_counts);
    }

    fn harvest(self) -> Facets {
        Facets {
            extensions: resolve(&self.ext, self.ext_counts),
            categories: resolve(&self.category, self.cat_counts),
            roots: resolve(&self.root, self.root_counts),
        }
    }
}
