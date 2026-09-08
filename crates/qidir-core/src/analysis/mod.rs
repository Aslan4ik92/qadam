//! Multilingual text analysis for Russian, Kazakh and English.
//!
//! The pipeline is deliberately simple and deterministic so that exactly the
//! same transformation is applied at indexing and at query time:
//!
//! 1. **Tokenization** — a token is a maximal run of Unicode alphanumeric
//!    characters (letters of any script and digits). Punctuation, whitespace
//!    and underscores separate tokens.
//! 2. **Lower-casing** (Unicode aware).
//! 3. **Per-token language detection** by script and by the presence of the
//!    nine Kazakh-specific Cyrillic letters (`ә ғ қ ң ө ұ ү һ і`).
//! 4. **Stemming** (only in [`AnalysisMode::Stem`]): Snowball Russian for
//!    Cyrillic tokens, Snowball English (Porter2) for Latin tokens and a
//!    light-weight suffix-stripping stemmer for Kazakh tokens.
//! 5. **Character folding** — `ё → е`, Kazakh letters are folded to their
//!    closest Russian letters (`қ → к`, `ә → а`, ...), Latin diacritics are
//!    removed (`é → e`). This lets users type queries on a Russian keyboard and
//!    still find Kazakh text, and vice versa.
//!
//! Two analyzers are registered in every index: `qidir_stem` (full pipeline)
//! and `qidir_exact` (no stemming). The exact analyzer powers the
//! "exact word forms" search mode.

pub mod fold;
pub mod kazakh;
pub mod lang;
pub mod tokenizer;

pub use lang::{detect_token_lang, TokenLang};
pub use tokenizer::{analyze, AnalysisMode, AnalyzedToken, QidirTokenizer};

/// Name of the stemming analyzer registered in the tantivy index.
pub const STEM_TOKENIZER: &str = "qidir_stem";
/// Name of the exact (non-stemming) analyzer registered in the tantivy index.
pub const EXACT_TOKENIZER: &str = "qidir_exact";

/// Register both QIDIR analyzers on a tantivy index.
pub fn register_tokenizers(index: &tantivy::Index) {
    use tantivy::tokenizer::TextAnalyzer;
    index
        .tokenizers()
        .register(STEM_TOKENIZER, TextAnalyzer::builder(QidirTokenizer::new(AnalysisMode::Stem)).build());
    index
        .tokenizers()
        .register(EXACT_TOKENIZER, TextAnalyzer::builder(QidirTokenizer::new(AnalysisMode::Exact)).build());
}
