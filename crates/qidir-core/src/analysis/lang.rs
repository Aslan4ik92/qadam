//! Script / language detection for individual tokens.

/// Language guessed for a single token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenLang {
    /// Cyrillic token containing at least one Kazakh-specific letter.
    Kazakh,
    /// Cyrillic token without Kazakh-specific letters (treated as Russian).
    Russian,
    /// Token written in Latin script (treated as English).
    English,
    /// Digits, mixed scripts, other scripts — left untouched by stemmers.
    Other,
}

/// Letters that exist in the Kazakh Cyrillic alphabet but not in Russian.
pub const KAZAKH_LETTERS: &[char] = &['ә', 'ғ', 'қ', 'ң', 'ө', 'ұ', 'ү', 'һ', 'і'];

#[inline]
pub fn is_cyrillic(c: char) -> bool {
    matches!(c, '\u{0400}'..='\u{052F}' | '\u{2DE0}'..='\u{2DFF}' | '\u{A640}'..='\u{A69F}')
}

#[inline]
pub fn is_latin(c: char) -> bool {
    c.is_ascii_alphabetic() || matches!(c, '\u{00C0}'..='\u{024F}' | '\u{1E00}'..='\u{1EFF}')
}

#[inline]
pub fn is_kazakh_letter(c: char) -> bool {
    KAZAKH_LETTERS.contains(&c)
}

/// Detect the language of a (lower-cased) token.
///
/// Tokens containing digits or mixing scripts are classified as
/// [`TokenLang::Other`] so they are never stemmed.
pub fn detect_token_lang(token: &str) -> TokenLang {
    let mut cyr = false;
    let mut lat = false;
    let mut kz = false;
    for c in token.chars() {
        if c.is_ascii_digit() || c.is_numeric() {
            return TokenLang::Other;
        }
        if is_cyrillic(c) {
            cyr = true;
            if is_kazakh_letter(c) {
                kz = true;
            }
        } else if is_latin(c) {
            lat = true;
        } else {
            return TokenLang::Other;
        }
    }
    match (cyr, lat) {
        (true, false) if kz => TokenLang::Kazakh,
        (true, false) => TokenLang::Russian,
        (false, true) => TokenLang::English,
        _ => TokenLang::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_languages() {
        assert_eq!(detect_token_lang("қазақстан"), TokenLang::Kazakh);
        assert_eq!(detect_token_lang("кітап"), TokenLang::Kazakh);
        assert_eq!(detect_token_lang("россия"), TokenLang::Russian);
        assert_eq!(detect_token_lang("search"), TokenLang::English);
        assert_eq!(detect_token_lang("abc123"), TokenLang::Other);
        assert_eq!(detect_token_lang("2024"), TokenLang::Other);
        assert_eq!(detect_token_lang("абвabc"), TokenLang::Other);
        assert_eq!(detect_token_lang("日本"), TokenLang::Other);
    }
}
