//! tantivy schema of the QIDIR index.
//!
//! | field           | type          | purpose                                          |
//! |-----------------|---------------|--------------------------------------------------|
//! | `path`          | string, stored| document identity, used for updates/deletes      |
//! | `path_text`     | text (exact)  | search inside folder names                       |
//! | `name`          | text (exact), stored | file name search                          |
//! | `ext`           | string, fast  | extension filter/facet                           |
//! | `category`      | string, fast  | coarse type filter/facet                         |
//! | `root`          | string, fast  | which indexed location the file belongs to       |
//! | `size`          | u64, fast     | size filter / sort                               |
//! | `modified`      | i64, fast     | mtime (unix seconds) filter / sort               |
//! | `indexed_at`    | i64, stored   | when the document was (re)indexed                |
//! | `has_content`   | u64, fast     | 1 when text was extracted                        |
//! | `content`       | text (stem)   | full text, stemmed — "smart" search              |
//! | `content_exact` | text (exact)  | full text, exact word forms                      |
//! | `body`          | stored only   | extracted text for snippets / previews           |
//! | `encoding`      | string, stored| detected text encoding                           |

use std::path::Path;

use tantivy::schema::{
    Field, IndexRecordOption, Schema, SchemaBuilder, TextFieldIndexing, TextOptions, FAST, INDEXED,
    STORED, STRING,
};
use tantivy::Index;

use crate::analysis::{register_tokenizers, EXACT_TOKENIZER, STEM_TOKENIZER};
use crate::error::Result;

/// Bump when the schema or analyzers change incompatibly; the engine then
/// rebuilds the index from scratch.
pub const SCHEMA_VERSION: u32 = 1;

/// Resolved field handles.
#[derive(Debug, Clone, Copy)]
pub struct Fields {
    pub path: Field,
    pub path_text: Field,
    pub name: Field,
    pub ext: Field,
    pub category: Field,
    pub root: Field,
    pub size: Field,
    pub modified: Field,
    pub indexed_at: Field,
    pub has_content: Field,
    pub content: Field,
    pub content_exact: Field,
    pub body: Field,
    pub encoding: Field,
}

pub fn build_schema() -> (Schema, Fields) {
    let mut b = SchemaBuilder::new();

    let exact_text = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer(EXACT_TOKENIZER)
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );
    let stem_text = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer(STEM_TOKENIZER)
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );

    let path = b.add_text_field("path", STRING | STORED);
    let path_text = b.add_text_field("path_text", exact_text.clone());
    let name = b.add_text_field("name", exact_text.clone().set_stored());
    let ext = b.add_text_field("ext", STRING | STORED | FAST);
    let category = b.add_text_field("category", STRING | STORED | FAST);
    let root = b.add_text_field("root", STRING | STORED | FAST);
    let size = b.add_u64_field("size", INDEXED | STORED | FAST);
    let modified = b.add_i64_field("modified", INDEXED | STORED | FAST);
    let indexed_at = b.add_i64_field("indexed_at", STORED);
    let has_content = b.add_u64_field("has_content", INDEXED | STORED | FAST);
    let content = b.add_text_field("content", stem_text);
    let content_exact = b.add_text_field("content_exact", exact_text);
    let body = b.add_text_field("body", STORED);
    let encoding = b.add_text_field("encoding", STRING | STORED);

    let schema = b.build();
    (
        schema,
        Fields {
            path,
            path_text,
            name,
            ext,
            category,
            root,
            size,
            modified,
            indexed_at,
            has_content,
            content,
            content_exact,
            body,
            encoding,
        },
    )
}

/// Open the index in `dir`, creating it when missing. Registers analyzers.
pub fn open_or_create_index(dir: &Path) -> Result<(Index, Fields)> {
    std::fs::create_dir_all(dir).map_err(|e| crate::error::Error::io(dir, e))?;
    let (schema, fields) = build_schema();
    let mmap = tantivy::directory::MmapDirectory::open(dir)?;
    let index = Index::open_or_create(mmap, schema)?;
    register_tokenizers(&index);
    Ok((index, fields))
}

/// Create a fresh in-memory index (tests, benchmarks).
pub fn create_in_ram() -> (Index, Fields) {
    let (schema, fields) = build_schema();
    let index = Index::create_in_ram(schema);
    register_tokenizers(&index);
    (index, fields)
}
