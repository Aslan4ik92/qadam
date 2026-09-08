//! Plain-text decoding with legacy encoding detection.
//!
//! Old Russian/Kazakh documents are frequently stored in Windows-1251, KOI8-R,
//! CP866 (DOS) or UTF-16. QIDIR detects BOMs first and then uses `chardetng`
//! which is tuned for exactly these Cyrillic encodings.

use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, UTF_8};

/// Result of decoding.
pub struct Decoded {
    pub text: String,
    /// Name of the detected encoding (`UTF-8`, `windows-1251`, ...).
    pub encoding: &'static str,
}

/// Detect BOM-less UTF-16 by looking at the high bytes of each code unit.
///
/// Latin text has `0x00` high bytes, Cyrillic text `0x04` (U+0400–U+04FF);
/// real text mixes them with spaces/newlines (`0x00`). Returns the encoding
/// when ≥ 90 % of one byte parity looks like high bytes and the other parity
/// does not.
pub fn detect_bomless_utf16(bytes: &[u8]) -> Option<&'static Encoding> {
    let sample = &bytes[..bytes.len().min(4096) & !1];
    if sample.len() < 8 {
        return None;
    }
    let is_hi = |b: u8| b == 0x00 || b == 0x04 || b == 0x05 || b == 0x20 || b == 0x21;
    let half = sample.len() / 2;
    let odd_hi = sample
        .iter()
        .skip(1)
        .step_by(2)
        .filter(|&&b| is_hi(b))
        .count();
    let even_hi = sample.iter().step_by(2).filter(|&&b| is_hi(b)).count();
    // Low bytes of real text are mostly printable.
    let even_printable = sample
        .iter()
        .step_by(2)
        .filter(|&&b| b >= 0x20 || b == b'\n' || b == b'\r' || b == b'\t')
        .count();
    let odd_printable = sample
        .iter()
        .skip(1)
        .step_by(2)
        .filter(|&&b| b >= 0x20 || b == b'\n' || b == b'\r' || b == b'\t')
        .count();
    if odd_hi * 10 >= half * 9 && even_hi * 10 <= half * 3 && even_printable * 10 >= half * 9 {
        return Some(UTF_16LE);
    }
    if even_hi * 10 >= half * 9 && odd_hi * 10 <= half * 3 && odd_printable * 10 >= half * 9 {
        return Some(UTF_16BE);
    }
    None
}

/// Heuristic: does this look like binary data rather than text?
pub fn looks_binary(bytes: &[u8]) -> bool {
    let sample = &bytes[..bytes.len().min(8192)];
    if sample.is_empty() {
        return false;
    }
    if sample.starts_with(&[0xFF, 0xFE]) || sample.starts_with(&[0xFE, 0xFF]) {
        return false;
    }
    let nul = sample.iter().filter(|&&b| b == 0).count();
    if nul == 0 {
        return false;
    }
    if detect_bomless_utf16(sample).is_some() {
        return false;
    }
    nul * 100 / sample.len() > 1
}

/// Decode `bytes` into a `String`, guessing the encoding.
pub fn decode(bytes: &[u8]) -> Decoded {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Decoded {
            text: UTF_8.decode_with_bom_removal(bytes).0.into_owned(),
            encoding: "UTF-8",
        };
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return Decoded {
            text: UTF_16LE.decode_with_bom_removal(bytes).0.into_owned(),
            encoding: "UTF-16LE",
        };
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return Decoded {
            text: UTF_16BE.decode_with_bom_removal(bytes).0.into_owned(),
            encoding: "UTF-16BE",
        };
    }
    // BOM-less UTF-16 (common for Windows "Unicode" text files) must be
    // checked before UTF-8 validation: Cyrillic UTF-16LE happens to be valid
    // UTF-8 garbage (`0x22 0x04` ...).
    if let Some(enc) = detect_bomless_utf16(bytes) {
        return Decoded {
            text: enc.decode_without_bom_handling(bytes).0.into_owned(),
            encoding: enc.name(),
        };
    }
    // Valid UTF-8 wins outright.
    if let Ok(s) = std::str::from_utf8(bytes) {
        return Decoded {
            text: s.to_string(),
            encoding: "UTF-8",
        };
    }
    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(&bytes[..bytes.len().min(256 * 1024)], true);
    // Hint the detector towards Cyrillic legacy encodings via TLD `ru`.
    let enc: &'static Encoding = detector.guess(Some(b"ru"), true);
    let (cow, _, _) = enc.decode(bytes);
    Decoded {
        text: cow.into_owned(),
        encoding: enc.name(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_passthrough() {
        let d = decode("Привет, мир".as_bytes());
        assert_eq!(d.text, "Привет, мир");
        assert_eq!(d.encoding, "UTF-8");
    }

    #[test]
    fn cp1251() {
        let (bytes, _, _) = encoding_rs::WINDOWS_1251
            .encode("Отчёт о продажах за квартал. Қазақстан Республикасы.");
        let d = decode(&bytes);
        assert_eq!(d.encoding, "windows-1251");
        assert!(d.text.contains("Отчёт о продажах"));
    }

    #[test]
    fn koi8r() {
        let (bytes, _, _) = encoding_rs::KOI8_R.encode(
            "Это довольно длинный текст на русском языке, чтобы детектор кодировок уверенно определил КОИ-8.",
        );
        let d = decode(&bytes);
        assert!(d.encoding.starts_with("KOI8"), "{}", d.encoding);
        assert!(d.text.contains("русском языке"));
    }

    #[test]
    fn utf16_with_and_without_bom() {
        let text = "Тестовый документ Word\nВторая строка текста, довольно длинная.";
        let mut with_bom = vec![0xFF, 0xFE];
        let mut without = Vec::new();
        for u in text.encode_utf16() {
            with_bom.extend_from_slice(&u.to_le_bytes());
            without.extend_from_slice(&u.to_le_bytes());
        }
        assert_eq!(decode(&with_bom).text, text);
        let d = decode(&without);
        assert_eq!(d.encoding, "UTF-16LE");
        assert_eq!(d.text, text);
        assert!(!looks_binary(&without));
    }

    #[test]
    fn binary_detection() {
        let mut exe = vec![
            0x4D, 0x5A, 0x90, 0x00, 0x03, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0xFF, 0xFF,
            0x00, 0x00,
        ];
        exe.extend((0..64u8).map(|i| i.wrapping_mul(37)));
        assert!(looks_binary(&exe));
        assert!(!looks_binary(b"hello world"));
        assert!(!looks_binary("Привет, мир!".as_bytes()));
    }

    #[test]
    fn latin_utf16_without_bom() {
        let text = "Plain English text in UTF-16 without a byte order mark.";
        let bytes: Vec<u8> = text.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let d = decode(&bytes);
        assert_eq!(d.encoding, "UTF-16LE");
        assert_eq!(d.text, text);
        let be: Vec<u8> = text.encode_utf16().flat_map(|u| u.to_be_bytes()).collect();
        assert_eq!(decode(&be).encoding, "UTF-16BE");
    }
}
