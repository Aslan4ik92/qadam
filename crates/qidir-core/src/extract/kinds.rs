//! File format classification.

use serde::{Deserialize, Serialize};

/// Coarse category shown in the UI and used for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileCategory {
    Document,
    Spreadsheet,
    Presentation,
    Pdf,
    Text,
    Code,
    Ebook,
    Web,
    Data,
    Email,
    Other,
}

impl FileCategory {
    pub const ALL: [FileCategory; 11] = [
        FileCategory::Document,
        FileCategory::Spreadsheet,
        FileCategory::Presentation,
        FileCategory::Pdf,
        FileCategory::Text,
        FileCategory::Code,
        FileCategory::Ebook,
        FileCategory::Web,
        FileCategory::Data,
        FileCategory::Email,
        FileCategory::Other,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            FileCategory::Document => "document",
            FileCategory::Spreadsheet => "spreadsheet",
            FileCategory::Presentation => "presentation",
            FileCategory::Pdf => "pdf",
            FileCategory::Text => "text",
            FileCategory::Code => "code",
            FileCategory::Ebook => "ebook",
            FileCategory::Web => "web",
            FileCategory::Data => "data",
            FileCategory::Email => "email",
            FileCategory::Other => "other",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        FileCategory::ALL.iter().copied().find(|c| c.as_str() == s)
    }
}

/// How the extractor should treat a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extractor {
    PlainText,
    Ooxml,
    Odf,
    Pdf,
    Rtf,
    Html,
    MsWord,
    Spreadsheet,
    Epub,
    Xml,
    /// Known format QIDIR cannot read yet: indexed by file name only.
    NameOnly,
}

/// Classify by lower-case extension (without dot).
pub fn classify(ext: &str) -> (FileCategory, Extractor) {
    match ext {
        // Word processing
        "docx" | "docm" | "dotx" | "dotm" => (FileCategory::Document, Extractor::Ooxml),
        "odt" | "ott" | "fodt" => (FileCategory::Document, Extractor::Odf),
        "doc" | "dot" | "wbk" => (FileCategory::Document, Extractor::MsWord),
        "rtf" => (FileCategory::Document, Extractor::Rtf),
        "pages" | "wpd" | "wps" => (FileCategory::Document, Extractor::NameOnly),
        // Spreadsheets
        "xlsx" | "xlsm" | "xltx" | "xltm" | "xlsb" => (FileCategory::Spreadsheet, Extractor::Spreadsheet),
        "xls" | "xlt" => (FileCategory::Spreadsheet, Extractor::Spreadsheet),
        "ods" | "ots" | "fods" => (FileCategory::Spreadsheet, Extractor::Spreadsheet),
        "csv" | "tsv" => (FileCategory::Spreadsheet, Extractor::PlainText),
        // Presentations
        "pptx" | "pptm" | "potx" | "ppsx" => (FileCategory::Presentation, Extractor::Ooxml),
        "odp" | "otp" | "fodp" => (FileCategory::Presentation, Extractor::Odf),
        "ppt" | "pps" | "pot" => (FileCategory::Presentation, Extractor::NameOnly),
        // PDF
        "pdf" => (FileCategory::Pdf, Extractor::Pdf),
        // E-books
        "epub" => (FileCategory::Ebook, Extractor::Epub),
        "fb2" => (FileCategory::Ebook, Extractor::Xml),
        "mobi" | "azw" | "azw3" | "djvu" | "chm" => (FileCategory::Ebook, Extractor::NameOnly),
        // Web
        "html" | "htm" | "xhtml" | "mht" | "mhtml" | "shtml" => (FileCategory::Web, Extractor::Html),
        // Data / markup
        "xml" | "xsl" | "xslt" | "xsd" | "svg" | "plist" | "kml" | "gpx" | "opml" => {
            (FileCategory::Data, Extractor::Xml)
        }
        "json" | "jsonl" | "ndjson" | "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf" | "properties"
        | "env" | "reg" => (FileCategory::Data, Extractor::PlainText),
        // Email
        "eml" | "msg" | "mbox" => (FileCategory::Email, Extractor::PlainText),
        // Plain text
        "txt" | "text" | "md" | "markdown" | "rst" | "log" | "nfo" | "diz" | "srt" | "sub" | "ass"
        | "vtt" | "tex" | "bib" | "adoc" | "asciidoc" | "org" | "1st" | "readme" | "me" => {
            (FileCategory::Text, Extractor::PlainText)
        }
        // Source code
        "rs" | "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" | "hh" | "cs" | "java" | "kt" | "kts" | "scala"
        | "go" | "py" | "pyw" | "rb" | "php" | "pl" | "pm" | "swift" | "m" | "mm" | "js" | "mjs" | "cjs"
        | "ts" | "tsx" | "jsx" | "vue" | "svelte" | "css" | "scss" | "sass" | "less" | "sql" | "sh"
        | "bash" | "zsh" | "fish" | "ps1" | "psm1" | "bat" | "cmd" | "vbs" | "lua" | "r" | "jl" | "dart"
        | "ex" | "exs" | "erl" | "hs" | "clj" | "lisp" | "el" | "vb" | "pas" | "dpr" | "asm" | "s" | "f"
        | "f90" | "for" | "groovy" | "gradle" | "cmake" | "make" | "mk" | "dockerfile" | "tf" | "proto"
        | "graphql" | "gql" | "1c" | "bsl" | "os" => (FileCategory::Code, Extractor::PlainText),
        _ => (FileCategory::Other, Extractor::NameOnly),
    }
}

/// Every extension QIDIR can extract text from (for the settings UI).
pub fn supported_extensions() -> Vec<&'static str> {
    vec![
        "docx", "docm", "dotx", "odt", "ott", "doc", "dot", "rtf", "xlsx", "xlsm", "xltx", "xlsb", "xls",
        "ods", "ots", "csv", "tsv", "pptx", "pptm", "ppsx", "odp", "pdf", "epub", "fb2", "html", "htm",
        "xhtml", "xml", "svg", "json", "yaml", "yml", "toml", "ini", "cfg", "conf", "eml", "mbox", "txt",
        "md", "rst", "log", "srt", "tex", "rs", "c", "h", "cpp", "cs", "java", "kt", "go", "py", "rb", "php",
        "js", "ts", "css", "sql", "sh", "ps1", "bat", "cmd", "lua", "vb", "pas", "1c", "bsl",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies() {
        assert_eq!(classify("docx"), (FileCategory::Document, Extractor::Ooxml));
        assert_eq!(classify("xls"), (FileCategory::Spreadsheet, Extractor::Spreadsheet));
        assert_eq!(classify("exe"), (FileCategory::Other, Extractor::NameOnly));
        assert_eq!(classify("py").0, FileCategory::Code);
        assert_eq!(FileCategory::parse("pdf"), Some(FileCategory::Pdf));
    }
}
