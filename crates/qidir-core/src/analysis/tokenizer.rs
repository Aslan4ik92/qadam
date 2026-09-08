//! The QIDIR tokenizer: tokenization → lower-casing → stemming → folding.

use rust_stemmers::{Algorithm, Stemmer};
use std::sync::OnceLock;
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

use super::fold::fold_token;
use super::kazakh;
use super::lang::{detect_token_lang, TokenLang};

/// Maximum token length (in characters) kept in the index. Longer tokens are
/// almost always hashes, base64 blobs or binary garbage.
pub const MAX_TOKEN_CHARS: usize = 64;

/// Whether to apply stemming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnalysisMode {
    /// Full pipeline including language-specific stemming.
    Stem,
    /// Lower-case + folding only. Exact word forms.
    Exact,
}

/// A token produced by [`analyze`], including offsets in the source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzedToken {
    /// Normalized (indexable) term text.
    pub term: String,
    /// Byte offset of the token start in the original text.
    pub offset_from: usize,
    /// Byte offset one past the token end in the original text.
    pub offset_to: usize,
    /// Zero-based word position.
    pub position: usize,
    /// Detected language of the original token.
    pub lang: TokenLang,
}

fn russian_stemmer() -> &'static Stemmer {
    static S: OnceLock<Stemmer> = OnceLock::new();
    S.get_or_init(|| Stemmer::create(Algorithm::Russian))
}

fn english_stemmer() -> &'static Stemmer {
    static S: OnceLock<Stemmer> = OnceLock::new();
    S.get_or_init(|| Stemmer::create(Algorithm::English))
}

/// Normalize a single already-lower-cased token according to `mode`: the
/// language-specific stem (Stem mode) or the folded form (Exact mode).
pub fn normalize_term(lower: &str, mode: AnalysisMode) -> (String, TokenLang) {
    let lang = detect_token_lang(lower);
    let stemmed: String = match (mode, lang) {
        (AnalysisMode::Exact, _) | (_, TokenLang::Other) => lower.to_string(),
        (AnalysisMode::Stem, TokenLang::Kazakh) => kazakh::stem(lower),
        (AnalysisMode::Stem, TokenLang::Russian) => {
            // Snowball Russian does not treat `ё` as `е`; fold first.
            let pre = fold_token(lower);
            russian_stemmer().stem(&pre).into_owned()
        }
        (AnalysisMode::Stem, TokenLang::English) => english_stemmer().stem(lower).into_owned(),
    };
    (fold_token(&stemmed), lang)
}

/// All index terms produced for one token.
///
/// In [`AnalysisMode::Stem`] a token yields several variants **at the same
/// position**:
///
/// * its exact folded form — so exact word forms always match (and rank
///   higher, because more terms match), even across languages: a query typed
///   on a Russian keyboard (`Казакстан`) finds Kazakh text (`Қазақстан`);
/// * the language-specific stem;
/// * for Cyrillic tokens **without** Kazakh-specific letters, *both* the
///   Russian and the Kazakh stem. Such tokens are genuinely ambiguous
///   (`мектеп`, `бала`, `дала` are Kazakh words spelled with Russian letters
///   only), and documents from Kazakhstan are frequently bilingual. Applying
///   both stemmers symmetrically on the index and query side gives correct
///   morphology for both languages at a modest index-size cost.
pub fn expand_term(lower: &str, mode: AnalysisMode) -> (Vec<String>, TokenLang) {
    let lang = detect_token_lang(lower);
    let exact = fold_token(lower);
    if mode == AnalysisMode::Exact || lang == TokenLang::Other {
        return (vec![exact], lang);
    }
    let mut out: Vec<String> = Vec::with_capacity(3);
    let mut push = |t: String| {
        if !t.is_empty() && !out.contains(&t) {
            out.push(t);
        }
    };
    match lang {
        TokenLang::Kazakh => push(fold_token(&kazakh::stem(lower))),
        TokenLang::Russian => {
            let pre = fold_token(lower);
            push(russian_stemmer().stem(&pre).into_owned());
            push(fold_token(&kazakh::stem(lower)));
        }
        TokenLang::English => push(english_stemmer().stem(lower).into_owned()),
        TokenLang::Other => {}
    }
    push(exact);
    (out, lang)
}

#[inline]
fn is_token_char(c: char) -> bool {
    c.is_alphanumeric()
}

/// Analyze `text` and return all tokens with byte offsets.
///
/// Several tokens may share a `position` (stem + exact form in Stem mode);
/// they are emitted consecutively.
///
/// This is the single source of truth used by the indexer (through
/// [`QidirTokenizer`]), the query parser and the preview highlighter.
pub fn analyze(text: &str, mode: AnalysisMode) -> Vec<AnalyzedToken> {
    let mut out = Vec::new();
    let mut position = 0usize;
    let mut start: Option<usize> = None;

    let push = |from: usize, to: usize, out: &mut Vec<AnalyzedToken>, position: &mut usize| {
        let raw = &text[from..to];
        if raw.chars().count() > MAX_TOKEN_CHARS {
            // Skip garbage tokens but still consume a position.
            *position += 1;
            return;
        }
        let lower = raw.to_lowercase();
        let (terms, lang) = expand_term(&lower, mode);
        let mut any = false;
        for term in terms {
            if term.is_empty() {
                continue;
            }
            any = true;
            out.push(AnalyzedToken { term, offset_from: from, offset_to: to, position: *position, lang });
        }
        if !any {
            *position += 1;
            return;
        }
        *position += 1;
    };

    for (idx, c) in text.char_indices() {
        if is_token_char(c) {
            if start.is_none() {
                start = Some(idx);
            }
        } else if let Some(s) = start.take() {
            push(s, idx, &mut out, &mut position);
        }
    }
    if let Some(s) = start {
        push(s, text.len(), &mut out, &mut position);
    }
    out
}

/// tantivy [`Tokenizer`] implementation backed by [`analyze`].
#[derive(Debug, Clone)]
pub struct QidirTokenizer {
    mode: AnalysisMode,
}

impl QidirTokenizer {
    pub fn new(mode: AnalysisMode) -> Self {
        Self { mode }
    }
}

/// Token stream over a pre-computed vector of tokens.
pub struct VecTokenStream {
    tokens: Vec<Token>,
    index: usize,
}

impl TokenStream for VecTokenStream {
    fn advance(&mut self) -> bool {
        // `index` starts at `usize::MAX` so the first advance lands on 0.
        self.index = self.index.wrapping_add(1);
        self.index < self.tokens.len()
    }

    fn token(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.tokens[self.index]
    }
}

impl Tokenizer for QidirTokenizer {
    type TokenStream<'a> = VecTokenStream;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        let tokens = analyze(text, self.mode)
            .into_iter()
            .map(|t| Token {
                offset_from: t.offset_from,
                offset_to: t.offset_to,
                position: t.position,
                text: t.term,
                position_length: 1,
            })
            .collect();
        VecTokenStream { tokens, index: usize::MAX }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms(text: &str, mode: AnalysisMode) -> Vec<String> {
        analyze(text, mode).into_iter().map(|t| t.term).collect()
    }

    /// Terms grouped by position.
    fn grouped(text: &str, mode: AnalysisMode) -> Vec<Vec<String>> {
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

    fn share_a_term(a: &[String], b: &[String]) -> bool {
        a.iter().any(|t| b.contains(t))
    }

    #[test]
    fn russian_inflections_share_a_stem() {
        let a = grouped("красные яблоки", AnalysisMode::Stem);
        let b = grouped("красное яблоко", AnalysisMode::Stem);
        assert_eq!(a.len(), 2);
        assert!(share_a_term(&a[0], &b[0]), "{a:?} vs {b:?}");
        assert!(share_a_term(&a[1], &b[1]), "{a:?} vs {b:?}");
        // Exact forms are kept alongside the stems.
        assert!(a[1].contains(&"яблоки".to_string()));
        assert!(b[1].contains(&"яблоко".to_string()));
    }

    #[test]
    fn english_stemming() {
        assert_eq!(
            terms("Searching documents", AnalysisMode::Stem),
            vec!["search", "searching", "document", "documents"]
        );
    }

    #[test]
    fn ambiguous_cyrillic_gets_both_stems() {
        // Kazakh words spelled with Russian letters only.
        let a = grouped("мектеп", AnalysisMode::Stem);
        let b = grouped("мектепке", AnalysisMode::Stem);
        assert!(share_a_term(&a[0], &b[0]), "{a:?} vs {b:?}");
        let a = grouped("балалар", AnalysisMode::Stem);
        let b = grouped("бала", AnalysisMode::Stem);
        assert!(share_a_term(&a[0], &b[0]), "{a:?} vs {b:?}");
        // Russian words keep working.
        let a = grouped("работа", AnalysisMode::Stem);
        let b = grouped("работе", AnalysisMode::Stem);
        assert!(share_a_term(&a[0], &b[0]), "{a:?} vs {b:?}");
    }

    #[test]
    fn kazakh_stemming_and_folding() {
        // "in our books" → folded stem of "кітап" plus the folded exact form
        assert_eq!(terms("кітаптарымызда", AnalysisMode::Stem), vec!["китап", "китаптарымызда"]);
        assert!(share_a_term(
            &grouped("кітап", AnalysisMode::Stem)[0],
            &grouped("кітаптарымызда", AnalysisMode::Stem)[0]
        ));
        // A query typed on a Russian keyboard reaches the Kazakh word through
        // the exact folded form even though the stems differ.
        let kz = grouped("Қазақстан", AnalysisMode::Stem);
        let ru = grouped("Казакстан", AnalysisMode::Stem);
        assert!(share_a_term(&kz[0], &ru[0]), "{kz:?} vs {ru:?}");
        // Inflected Kazakh forms share the Kazakh stem.
        let kz2 = grouped("Қазақстанның", AnalysisMode::Stem);
        assert!(share_a_term(&kz[0], &kz2[0]), "{kz:?} vs {kz2:?}");
    }

    #[test]
    fn exact_mode_does_not_stem_but_folds() {
        assert_eq!(terms("Ёлки яблоки", AnalysisMode::Exact), vec!["елки", "яблоки"]);
        assert_eq!(terms("Қазақстан", AnalysisMode::Exact), vec!["казакстан"]);
    }

    #[test]
    fn offsets_and_positions() {
        let toks = analyze("Привет, мир! 2024", AnalysisMode::Exact);
        assert_eq!(toks.len(), 3);
        assert_eq!(&"Привет, мир! 2024"[toks[0].offset_from..toks[0].offset_to], "Привет");
        assert_eq!(&"Привет, мир! 2024"[toks[1].offset_from..toks[1].offset_to], "мир");
        assert_eq!(toks[2].term, "2024");
        assert_eq!(toks[2].position, 2);
    }

    #[test]
    fn variants_share_position_and_offsets() {
        let toks = analyze("яблоки", AnalysisMode::Stem);
        assert_eq!(toks.len(), 2);
        assert_eq!(toks[0].position, toks[1].position);
        assert_eq!(toks[0].offset_from, toks[1].offset_from);
    }

    #[test]
    fn underscore_and_dots_split() {
        assert_eq!(
            terms("Отчет_2023_final.docx", AnalysisMode::Exact),
            vec!["отчет", "2023", "final", "docx"]
        );
    }

    #[test]
    fn long_garbage_skipped() {
        let junk = "a".repeat(200);
        assert!(terms(&junk, AnalysisMode::Exact).is_empty());
    }

    #[test]
    fn tantivy_tokenizer_streams() {
        use tantivy::tokenizer::TextAnalyzer;
        let mut an = TextAnalyzer::builder(QidirTokenizer::new(AnalysisMode::Stem)).build();
        let mut stream = an.token_stream("Документы поиска");
        let mut got = Vec::new();
        while stream.advance() {
            got.push((stream.token().position, stream.token().text.clone()));
        }
        assert_eq!(got[0], (0, "документ".to_string()));
        assert!(got.contains(&(0, "документы".to_string())));
        assert!(got.contains(&(1, "поиск".to_string())));
        assert!(got.contains(&(1, "поиска".to_string())));
        assert!(got.iter().all(|(p, _)| *p <= 1));
    }
}
