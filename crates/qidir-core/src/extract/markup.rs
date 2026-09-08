//! Shared helpers for XML/HTML based formats.

use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::BufRead;

/// Extract the character data of an XML document, inserting whitespace at
/// element boundaries so words from adjacent elements never glue together.
///
/// `block_tags` are local names after which a newline (instead of a space) is
/// inserted — used to preserve paragraph structure in previews.
pub fn xml_text<R: BufRead>(reader: R, block_tags: &[&str], skip_tags: &[&str]) -> String {
    let mut xml = Reader::from_reader(reader);
    xml.config_mut().trim_text(false);
    xml.config_mut().check_end_names = false;
    xml.config_mut().expand_empty_elements = false;

    let mut out = String::new();
    let mut buf = Vec::new();
    let mut skip_depth = 0usize;

    loop {
        match xml.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = local_name(e.name().as_ref());
                if skip_depth > 0 {
                    skip_depth += 1;
                } else if skip_tags.iter().any(|t| t.eq_ignore_ascii_case(&name)) {
                    skip_depth = 1;
                }
            }
            Ok(Event::End(e)) => {
                if skip_depth > 0 {
                    skip_depth -= 1;
                    continue;
                }
                let name = local_name(e.name().as_ref());
                if block_tags.iter().any(|t| t.eq_ignore_ascii_case(&name)) {
                    push_sep(&mut out, '\n');
                } else {
                    push_sep(&mut out, ' ');
                }
            }
            Ok(Event::Empty(e)) => {
                if skip_depth > 0 {
                    continue;
                }
                let name = local_name(e.name().as_ref());
                if matches!(name.as_str(), "br" | "tab" | "cr" | "line-break" | "s") {
                    push_sep(
                        &mut out,
                        if name == "br" || name == "cr" || name == "line-break" {
                            '\n'
                        } else {
                            ' '
                        },
                    );
                } else if block_tags.iter().any(|t| t.eq_ignore_ascii_case(&name)) {
                    push_sep(&mut out, '\n');
                }
            }
            Ok(Event::Text(t)) => {
                if skip_depth > 0 {
                    continue;
                }
                match t.unescape() {
                    Ok(s) => out.push_str(&s),
                    Err(_) => out.push_str(&String::from_utf8_lossy(t.as_ref())),
                }
            }
            Ok(Event::CData(c)) => {
                if skip_depth == 0 {
                    out.push_str(&String::from_utf8_lossy(c.as_ref()));
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => break, // tolerate malformed markup: keep what we have
        }
        buf.clear();
    }
    collapse_whitespace(&out)
}

fn local_name(name: &[u8]) -> String {
    let s = String::from_utf8_lossy(name);
    match s.rfind(':') {
        Some(i) => s[i + 1..].to_string(),
        None => s.into_owned(),
    }
}

fn push_sep(out: &mut String, sep: char) {
    match out.chars().last() {
        None => {}
        Some('\n') => {}
        Some(' ') if sep == '\n' => {
            out.pop();
            out.push('\n');
        }
        Some(' ') => {}
        Some(_) => out.push(sep),
    }
}

/// Collapse runs of spaces/tabs and limit consecutive newlines to two.
pub fn collapse_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    let mut newlines = 0u8;
    for c in s.chars() {
        match c {
            '\n' | '\r' => {
                if newlines < 2 {
                    if prev_space && out.ends_with(' ') {
                        out.pop();
                    }
                    out.push('\n');
                    newlines += 1;
                }
                prev_space = false;
            }
            c if c.is_whitespace() || c == '\u{00A0}' => {
                if !prev_space && newlines == 0 {
                    out.push(' ');
                }
                prev_space = true;
            }
            c => {
                out.push(c);
                prev_space = false;
                newlines = 0;
            }
        }
    }
    out.trim().to_string()
}

/// Very tolerant HTML → text conversion (HTML is not XML: unclosed tags,
/// unquoted attributes and entities are common).
pub fn html_to_text(html: &str) -> String {
    let lower_tags_block = [
        "p",
        "div",
        "br",
        "li",
        "ul",
        "ol",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "tr",
        "table",
        "section",
        "article",
        "header",
        "footer",
        "blockquote",
        "pre",
        "hr",
        "title",
        "dd",
        "dt",
    ];
    let mut out = String::with_capacity(html.len() / 2);
    let bytes = html.as_bytes();
    let mut i = 0;
    let mut skip_until: Option<&str> = None;
    while i < html.len() {
        if bytes[i] == b'<' {
            // Comment
            if html[i..].starts_with("<!--") {
                match html[i + 4..].find("-->") {
                    Some(e) => {
                        i = i + 4 + e + 3;
                        continue;
                    }
                    None => break,
                }
            }
            let end = match html[i..].find('>') {
                Some(e) => i + e,
                None => break,
            };
            let tag_body = &html[i + 1..end];
            let closing = tag_body.starts_with('/');
            let name: String = tag_body
                .trim_start_matches('/')
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase();
            if let Some(until) = skip_until {
                if closing && name == until {
                    skip_until = None;
                }
                i = end + 1;
                continue;
            }
            if !closing && matches!(name.as_str(), "script" | "style" | "noscript" | "template") {
                skip_until = Some(match name.as_str() {
                    "script" => "script",
                    "style" => "style",
                    "noscript" => "noscript",
                    _ => "template",
                });
            } else if lower_tags_block.contains(&name.as_str()) {
                push_sep(&mut out, '\n');
            } else if !name.is_empty() {
                push_sep(&mut out, ' ');
            }
            i = end + 1;
        } else {
            if skip_until.is_none() {
                let next = html[i..].find('<').map(|n| i + n).unwrap_or(html.len());
                out.push_str(&decode_entities(&html[i..next]));
                i = next;
            } else {
                let next = html[i..].find('<').map(|n| i + n).unwrap_or(html.len());
                i = next;
            }
        }
    }
    collapse_whitespace(&out)
}

/// Decode the entities that actually occur in documents.
pub fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(pos) = rest.find('&') {
        out.push_str(&rest[..pos]);
        rest = &rest[pos..];
        let semi = rest.find(';');
        match semi {
            Some(end) if end <= 12 => {
                let ent = &rest[1..end];
                let decoded: Option<String> = if let Some(num) = ent.strip_prefix('#') {
                    let code = if let Some(hex) = num.strip_prefix(['x', 'X']) {
                        u32::from_str_radix(hex, 16).ok()
                    } else {
                        num.parse::<u32>().ok()
                    };
                    code.and_then(char::from_u32).map(|c| c.to_string())
                } else {
                    named_entity(ent).map(|c| c.to_string())
                };
                match decoded {
                    Some(d) => {
                        out.push_str(&d);
                        rest = &rest[end + 1..];
                    }
                    None => {
                        out.push('&');
                        rest = &rest[1..];
                    }
                }
            }
            _ => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn named_entity(name: &str) -> Option<char> {
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => ' ',
        "laquo" => '«',
        "raquo" => '»',
        "ndash" => '–',
        "mdash" => '—',
        "hellip" => '…',
        "copy" => '©',
        "reg" => '®',
        "trade" => '™',
        "deg" => '°',
        "middot" => '·',
        "bull" => '•',
        "lsquo" => '‘',
        "rsquo" => '’',
        "ldquo" => '“',
        "rdquo" => '”',
        "bdquo" => '„',
        "euro" => '€',
        "sect" => '§',
        "para" => '¶',
        "shy" => '\u{00AD}',
        "times" => '×',
        "minus" => '−',
        "plusmn" => '±',
        "frac12" => '½',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_text_extracts_with_separators() {
        let xml = r#"<?xml version="1.0"?><w:document xmlns:w="x"><w:body><w:p><w:r><w:t>Привет</w:t></w:r><w:r><w:t xml:space="preserve"> мир</w:t></w:r></w:p><w:p><w:r><w:t>Вторая строка</w:t></w:r></w:p></w:body></w:document>"#;
        let t = xml_text(xml.as_bytes(), &["p"], &[]);
        assert_eq!(t, "Привет мир\nВторая строка");
    }

    #[test]
    fn html_strips_scripts_and_decodes_entities() {
        let html = r#"<html><head><title>Заголовок</title><style>p{color:red}</style><script>var x = "<p>";</script></head>
        <body><h1>Тест &amp; проверка</h1><p>Первый&nbsp;абзац &laquo;кавычки&raquo; &#1179;&#x430;&#x437;&#x430;&#x49b;</p><p>Второй<br>абзац</p></body></html>"#;
        let t = html_to_text(html);
        assert!(t.starts_with("Заголовок"), "{t}");
        assert!(t.contains("Тест & проверка"));
        assert!(t.contains("«кавычки»"));
        assert!(t.contains("қазақ"));
        assert!(!t.contains("color:red"));
        assert!(!t.contains("var x"));
        assert!(t.contains("Второй\nабзац"));
    }

    #[test]
    fn collapse() {
        assert_eq!(collapse_whitespace("  a \t b \n\n\n\n c  "), "a b\n\nc");
    }
}
