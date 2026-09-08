//! PDF text extraction via `pdf-extract`.
//!
//! The underlying library can panic on exotic files, so extraction is wrapped
//! in `catch_unwind` — a broken PDF must never take the indexer down.

use crate::error::{Error, Result};

pub fn pdf(bytes: &[u8]) -> Result<String> {
    let result = std::panic::catch_unwind(|| pdf_extract::extract_text_from_mem(bytes));
    match result {
        Ok(Ok(text)) => Ok(super::markup::collapse_whitespace(&text)),
        Ok(Err(e)) => Err(Error::Extract(format!("pdf: {e}"))),
        Err(_) => Err(Error::Extract("pdf: extractor panicked".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a tiny valid single-page PDF with ASCII text and standard fonts.
    fn minimal_pdf(text: &str) -> Vec<u8> {
        let content = format!("BT /F1 24 Tf 72 700 Td ({text}) Tj ET");
        let mut objs: Vec<String> = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".into(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".into(),
            format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into(),
        ];
        let mut out = String::from("%PDF-1.4\n");
        let mut offsets = Vec::new();
        for (i, o) in objs.drain(..).enumerate() {
            offsets.push(out.len());
            out.push_str(&format!("{} 0 obj\n{}\nendobj\n", i + 1, o));
        }
        let xref = out.len();
        out.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1));
        for off in offsets {
            out.push_str(&format!("{off:010} 00000 n \n"));
        }
        out.push_str(&format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", 6, xref));
        out.into_bytes()
    }

    #[test]
    fn extracts_text() {
        let bytes = minimal_pdf("Hello QIDIR search");
        let t = pdf(&bytes).unwrap();
        assert!(t.contains("Hello QIDIR search"), "{t:?}");
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(pdf(b"%PDF-1.4 garbage").is_err());
    }
}
