//! OpenDocument text and presentation formats (ODT / ODP / flat XML variants).

use std::io::{Cursor, Read};

use super::markup::xml_text;
use crate::error::{Error, Result};

const ODF_BLOCK_TAGS: &[&str] = &["p", "h", "table-row", "list-item"];
const ODF_SKIP_TAGS: &[&str] = &["tracked-changes", "annotation", "note-citation"];

/// Extract text from an ODF package (`content.xml`) or a flat ODF XML file.
pub fn odf(bytes: &[u8]) -> Result<String> {
    if bytes.starts_with(b"PK") {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes))
            .map_err(|e| Error::Extract(format!("not a zip container: {e}")))?;
        let mut content =
            zip.by_name("content.xml").map_err(|_| Error::Extract("content.xml not found".into()))?;
        let mut buf = Vec::with_capacity(content.size() as usize);
        content.read_to_end(&mut buf).map_err(|e| Error::Extract(e.to_string()))?;
        Ok(xml_text(buf.as_slice(), ODF_BLOCK_TAGS, ODF_SKIP_TAGS))
    } else {
        // Flat ODF (.fodt / .fodp) is a single XML document.
        Ok(xml_text(bytes, ODF_BLOCK_TAGS, ODF_SKIP_TAGS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::ooxml::build_zip;

    #[test]
    fn odt() {
        let content = r#"<?xml version="1.0"?><office:document-content xmlns:office="o" xmlns:text="t"><office:body><office:text><text:h>Заголовок</text:h><text:p>Абзац <text:span>с текстом</text:span>.</text:p><text:p>Екінші абзац</text:p></office:text></office:body></office:document-content>"#;
        let bytes =
            build_zip(&[("mimetype", "application/vnd.oasis.opendocument.text"), ("content.xml", content)]);
        let t = odf(&bytes).unwrap();
        assert_eq!(t, "Заголовок\nАбзац с текстом .\nЕкінші абзац");
    }
}
