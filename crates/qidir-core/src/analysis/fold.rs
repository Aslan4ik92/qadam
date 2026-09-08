//! Character folding: makes searching tolerant to keyboard layout limitations
//! and orthographic variants.
//!
//! * `ё → е` (Russian texts frequently omit the diaeresis).
//! * Kazakh letters are folded to their nearest Russian letters so that a query
//!   typed on a Russian keyboard (`казак`) still finds `қазақ`, and vice versa.
//! * Ukrainian/Belarusian letters that occur in mixed archives are folded too.
//! * Latin letters lose their diacritics (`résumé → resume`).

use unicode_normalization::UnicodeNormalization;

use super::lang::{is_cyrillic, is_latin};

/// Fold a single Cyrillic character.
#[inline]
pub fn fold_cyrillic_char(c: char) -> char {
    match c {
        'ё' => 'е',
        // Kazakh
        'ә' => 'а',
        'ғ' => 'г',
        'қ' => 'к',
        'ң' => 'н',
        'ө' => 'о',
        'ұ' => 'у',
        'ү' => 'у',
        'һ' => 'х',
        'і' => 'и',
        // Ukrainian / Belarusian
        'ї' => 'и',
        'є' => 'е',
        'ґ' => 'г',
        'ў' => 'у',
        _ => c,
    }
}

/// Fold a lower-cased token: Cyrillic mapping + Latin diacritic stripping.
pub fn fold_token(token: &str) -> String {
    let mut needs_latin_fold = false;
    let mut needs_cyr_fold = false;
    for c in token.chars() {
        if is_cyrillic(c) {
            if fold_cyrillic_char(c) != c {
                needs_cyr_fold = true;
            }
        } else if !c.is_ascii() && is_latin(c) {
            needs_latin_fold = true;
        }
    }
    if !needs_latin_fold && !needs_cyr_fold {
        return token.to_string();
    }

    let mut out = String::with_capacity(token.len());
    for c in token.chars() {
        if is_cyrillic(c) {
            out.push(fold_cyrillic_char(c));
        } else if !c.is_ascii() && is_latin(c) {
            match c {
                'ß' => out.push_str("ss"),
                'æ' => out.push_str("ae"),
                'œ' => out.push_str("oe"),
                'ø' => out.push('o'),
                'đ' => out.push('d'),
                'ł' => out.push('l'),
                'ı' => out.push('i'),
                _ => {
                    // Decompose and drop combining marks.
                    for d in c.nfkd() {
                        if !is_combining_mark(d) {
                            out.push(d);
                        }
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[inline]
fn is_combining_mark(c: char) -> bool {
    matches!(c, '\u{0300}'..='\u{036F}' | '\u{1AB0}'..='\u{1AFF}' | '\u{1DC0}'..='\u{1DFF}' | '\u{20D0}'..='\u{20FF}' | '\u{FE20}'..='\u{FE2F}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_kazakh_to_russian() {
        assert_eq!(fold_token("қазақстан"), "казакстан");
        assert_eq!(fold_token("әсем"), "асем");
        assert_eq!(fold_token("өткен"), "откен");
        assert_eq!(fold_token("білім"), "билим");
        assert_eq!(fold_token("ұлт"), "улт");
    }

    #[test]
    fn folds_yo() {
        assert_eq!(fold_token("ёлка"), "елка");
        assert_eq!(fold_token("елка"), "елка");
    }

    #[test]
    fn folds_latin_diacritics() {
        assert_eq!(fold_token("résumé"), "resume");
        assert_eq!(fold_token("straße"), "strasse");
        assert_eq!(fold_token("naïve"), "naive");
    }

    #[test]
    fn keeps_short_i_intact() {
        // `й` must NOT be decomposed into `и` + breve.
        assert_eq!(fold_token("йогурт"), "йогурт");
        assert_eq!(fold_token("майор"), "майор");
    }
}
