//! Legacy binary Microsoft Word (`.doc`, Word 6/95/97–2003) via the OLE2
//! compound file format.
//!
//! Word 97–2003 keeps text in the `WordDocument` stream, addressed through a
//! *piece table* stored in the `0Table`/`1Table` stream. Each piece is either
//! UTF-16LE or "compressed" 8-bit text. Word 6/95 files have no reliable piece
//! table in the same place, so for them the raw text range `fcMin..fcMac` is
//! decoded with encoding detection — which works well for Russian cp1251 files.

use std::io::{Cursor, Read};

use crate::error::{Error, Result};

const FC_CLX: usize = 0x01A2;
const LCB_CLX: usize = 0x01A6;

pub fn doc(bytes: &[u8]) -> Result<String> {
    let mut cfb = cfb::CompoundFile::open(Cursor::new(bytes))
        .map_err(|e| Error::Extract(format!("not an OLE2 file: {e}")))?;
    let mut word = Vec::new();
    cfb.open_stream("/WordDocument")
        .map_err(|_| Error::Extract("WordDocument stream not found".into()))?
        .read_to_end(&mut word)
        .map_err(|e| Error::Extract(e.to_string()))?;
    if word.len() < 0x20 {
        return Err(Error::Extract("WordDocument stream too short".into()));
    }
    let ident = u16le(&word, 0);
    if ident != 0xA5EC && ident != 0xA5DC {
        return Err(Error::Extract(format!("unexpected wIdent {ident:#x}")));
    }
    let n_fib = u16le(&word, 2);
    let flags = u16le(&word, 0x0A);
    let encrypted = flags & 0x0100 != 0;
    if encrypted {
        return Err(Error::Extract("document is encrypted".into()));
    }
    let which_table = if flags & 0x0200 != 0 { "/1Table" } else { "/0Table" };

    if n_fib >= 0x00C1 && word.len() > LCB_CLX + 4 {
        if let Ok(text) = piece_table_text(&mut cfb, &word, which_table) {
            if !text.trim().is_empty() {
                return Ok(clean(&text));
            }
        }
    }
    // Word 6/95 or piece table unusable: raw text range.
    let fc_min = u32le(&word, 0x18) as usize;
    let fc_mac = u32le(&word, 0x1C) as usize;
    if fc_min < fc_mac && fc_mac <= word.len() {
        let raw = &word[fc_min..fc_mac];
        let decoded = super::plain::decode(raw);
        return Ok(clean(&decoded.text));
    }
    Err(Error::Extract("cannot locate document text".into()))
}

fn piece_table_text(
    cfb: &mut cfb::CompoundFile<Cursor<&[u8]>>,
    word: &[u8],
    table_name: &str,
) -> Result<String> {
    let fc_clx = u32le(word, FC_CLX) as usize;
    let lcb_clx = u32le(word, LCB_CLX) as usize;
    let mut table = Vec::new();
    cfb.open_stream(table_name)
        .map_err(|_| Error::Extract("table stream not found".into()))?
        .read_to_end(&mut table)
        .map_err(|e| Error::Extract(e.to_string()))?;
    if fc_clx + lcb_clx > table.len() || lcb_clx == 0 {
        return Err(Error::Extract("clx out of range".into()));
    }
    let clx = &table[fc_clx..fc_clx + lcb_clx];

    // Skip Prc entries (clxt = 0x01) until the Pcdt (clxt = 0x02).
    let mut pos = 0usize;
    while pos < clx.len() {
        match clx[pos] {
            0x01 => {
                if pos + 3 > clx.len() {
                    return Err(Error::Extract("truncated prc".into()));
                }
                let cb = u16le(clx, pos + 1) as usize;
                pos += 3 + cb;
            }
            0x02 => {
                pos += 1;
                break;
            }
            _ => return Err(Error::Extract("bad clx".into())),
        }
    }
    if pos + 4 > clx.len() {
        return Err(Error::Extract("missing pcdt".into()));
    }
    let lcb = u32le(clx, pos) as usize;
    pos += 4;
    if pos + lcb > clx.len() || lcb < 4 {
        return Err(Error::Extract("bad plcpcd".into()));
    }
    let plc = &clx[pos..pos + lcb];
    let n = (lcb - 4) / 12;
    let mut text = String::new();
    for k in 0..n {
        let cp_start = u32le(plc, k * 4) as usize;
        let cp_end = u32le(plc, (k + 1) * 4) as usize;
        if cp_end < cp_start {
            continue;
        }
        let pcd_off = (n + 1) * 4 + k * 8;
        let fc_raw = u32le(plc, pcd_off + 2);
        let compressed = fc_raw & 0x4000_0000 != 0;
        let fc = (fc_raw & 0x3FFF_FFFF) as usize;
        let len_cp = cp_end - cp_start;
        if compressed {
            let start = fc / 2;
            let end = (start + len_cp).min(word.len());
            if start >= end {
                continue;
            }
            let (s, _, _) = encoding_rs::WINDOWS_1252.decode(&word[start..end]);
            text.push_str(&s);
        } else {
            let start = fc;
            let end = (start + len_cp * 2).min(word.len());
            if start >= end {
                continue;
            }
            let (s, _) = encoding_rs::UTF_16LE.decode_without_bom_handling(&word[start..end]);
            text.push_str(&s);
        }
    }
    Ok(text)
}

/// Replace Word control characters with their textual meaning and drop field
/// instructions (`HYPERLINK "..."`).
fn clean(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_field_code = false;
    for c in raw.chars() {
        match c {
            '\u{13}' => in_field_code = true,  // field begin
            '\u{14}' => in_field_code = false, // field separator: result follows
            '\u{15}' => in_field_code = false, // field end
            _ if in_field_code => {}
            '\r' | '\u{0B}' | '\u{0C}' => out.push('\n'),
            '\u{07}' => out.push('\t'),
            '\u{01}' | '\u{02}' | '\u{05}' | '\u{08}' => {}
            c if (c as u32) < 0x20 && c != '\n' && c != '\t' => {}
            _ => out.push(c),
        }
    }
    super::markup::collapse_whitespace(&out)
}

#[inline]
fn u16le(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([b[off], b[off + 1]])
}
#[inline]
fn u32le(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Build a synthetic Word 97 file: FIB + one UTF-16 piece + one cp1252 piece.
    fn synthetic_doc(unicode_text: &str, ansi_text: &str) -> Vec<u8> {
        // --- WordDocument stream ---
        let mut word = vec![0u8; 0x200];
        word[0..2].copy_from_slice(&0xA5ECu16.to_le_bytes());
        word[2..4].copy_from_slice(&0x00C1u16.to_le_bytes());
        word[0x0A..0x0C].copy_from_slice(&0x0200u16.to_le_bytes()); // 1Table
        let text_start = word.len();
        let mut utf16 = Vec::new();
        for u in unicode_text.encode_utf16() {
            utf16.extend_from_slice(&u.to_le_bytes());
        }
        word.extend_from_slice(&utf16);
        let ansi_start = word.len();
        word.extend_from_slice(ansi_text.as_bytes());
        // --- Piece table in 1Table ---
        let cp1 = unicode_text.encode_utf16().count() as u32;
        let cp2 = cp1 + ansi_text.len() as u32;
        let mut plc = Vec::new();
        plc.extend_from_slice(&0u32.to_le_bytes());
        plc.extend_from_slice(&cp1.to_le_bytes());
        plc.extend_from_slice(&cp2.to_le_bytes());
        // pcd 1: unicode
        plc.extend_from_slice(&0u16.to_le_bytes());
        plc.extend_from_slice(&(text_start as u32).to_le_bytes());
        plc.extend_from_slice(&0u16.to_le_bytes());
        // pcd 2: compressed → fc = byte offset * 2 with bit 30 set
        plc.extend_from_slice(&0u16.to_le_bytes());
        plc.extend_from_slice(&(((ansi_start * 2) as u32) | 0x4000_0000).to_le_bytes());
        plc.extend_from_slice(&0u16.to_le_bytes());
        let mut clx = vec![0x02u8];
        clx.extend_from_slice(&(plc.len() as u32).to_le_bytes());
        clx.extend_from_slice(&plc);
        let mut table = vec![0u8; 16];
        let fc_clx = table.len() as u32;
        table.extend_from_slice(&clx);
        word[FC_CLX..FC_CLX + 4].copy_from_slice(&fc_clx.to_le_bytes());
        word[LCB_CLX..LCB_CLX + 4].copy_from_slice(&(clx.len() as u32).to_le_bytes());

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut comp = cfb::CompoundFile::create(&mut cursor).unwrap();
            comp.create_stream("/WordDocument").unwrap().write_all(&word).unwrap();
            comp.create_stream("/1Table").unwrap().write_all(&table).unwrap();
            comp.flush().unwrap();
        }
        cursor.into_inner()
    }

    #[test]
    fn reads_piece_table() {
        let bytes = synthetic_doc(
            "Қазақша мәтін.\rВторой абзац\u{13}HYPERLINK\u{14}ссылка\u{15}",
            " Plain ANSI tail",
        );
        let t = doc(&bytes).unwrap();
        assert_eq!(t, "Қазақша мәтін.\nВторой абзацссылка Plain ANSI tail");
    }

    #[test]
    fn rejects_non_ole() {
        assert!(doc(b"not ole").is_err());
    }
}
