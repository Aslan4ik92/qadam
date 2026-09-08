//! Text extraction from files.
//!
//! [`extract_file`] is the single entry point used by the indexer. It reads the
//! file, dispatches on the extension and returns normalized plain text. Every
//! extractor is defensive: a corrupted document yields an error (the file is
//! then indexed by name only) and never a panic.

pub mod epub;
pub mod kinds;
pub mod markup;
pub mod msword;
pub mod odf;
pub mod ooxml;
pub mod pdf;
pub mod plain;
pub mod rtf;
pub mod spreadsheet;

use std::path::Path;

pub use kinds::{classify, supported_extensions, Extractor, FileCategory};

use crate::error::{Error, Result};

/// Output of a successful extraction.
#[derive(Debug, Clone)]
pub struct Extracted {
    pub text: String,
    /// Text encoding detected for plain-text formats.
    pub encoding: Option<&'static str>,
    pub category: FileCategory,
}

/// Options controlling extraction.
#[derive(Debug, Clone, Copy)]
pub struct ExtractOptions {
    /// Files larger than this are indexed by name only.
    pub max_file_size: u64,
    /// Whether PDF text extraction is enabled.
    pub extract_pdf: bool,
    /// Hard cap on extracted text length (chars) to keep the index bounded.
    pub max_text_chars: usize,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            max_file_size: 64 * 1024 * 1024,
            extract_pdf: true,
            max_text_chars: 5_000_000,
        }
    }
}

/// Extract text from in-memory bytes for the given lower-case extension.
pub fn extract_bytes(bytes: &[u8], ext: &str, opts: &ExtractOptions) -> Result<Extracted> {
    let (category, extractor) = classify(ext);
    let mut encoding = None;
    let text = match extractor {
        Extractor::PlainText => {
            if plain::looks_binary(bytes) {
                return Err(Error::Extract("binary content".into()));
            }
            let d = plain::decode(bytes);
            encoding = Some(d.encoding);
            markup::collapse_whitespace(&d.text)
        }
        Extractor::Ooxml => match ext {
            "pptx" | "pptm" | "potx" | "ppsx" => ooxml::pptx(bytes)?,
            _ => ooxml::docx(bytes)?,
        },
        Extractor::Odf => odf::odf(bytes)?,
        Extractor::Spreadsheet => spreadsheet::spreadsheet(bytes, ext)?,
        Extractor::Pdf => {
            if !opts.extract_pdf {
                return Err(Error::Unsupported("pdf extraction disabled".into()));
            }
            pdf::pdf(bytes)?
        }
        Extractor::Rtf => rtf::rtf(bytes),
        Extractor::Html => {
            let d = plain::decode(bytes);
            encoding = Some(d.encoding);
            markup::html_to_text(&d.text)
        }
        Extractor::MsWord => msword::doc(bytes)?,
        Extractor::Epub => epub::epub(bytes)?,
        Extractor::Xml => {
            let d = plain::decode(bytes);
            encoding = Some(d.encoding);
            markup::xml_text(
                d.text.as_bytes(),
                &[
                    "p",
                    "v",
                    "title",
                    "subtitle",
                    "section",
                    "item",
                    "entry",
                    "row",
                    "tr",
                    "li",
                    "h1",
                    "h2",
                    "h3",
                    "div",
                    "text",
                    "stanza",
                    "epigraph",
                    "description",
                    "summary",
                ],
                &["binary", "style", "script", "coverpage"],
            )
        }
        Extractor::NameOnly => return Err(Error::Unsupported(ext.to_string())),
    };
    let text = truncate_chars(text, opts.max_text_chars);
    Ok(Extracted {
        text,
        encoding,
        category,
    })
}

/// Read and extract a file from disk. Returns `Ok(None)` when the file is a
/// known-but-unsupported type or exceeds the size limit (index by name only).
pub fn extract_file(
    path: &Path,
    ext: &str,
    size: u64,
    opts: &ExtractOptions,
) -> Result<Option<Extracted>> {
    let (_, extractor) = classify(ext);
    if extractor == Extractor::NameOnly {
        return Ok(None);
    }
    if size > opts.max_file_size {
        return Ok(None);
    }
    if ext == "pdf" && !opts.extract_pdf {
        return Ok(None);
    }
    let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
    match extract_bytes(&bytes, ext, opts) {
        Ok(e) => Ok(Some(e)),
        Err(Error::Unsupported(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

fn truncate_chars(mut s: String, max: usize) -> String {
    if s.chars().count() > max {
        let cut = s.char_indices().nth(max).map(|(i, _)| i).unwrap_or(s.len());
        s.truncate(cut);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatches_plain_text() {
        let e = extract_bytes("Привет".as_bytes(), "txt", &ExtractOptions::default()).unwrap();
        assert_eq!(e.text, "Привет");
        assert_eq!(e.encoding, Some("UTF-8"));
        assert_eq!(e.category, FileCategory::Text);
    }

    #[test]
    fn fb2_via_xml() {
        let fb2 = r#"<?xml version="1.0" encoding="utf-8"?><FictionBook><description><title-info><book-title>Абай жолы</book-title></title-info></description><body><section><title><p>Бірінші бөлім</p></title><p>Мәтін.</p></section></body><binary id="c">AAAA</binary></FictionBook>"#;
        let e = extract_bytes(fb2.as_bytes(), "fb2", &ExtractOptions::default()).unwrap();
        assert!(e.text.contains("Абай жолы"));
        assert!(e.text.contains("Мәтін."));
        assert!(!e.text.contains("AAAA"));
    }

    #[test]
    fn unknown_ext_is_name_only() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.exe");
        std::fs::write(&p, b"MZ\0\0").unwrap();
        assert!(extract_file(&p, "exe", 4, &ExtractOptions::default())
            .unwrap()
            .is_none());
    }

    #[test]
    fn binary_disguised_as_txt_is_error() {
        assert!(extract_bytes(
            &[0u8, 1, 2, 3, 0, 0, 0, 5],
            "txt",
            &ExtractOptions::default()
        )
        .is_err());
    }
}
