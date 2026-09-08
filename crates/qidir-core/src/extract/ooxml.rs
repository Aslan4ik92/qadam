//! Office Open XML: DOCX (Word) and PPTX (PowerPoint).
//!
//! XLSX is handled by [`super::spreadsheet`] through `calamine`, which knows
//! about shared strings and cell types.

use std::io::{Cursor, Read};

use zip::ZipArchive;

use super::markup::xml_text;
use crate::error::{Error, Result};

const DOCX_BLOCK_TAGS: &[&str] = &["p", "tr", "tbl", "sectPr"];
const DOCX_SKIP_TAGS: &[&str] = &["instrText", "delText", "del", "moveFrom"];

fn read_entry(zip: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> Option<Vec<u8>> {
    let mut f = zip.by_name(name).ok()?;
    let mut buf = Vec::with_capacity(f.size() as usize);
    f.read_to_end(&mut buf).ok()?;
    Some(buf)
}

fn open(bytes: &[u8]) -> Result<ZipArchive<Cursor<&[u8]>>> {
    ZipArchive::new(Cursor::new(bytes)).map_err(|e| Error::Extract(format!("not a zip container: {e}")))
}

/// Extract text of a DOCX file: main document, headers/footers, footnotes,
/// endnotes and comments are all indexed.
pub fn docx(bytes: &[u8]) -> Result<String> {
    let mut zip = open(bytes)?;
    let names: Vec<String> = zip.file_names().map(|s| s.to_string()).collect();
    let mut parts: Vec<String> = Vec::new();

    // Main body first so previews start with the actual document.
    if let Some(doc) = read_entry(&mut zip, "word/document.xml") {
        parts.push(xml_text(doc.as_slice(), DOCX_BLOCK_TAGS, DOCX_SKIP_TAGS));
    }
    let mut extra: Vec<&String> = names
        .iter()
        .filter(|n| {
            n.starts_with("word/")
                && n.ends_with(".xml")
                && (n.contains("header")
                    || n.contains("footer")
                    || n.ends_with("footnotes.xml")
                    || n.ends_with("endnotes.xml")
                    || n.ends_with("comments.xml"))
        })
        .collect();
    extra.sort();
    for n in extra {
        if let Some(data) = read_entry(&mut zip, n) {
            let t = xml_text(data.as_slice(), DOCX_BLOCK_TAGS, DOCX_SKIP_TAGS);
            if !t.is_empty() {
                parts.push(t);
            }
        }
    }
    if parts.is_empty() {
        return Err(Error::Extract("word/document.xml not found".into()));
    }
    Ok(parts.join("\n\n"))
}

/// Extract text of a PPTX file: slides in order, then notes.
pub fn pptx(bytes: &[u8]) -> Result<String> {
    let mut zip = open(bytes)?;
    let names: Vec<String> = zip.file_names().map(|s| s.to_string()).collect();

    let mut slides: Vec<(u32, String)> = names
        .iter()
        .filter_map(|n| {
            let rest = n.strip_prefix("ppt/slides/slide")?;
            let num: u32 = rest.strip_suffix(".xml")?.parse().ok()?;
            Some((num, n.clone()))
        })
        .collect();
    slides.sort();
    let mut notes: Vec<(u32, String)> = names
        .iter()
        .filter_map(|n| {
            let rest = n.strip_prefix("ppt/notesSlides/notesSlide")?;
            let num: u32 = rest.strip_suffix(".xml")?.parse().ok()?;
            Some((num, n.clone()))
        })
        .collect();
    notes.sort();

    let mut parts = Vec::new();
    for (_, n) in slides.iter().chain(notes.iter()) {
        if let Some(data) = read_entry(&mut zip, n) {
            let t = xml_text(data.as_slice(), &["p", "tr"], &[]);
            if !t.is_empty() {
                parts.push(t);
            }
        }
    }
    if parts.is_empty() && slides.is_empty() {
        return Err(Error::Extract("no slides found".into()));
    }
    Ok(parts.join("\n\n"))
}

/// Test helper: build a minimal OOXML container with the given entries.
#[cfg(test)]
pub(crate) fn build_zip(entries: &[(&str, &str)]) -> Vec<u8> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    let mut buf = Cursor::new(Vec::new());
    {
        let mut w = zip::ZipWriter::new(&mut buf);
        for (name, content) in entries {
            w.start_file(*name, SimpleFileOptions::default()).unwrap();
            w.write_all(content.as_bytes()).unwrap();
        }
        w.finish().unwrap();
    }
    buf.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docx_roundtrip() {
        let doc = r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Договор</w:t></w:r><w:r><w:t xml:space="preserve"> №12</w:t></w:r></w:p><w:p><w:r><w:t>Қазақстан Республикасы</w:t></w:r></w:p><w:p><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText>PAGE</w:instrText></w:r></w:p></w:body></w:document>"#;
        let hdr = r#"<w:hdr xmlns:w="x"><w:p><w:r><w:t>Колонтитул</w:t></w:r></w:p></w:hdr>"#;
        let bytes = build_zip(&[
            ("[Content_Types].xml", "<Types/>"),
            ("word/document.xml", doc),
            ("word/header1.xml", hdr),
        ]);
        let text = docx(&bytes).unwrap();
        assert!(text.starts_with("Договор №12\nҚазақстан Республикасы"), "{text}");
        assert!(text.contains("Колонтитул"));
        assert!(!text.contains("PAGE"));
    }

    #[test]
    fn pptx_slides_in_order() {
        let s = |t: &str| {
            format!(
                r#"<p:sld xmlns:a="a" xmlns:p="p"><p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>{t}</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#
            )
        };
        let s1 = s("Первый слайд");
        let s2 = s("Second slide");
        let s10 = s("Десятый слайд");
        let bytes = build_zip(&[
            ("ppt/slides/slide10.xml", &s10),
            ("ppt/slides/slide2.xml", &s2),
            ("ppt/slides/slide1.xml", &s1),
        ]);
        let text = pptx(&bytes).unwrap();
        assert_eq!(text, "Первый слайд\n\nSecond slide\n\nДесятый слайд");
    }

    #[test]
    fn not_a_zip() {
        assert!(docx(b"garbage").is_err());
    }
}
