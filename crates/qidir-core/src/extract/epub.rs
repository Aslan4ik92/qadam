//! EPUB e-books: every XHTML document inside the container, in spine order when
//! the OPF manifest can be read, otherwise alphabetically.

use std::io::{Cursor, Read};

use super::markup::html_to_text;
use crate::error::{Error, Result};

pub fn epub(bytes: &[u8]) -> Result<String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| Error::Extract(format!("not a zip container: {e}")))?;
    let names: Vec<String> = zip.file_names().map(|s| s.to_string()).collect();

    let mut ordered: Vec<String> = Vec::new();
    if let Some(opf_name) = names.iter().find(|n| n.ends_with(".opf")) {
        if let Some(opf) = read(&mut zip, opf_name) {
            let base = opf_name
                .rsplit_once('/')
                .map(|(d, _)| format!("{d}/"))
                .unwrap_or_default();
            ordered = spine_order(&String::from_utf8_lossy(&opf))
                .into_iter()
                .map(|href| format!("{base}{href}"))
                .filter(|n| names.contains(n))
                .collect();
        }
    }
    if ordered.is_empty() {
        ordered = names
            .iter()
            .filter(|n| {
                let l = n.to_ascii_lowercase();
                l.ends_with(".xhtml") || l.ends_with(".html") || l.ends_with(".htm")
            })
            .cloned()
            .collect();
        ordered.sort();
    }
    let mut parts = Vec::new();
    for n in ordered {
        if let Some(data) = read(&mut zip, &n) {
            let t = html_to_text(&super::plain::decode(&data).text);
            if !t.is_empty() {
                parts.push(t);
            }
        }
    }
    if parts.is_empty() {
        return Err(Error::Extract("no readable chapters".into()));
    }
    Ok(parts.join("\n\n"))
}

fn read(zip: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str) -> Option<Vec<u8>> {
    let mut f = zip.by_name(name).ok()?;
    let mut buf = Vec::with_capacity(f.size() as usize);
    f.read_to_end(&mut buf).ok()?;
    Some(buf)
}

/// Resolve `<spine>` itemrefs to manifest hrefs.
fn spine_order(opf: &str) -> Vec<String> {
    let mut id_to_href = std::collections::HashMap::new();
    for item in opf.split("<item ").skip(1) {
        let tag_end = item.find('>').unwrap_or(item.len());
        let tag = &item[..tag_end];
        if let (Some(id), Some(href)) = (attr(tag, "id"), attr(tag, "href")) {
            id_to_href.insert(id, href);
        }
    }
    let mut out = Vec::new();
    for itemref in opf.split("<itemref ").skip(1) {
        let tag_end = itemref.find('>').unwrap_or(itemref.len());
        if let Some(idref) = attr(&itemref[..tag_end], "idref") {
            if let Some(href) = id_to_href.get(&idref) {
                out.push(super::markup::decode_entities(href));
            }
        }
    }
    out
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=");
    let pos = tag.find(&needle)?;
    let rest = &tag[pos + needle.len()..];
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let end = rest[1..].find(quote)?;
    Some(rest[1..1 + end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::ooxml::build_zip;

    #[test]
    fn spine_order_respected() {
        let opf = r#"<package><manifest><item id="c2" href="ch2.xhtml" media-type="application/xhtml+xml"/><item id="c1" href="ch1.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="c1"/><itemref idref="c2"/></spine></package>"#;
        let bytes = build_zip(&[
            ("OEBPS/content.opf", opf),
            (
                "OEBPS/ch2.xhtml",
                "<html><body><p>Вторая глава</p></body></html>",
            ),
            (
                "OEBPS/ch1.xhtml",
                "<html><body><p>Первая глава</p></body></html>",
            ),
        ]);
        assert_eq!(epub(&bytes).unwrap(), "Первая глава\n\nВторая глава");
    }
}
