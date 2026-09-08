//! A light-weight suffix-stripping stemmer for Kazakh (Cyrillic script).
//!
//! Kazakh is an agglutinative language: a noun can carry plural, possessive and
//! case suffixes at the same time (`кітап-тар-ымыз-да` = "in our books"), verbs
//! carry tense/participle and personal endings. There is no Snowball stemmer
//! for Kazakh, so QIDIR ships its own conservative implementation.
//!
//! Design goals:
//! * **Deterministic and symmetric** — the same function is applied to indexed
//!   text and to queries, so even an imperfect stem still yields correct
//!   matches for words inflected the same way.
//! * **Conservative** — a suffix is only stripped when at least three
//!   characters remain, and at most four suffixes are stripped in total.
//! * **Order-aware** — suffixes are removed from the outside in, following the
//!   real morphotactics of the language: personal endings → case → possessive →
//!   plural → derivational/participle.

/// Personal (predicative) endings: `мен оқушы-мын`, `сен оқушы-сың`.
const PERSONAL: &[&str] =
    &["мын", "мін", "бын", "бін", "пын", "пін", "сың", "сің", "сыз", "сіз", "мыз", "міз", "ңыз", "ңіз"];

/// Case endings (genitive, dative, accusative, locative, ablative, instrumental).
const CASE: &[&str] = &[
    // genitive
    "ның",
    "нің",
    "дың",
    "дің",
    "тың",
    "тің",
    // ablative
    "дан",
    "ден",
    "тан",
    "тен",
    "нан",
    "нен",
    // instrumental
    "мен",
    "бен",
    "пен",
    "менен",
    "бенен",
    "пенен",
    // locative
    "нда",
    "нде",
    "да",
    "де",
    "та",
    "те",
    // dative
    "ға",
    "ге",
    "қа",
    "ке",
    "на",
    "не",
    // accusative
    "ны",
    "ні",
    "ды",
    "ді",
    "ты",
    "ті",
];

/// Possessive endings.
const POSSESSIVE: &[&str] = &[
    "ымыз", "іміз", "ыңыз", "іңіз", "мыз", "міз", "ңыз", "ңіз", "ым", "ім", "ың", "ің", "сы", "сі", "ы", "і",
    "м", "ң",
];

/// Plural endings.
const PLURAL: &[&str] = &["лар", "лер", "дар", "дер", "тар", "тер"];

/// Frequent verbal / derivational suffixes.
const VERBAL: &[&str] = &[
    // past participle
    "ған",
    "ген",
    "қан",
    "кен",
    // habitual participle
    "атын",
    "етін",
    "йтын",
    "йтін",
    // future participle
    "атын",
    "етін",
    "ар",
    "ер",
    "йар",
    "йер",
    // converbs
    "ып",
    "іп",
    // negative
    "ма",
    "ме",
    "ба",
    "бе",
    "па",
    "пе",
    // verbal noun
    "у",
    "ю",
    // nominal derivation
    "лық",
    "лік",
    "дық",
    "дік",
    "тық",
    "тік",
    "шы",
    "ші",
    "шылық",
    "шілік",
];

const MIN_STEM_CHARS: usize = 3;
const MAX_STRIPS: usize = 4;

/// Strip one suffix from `word` if it belongs to `suffixes` and enough
/// characters remain. Returns `true` on success.
fn strip_one(word: &mut String, suffixes: &[&str]) -> bool {
    let char_len = word.chars().count();
    // Try the longest matching suffix first.
    let mut best: Option<&str> = None;
    for s in suffixes {
        if word.ends_with(s) {
            let s_chars = s.chars().count();
            if char_len - s_chars >= MIN_STEM_CHARS
                && best.map(|b| b.chars().count() < s_chars).unwrap_or(true)
            {
                best = Some(s);
            }
        }
    }
    if let Some(s) = best {
        let new_len = word.len() - s.len();
        word.truncate(new_len);
        true
    } else {
        false
    }
}

/// Stem a lower-cased Kazakh word.
pub fn stem(word: &str) -> String {
    let mut w = word.to_string();
    if w.chars().count() <= MIN_STEM_CHARS {
        return w;
    }
    let mut strips = 0;

    // Stage 1: personal endings (only meaningful on predicates, but harmless).
    if strip_one(&mut w, PERSONAL) {
        strips += 1;
    }
    // Stage 2: case.
    if strips < MAX_STRIPS && strip_one(&mut w, CASE) {
        strips += 1;
    }
    // Stage 3: possessive (may appear twice in rare cases — strip once).
    if strips < MAX_STRIPS && strip_one(&mut w, POSSESSIVE) {
        strips += 1;
    }
    // Stage 4: plural.
    if strips < MAX_STRIPS && strip_one(&mut w, PLURAL) {
        strips += 1;
    }
    // Stage 5: verbal / derivational — only when nothing else matched, to stay
    // conservative on nouns.
    if strips == 0 {
        strip_one(&mut w, VERBAL);
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noun_paradigm_collapses_to_one_stem() {
        let forms = [
            "кітап",
            "кітаптар",
            "кітаптың",
            "кітапта",
            "кітаптан",
            "кітапқа",
            "кітапты",
            "кітаппен",
            "кітабым", // consonant alternation is *not* handled – stays distinct on purpose
        ];
        let stems: Vec<String> = forms.iter().map(|f| stem(f)).collect();
        assert_eq!(stems[0], "кітап");
        for s in &stems[..8] {
            assert_eq!(s, "кітап", "forms: {stems:?}");
        }
    }

    #[test]
    fn stacked_suffixes() {
        // кітап-тар-ымыз-да = "in our books"
        assert_eq!(stem("кітаптарымызда"), "кітап");
        // бала-лар-ы-ның = "of their children"
        assert_eq!(stem("балаларының"), "бала");
        // қала-лар-да = "in the cities"
        assert_eq!(stem("қалаларда"), "қала");
        // мектеп-тер-ге
        assert_eq!(stem("мектептерге"), "мектеп");
    }

    #[test]
    fn keeps_short_words() {
        assert_eq!(stem("ел"), "ел");
        assert_eq!(stem("бас"), "бас");
        assert_eq!(stem("ана"), "ана");
    }

    #[test]
    fn verbal_forms() {
        assert_eq!(stem("оқыған"), "оқы");
        assert_eq!(stem("жазу"), "жаз");
        assert_eq!(stem("келген"), "кел");
    }
}
