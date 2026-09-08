//! Snippet generation and match highlighting driven by the same analyzer
//! that built the index, so highlighted words are exactly those that matched
//! (including inflected forms in Smart mode).

use std::collections::HashSet;
use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::analysis::tokenizer::expand_term;
use crate::analysis::{analyze, AnalysisMode};

/// Predicates describing which normalized tokens count as matches.
#[derive(Debug, Default, Clone)]
pub struct Matcher {
    /// Terms normalized with the search mode's analyzer.
    pub terms: HashSet<String>,
    /// Exact-mode prefixes.
    pub prefixes: Vec<String>,
    /// Exact-mode anchored regexes.
    pub regexes: Vec<regex::Regex>,
    /// Exact-mode fuzzy terms with max edit distance.
    pub fuzzy: Vec<(String, u8)>,
}

impl Matcher {
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
            && self.prefixes.is_empty()
            && self.regexes.is_empty()
            && self.fuzzy.is_empty()
    }

    fn needs_exact(&self) -> bool {
        !self.prefixes.is_empty() || !self.regexes.is_empty() || !self.fuzzy.is_empty()
    }

    /// Does a token match? `mode_term` is the token normalized with the search
    /// mode analyzer, `exact_term` with the exact analyzer.
    fn matches(&self, mode_term: &str, exact_term: &str) -> bool {
        if self.terms.contains(mode_term) {
            return true;
        }
        if self
            .prefixes
            .iter()
            .any(|p| exact_term.starts_with(p.as_str()))
        {
            return true;
        }
        if self.regexes.iter().any(|r| r.is_match(exact_term)) {
            return true;
        }
        self.fuzzy
            .iter()
            .any(|(t, d)| levenshtein_within(t, exact_term, *d as usize))
    }
}

/// Byte ranges of matching tokens in `text`, scanning at most `max_scan_bytes`
/// and stopping after `max_matches` matches (0 = unlimited).
pub fn find_matches(
    text: &str,
    matcher: &Matcher,
    mode: AnalysisMode,
    max_matches: usize,
) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    if matcher.is_empty() {
        return out;
    }
    let needs_exact = matcher.needs_exact();
    // Analyze in the exact mode (cheap) and derive the mode term when needed.
    for tok in analyze(text, AnalysisMode::Exact) {
        let exact_term = tok.term.as_str();
        // In Stem mode the query terms contain both stems and exact forms, so a
        // token matches when either of its variants is present.
        let hit = if mode == AnalysisMode::Exact {
            matcher.matches(exact_term, exact_term)
        } else if matcher.terms.contains(exact_term) {
            true
        } else {
            let raw = &text[tok.offset_from..tok.offset_to];
            let (variants, _) = expand_term(&raw.to_lowercase(), mode);
            if variants.iter().any(|v| matcher.terms.contains(v)) {
                true
            } else if needs_exact {
                matcher.matches(exact_term, exact_term)
            } else {
                false
            }
        };
        if hit {
            out.push(tok.offset_from..tok.offset_to);
            if max_matches != 0 && out.len() >= max_matches {
                break;
            }
        }
    }
    out
}

/// A snippet ready for the UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    /// HTML-escaped text with `<mark>…</mark>` around matches.
    pub html: String,
    /// Number of highlighted matches in the snippet.
    pub matches: usize,
}

/// Size of the chunks scanned progressively when looking for the first match
/// in a large document.
const SCAN_CHUNK: usize = 64 * 1024;

/// Build the best snippet of roughly `max_chars` characters.
///
/// Large documents are scanned progressively: the first chunk that contains a
/// match is used, so snippets stay cheap even for multi-megabyte texts.
pub fn snippet(text: &str, matcher: &Matcher, mode: AnalysisMode, max_chars: usize) -> Snippet {
    if matcher.is_empty() || text.is_empty() {
        return Snippet {
            html: leading_snippet(text, max_chars),
            matches: 0,
        };
    }
    let mut start = 0usize;
    while start < text.len() {
        let mut end = (start + SCAN_CHUNK).min(text.len());
        // Extend to a char boundary and past the current word.
        while end < text.len()
            && (!text.is_char_boundary(end)
                || text[end..].starts_with(|c: char| c.is_alphanumeric()))
        {
            end += 1;
        }
        let chunk = &text[start..end];
        let matches = find_matches(chunk, matcher, mode, 64);
        if !matches.is_empty() {
            let (html, n) = render_window(chunk, &matches, max_chars);
            return Snippet { html, matches: n };
        }
        start = end;
    }
    Snippet {
        html: leading_snippet(text, max_chars),
        matches: 0,
    }
}

/// Snippet of the beginning of the text without highlighting.
pub fn leading_snippet(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (count, c) in text.chars().enumerate() {
        if count >= max_chars {
            out.push('…');
            break;
        }
        out.push(if c == '\n' || c == '\t' { ' ' } else { c });
    }
    escape_html(&out)
}

/// Choose the window around the densest cluster of matches and render it.
fn render_window(text: &str, matches: &[Range<usize>], max_chars: usize) -> (String, usize) {
    // Pick the window start that covers the most matches within max_chars bytes
    // (approximation: bytes ≈ chars for the window size computation, then fix up).
    let window_bytes = max_chars * 2; // Cyrillic is 2 bytes per char.
    let mut best_i = 0;
    let mut best_count = 0;
    for i in 0..matches.len() {
        let from = matches[i].start;
        let count = matches[i..]
            .iter()
            .take_while(|m| m.end <= from + window_bytes)
            .count();
        if count > best_count {
            best_count = count;
            best_i = i;
        }
    }
    let first = &matches[best_i];
    // Centre the window: put some context before the first match.
    let lead = window_bytes / 4;
    let mut start = first.start.saturating_sub(lead);
    while start > 0 && !text.is_char_boundary(start) {
        start -= 1;
    }
    // Snap to word boundary.
    if start > 0 {
        if let Some(p) = text[start..].find(|c: char| c.is_whitespace()) {
            start += p;
        }
    }
    let mut end = (start + window_bytes).min(text.len());
    while end < text.len() && !text.is_char_boundary(end) {
        end += 1;
    }
    if end < text.len() {
        if let Some(p) = text[..end].rfind(|c: char| c.is_whitespace()) {
            if p > start {
                end = p;
            }
        }
    }

    let mut html = String::with_capacity((end - start) * 2);
    if start > 0 {
        html.push('…');
    }
    let mut cursor = start;
    let mut n = 0;
    for m in matches {
        if m.start < start {
            continue;
        }
        if m.end > end {
            break;
        }
        html.push_str(&escape_html(&squash(&text[cursor..m.start])));
        html.push_str("<mark>");
        html.push_str(&escape_html(&text[m.start..m.end]));
        html.push_str("</mark>");
        cursor = m.end;
        n += 1;
    }
    html.push_str(&escape_html(&squash(&text[cursor..end])));
    if end < text.len() {
        html.push('…');
    }
    (html, n)
}

fn squash(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_ws = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_ws {
                out.push(' ');
            }
            prev_ws = true;
        } else {
            out.push(c);
            prev_ws = false;
        }
    }
    out
}

pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// Levenshtein distance bounded by `max` (early exit).
pub fn levenshtein_within(a: &str, b: &str, max: usize) -> bool {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > max {
        return false;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        let mut row_min = cur[0];
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            row_min = row_min.min(cur[j]);
        }
        if row_min > max {
            return false;
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()] <= max
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matcher(words: &[&str], mode: AnalysisMode) -> Matcher {
        let mut m = Matcher::default();
        for w in words {
            for t in analyze(w, mode) {
                m.terms.insert(t.term);
            }
        }
        m
    }

    #[test]
    fn highlights_inflected_forms_in_smart_mode() {
        let text = "Красное яблоко лежало на столе. Красные яблоки были вкусными.";
        let m = matcher(&["яблоко"], AnalysisMode::Stem);
        let ranges = find_matches(text, &m, AnalysisMode::Stem, 0);
        let words: Vec<&str> = ranges.iter().map(|r| &text[r.clone()]).collect();
        assert_eq!(words, vec!["яблоко", "яблоки"]);
    }

    #[test]
    fn exact_mode_only_exact_forms() {
        let text = "яблоко и яблоки";
        let m = matcher(&["яблоко"], AnalysisMode::Exact);
        let ranges = find_matches(text, &m, AnalysisMode::Exact, 0);
        assert_eq!(ranges.len(), 1);
    }

    #[test]
    fn snippet_marks_and_escapes() {
        let text = "a <b> ".repeat(10)
            + "Здесь находится важное слово договор и всё остальное. "
            + &"x ".repeat(200);
        let m = matcher(&["договор"], AnalysisMode::Stem);
        let s = snippet(&text, &m, AnalysisMode::Stem, 60);
        assert!(s.html.contains("<mark>договор</mark>"), "{}", s.html);
        assert!(s.html.contains("&lt;b&gt;") || !s.html.contains("<b>"));
        assert_eq!(s.matches, 1);
        assert!(s.html.starts_with('…'));
        assert!(s.html.ends_with('…'));
    }

    #[test]
    fn snippet_in_large_document_found_late() {
        let mut text = "слово ".repeat(50_000); // ~600 KB
        text.push_str("уникальныйтермин конец");
        let m = matcher(&["уникальныйтермин"], AnalysisMode::Exact);
        let s = snippet(&text, &m, AnalysisMode::Exact, 100);
        assert!(s.html.contains("<mark>уникальныйтермин</mark>"));
    }

    #[test]
    fn prefix_and_fuzzy() {
        let text = "договоры договорённость дгоовор";
        let mut m = Matcher::default();
        m.prefixes.push("догов".into());
        assert_eq!(find_matches(text, &m, AnalysisMode::Exact, 0).len(), 2);
        let mut f = Matcher::default();
        f.fuzzy.push(("договор".into(), 2));
        // договоры (1 edit) and дгоовор (2 edits); договорённость is too far.
        assert_eq!(find_matches(text, &f, AnalysisMode::Exact, 0).len(), 2);
    }

    #[test]
    fn levenshtein() {
        assert!(levenshtein_within("kitten", "sitting", 3));
        assert!(!levenshtein_within("kitten", "sitting", 2));
        assert!(levenshtein_within("договор", "договр", 1));
    }
}
