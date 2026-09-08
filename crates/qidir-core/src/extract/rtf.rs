//! Rich Text Format → plain text.
//!
//! A small hand-written RTF reader that understands the parts that matter for
//! indexing: groups, control words, hex-escaped bytes in the document code
//! page (`\'e0`), Unicode escapes (`\u1079?`) with `\ucN` skipping, and the
//! destinations that must be ignored (font tables, pictures, metadata, ...).
//! Russian RTFs produced by Word 97–2003 and by 1C use `\ansicpg1251` with hex
//! escapes, so code page handling is essential.

use encoding_rs::Encoding;

const SKIP_DESTINATIONS: &[&str] = &[
    "fonttbl",
    "colortbl",
    "stylesheet",
    "info",
    "pict",
    "object",
    "listtable",
    "listoverridetable",
    "revtbl",
    "rsidtbl",
    "generator",
    "themedata",
    "colorschememapping",
    "datastore",
    "xmlnstbl",
    "latentstyles",
    "fldinst",
    "header",
    "footer",
    "headerf",
    "footerf",
    "headerl",
    "footerl",
    "headerr",
    "footerr",
    "ftnsep",
    "ftnsepc",
    "aftnsep",
    "aftnsepc",
    "mmathPr",
    "bkmkstart",
    "bkmkend",
    "shpinst",
    "sp",
    "sn",
    "sv",
    "template",
    "pntext",
    "pntxta",
    "pntxtb",
    "fchars",
    "lchars",
    "panose",
    "falt",
    "wgrffmtfilter",
    "passwordhash",
    "protusertbl",
    "userprops",
    "background",
    "docvar",
    "atrfstart",
    "atrfend",
    "atnid",
    "atnauthor",
    "atndate",
    "annotation",
];

struct Group {
    skip: bool,
    uc: usize,
}

pub fn rtf(bytes: &[u8]) -> String {
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len() / 2);
    // Text is produced as UTF-8 directly; hex bytes are decoded in chunks.
    let mut text = String::with_capacity(bytes.len() / 2);
    let mut hex_buf: Vec<u8> = Vec::new();
    let mut codepage: &'static Encoding = encoding_rs::WINDOWS_1252;
    let mut stack: Vec<Group> = vec![Group { skip: false, uc: 1 }];
    let mut i = 0usize;
    let mut skip_after_unicode = 0usize;
    let mut first_in_group = false;

    let flush_hex = |hex_buf: &mut Vec<u8>, text: &mut String, cp: &'static Encoding| {
        if !hex_buf.is_empty() {
            let (s, _, _) = cp.decode(hex_buf);
            text.push_str(&s);
            hex_buf.clear();
        }
    };

    while i < bytes.len() {
        let b = bytes[i];
        let cur_skip = stack.last().map(|g| g.skip).unwrap_or(false);
        match b {
            b'{' => {
                flush_hex(&mut hex_buf, &mut text, codepage);
                let parent = stack.last().map(|g| (g.skip, g.uc)).unwrap_or((false, 1));
                stack.push(Group { skip: parent.0, uc: parent.1 });
                first_in_group = true;
                i += 1;
            }
            b'}' => {
                flush_hex(&mut hex_buf, &mut text, codepage);
                stack.pop();
                if stack.is_empty() {
                    break;
                }
                first_in_group = false;
                i += 1;
            }
            b'\\' => {
                i += 1;
                if i >= bytes.len() {
                    break;
                }
                let c = bytes[i];
                if c == b'\'' {
                    // Hex escaped byte.
                    if i + 2 < bytes.len() + 1 && i + 2 <= bytes.len() - 1 + 1 {
                        let hex = &bytes[i + 1..(i + 3).min(bytes.len())];
                        if hex.len() == 2 {
                            if let Ok(v) = u8::from_str_radix(std::str::from_utf8(hex).unwrap_or("zz"), 16) {
                                if skip_after_unicode > 0 {
                                    skip_after_unicode -= 1;
                                } else if !cur_skip {
                                    hex_buf.push(v);
                                }
                            }
                        }
                        i += 3;
                    } else {
                        i += 1;
                    }
                    first_in_group = false;
                    continue;
                }
                if c == b'*' {
                    // `\*\dest` — unknown destination: skip unless we know it.
                    if let Some(g) = stack.last_mut() {
                        g.skip = true;
                    }
                    i += 1;
                    first_in_group = false;
                    continue;
                }
                if !c.is_ascii_alphabetic() {
                    // Control symbol.
                    flush_hex(&mut hex_buf, &mut text, codepage);
                    if !cur_skip {
                        match c {
                            b'~' => text.push(' '),
                            b'-' | b'_' => {}
                            b'\\' | b'{' | b'}' => text.push(c as char),
                            b'\n' | b'\r' => text.push('\n'),
                            _ => {}
                        }
                    }
                    i += 1;
                    first_in_group = false;
                    continue;
                }
                // Control word.
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let word = std::str::from_utf8(&bytes[start..i]).unwrap_or("");
                let mut param: Option<i64> = None;
                let pstart = i;
                if i < bytes.len() && (bytes[i] == b'-' || bytes[i].is_ascii_digit()) {
                    i += 1;
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                    param = std::str::from_utf8(&bytes[pstart..i]).ok().and_then(|s| s.parse().ok());
                }
                // A single space after a control word is part of it.
                if i < bytes.len() && bytes[i] == b' ' {
                    i += 1;
                }

                let was_first = first_in_group;
                first_in_group = false;

                if SKIP_DESTINATIONS.contains(&word) && was_first {
                    if let Some(g) = stack.last_mut() {
                        g.skip = true;
                    }
                    continue;
                }
                match word {
                    "ansicpg" => {
                        if let Some(cp) = param {
                            codepage = codepage_encoding(cp).unwrap_or(codepage);
                        }
                    }
                    "uc" => {
                        if let (Some(n), Some(g)) = (param, stack.last_mut()) {
                            g.uc = n.max(0) as usize;
                        }
                    }
                    "u" => {
                        flush_hex(&mut hex_buf, &mut text, codepage);
                        if let Some(code) = param {
                            let code = if code < 0 { code + 65536 } else { code };
                            if !cur_skip {
                                if let Some(ch) = char::from_u32(code as u32) {
                                    text.push(ch);
                                }
                            }
                            skip_after_unicode = stack.last().map(|g| g.uc).unwrap_or(1);
                        }
                    }
                    "par" | "line" | "sect" | "page" | "row" => {
                        flush_hex(&mut hex_buf, &mut text, codepage);
                        if !cur_skip {
                            text.push('\n');
                        }
                    }
                    "cell" | "tab" | "nestcell" => {
                        flush_hex(&mut hex_buf, &mut text, codepage);
                        if !cur_skip {
                            text.push('\t');
                        }
                    }
                    "emdash" => push_if(&mut text, cur_skip, '—'),
                    "endash" => push_if(&mut text, cur_skip, '–'),
                    "bullet" => push_if(&mut text, cur_skip, '•'),
                    "lquote" => push_if(&mut text, cur_skip, '‘'),
                    "rquote" => push_if(&mut text, cur_skip, '’'),
                    "ldblquote" => push_if(&mut text, cur_skip, '“'),
                    "rdblquote" => push_if(&mut text, cur_skip, '”'),
                    "emspace" | "enspace" | "qmspace" => push_if(&mut text, cur_skip, ' '),
                    "bin" => {
                        // Binary data of `param` bytes follows: skip it.
                        if let Some(n) = param {
                            i = (i + n.max(0) as usize).min(bytes.len());
                        }
                    }
                    _ => {}
                }
            }
            b'\r' | b'\n' => {
                i += 1;
            }
            _ => {
                if skip_after_unicode > 0 {
                    skip_after_unicode -= 1;
                } else if !cur_skip {
                    if b < 0x80 {
                        flush_hex(&mut hex_buf, &mut text, codepage);
                        text.push(b as char);
                    } else {
                        hex_buf.push(b);
                    }
                }
                first_in_group = false;
                i += 1;
            }
        }
    }
    flush_hex(&mut hex_buf, &mut text, codepage);
    out.clear();
    super::markup::collapse_whitespace(&text)
}

fn push_if(text: &mut String, skip: bool, c: char) {
    if !skip {
        text.push(c);
    }
}

/// Map a Windows code page number to an encoding.
pub fn codepage_encoding(cp: i64) -> Option<&'static Encoding> {
    Some(match cp {
        1250 => encoding_rs::WINDOWS_1250,
        1251 => encoding_rs::WINDOWS_1251,
        1252 => encoding_rs::WINDOWS_1252,
        1253 => encoding_rs::WINDOWS_1253,
        1254 => encoding_rs::WINDOWS_1254,
        1255 => encoding_rs::WINDOWS_1255,
        1256 => encoding_rs::WINDOWS_1256,
        1257 => encoding_rs::WINDOWS_1257,
        1258 => encoding_rs::WINDOWS_1258,
        866 => encoding_rs::IBM866,
        20866 => encoding_rs::KOI8_R,
        21866 => encoding_rs::KOI8_U,
        28595 => encoding_rs::ISO_8859_5,
        932 => encoding_rs::SHIFT_JIS,
        936 => encoding_rs::GBK,
        949 => encoding_rs::EUC_KR,
        950 => encoding_rs::BIG5,
        65001 => encoding_rs::UTF_8,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cp1251_hex_escapes() {
        // "Привет" in cp1251 hex escapes + a font table that must be ignored.
        let src = br"{\rtf1\ansi\ansicpg1251\deff0{\fonttbl{\f0\fnil\fcharset204 Times New Roman;}}\pard\f0\fs24 \'cf\'f0\'e8\'e2\'e5\'f2, world!\par \'ca\'e0\'e7\'e0\'ea\'f1\'f2\'e0\'ed\par}";
        let t = rtf(src);
        assert_eq!(t, "Привет, world!\nКазакстан");
    }

    #[test]
    fn unicode_escapes_with_fallback_skipping() {
        let src = br"{\rtf1\ansi\uc1 \u1179?\u1072?\u1079?\u1072?\u1179? \u1090?\u1110?\u1083?\u1110?\par}";
        assert_eq!(rtf(src), "қазақ тілі");
    }

    #[test]
    fn skips_pictures_and_info() {
        let src = br"{\rtf1\ansi{\info{\title Secret title}{\author Bob}}{\*\generator Riched20}Visible text{\pict\wmetafile8 0102030405}\par}";
        assert_eq!(rtf(src), "Visible text");
    }
}
