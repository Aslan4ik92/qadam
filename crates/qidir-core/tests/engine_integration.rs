//! End-to-end tests of the engine: crawl → extract → index → search → update.

use std::path::PathBuf;
use std::sync::Arc;

use qidir_core::config::{AppPaths, IndexRoot, Settings};
use qidir_core::extract::FileCategory;
use qidir_core::search::{SearchMode, SearchRequest, SortOrder};
use qidir_core::Engine;

fn write(path: &PathBuf, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// Minimal DOCX with the given paragraphs.
fn docx(paragraphs: &[&str]) -> Vec<u8> {
    use std::io::Write;
    let mut body = String::new();
    for p in paragraphs {
        body.push_str(&format!("<w:p><w:r><w:t>{p}</w:t></w:r></w:p>"));
    }
    let doc = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}</w:body></w:document>"#
    );
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut w = zip::ZipWriter::new(&mut buf);
        let o = zip::write::SimpleFileOptions::default();
        w.start_file("[Content_Types].xml", o).unwrap();
        w.write_all(b"<Types/>").unwrap();
        w.start_file("word/document.xml", o).unwrap();
        w.write_all(doc.as_bytes()).unwrap();
        w.finish().unwrap();
    }
    buf.into_inner()
}

struct Fixture {
    _data: tempfile::TempDir,
    docs: tempfile::TempDir,
    engine: Arc<Engine>,
}

fn fixture() -> Fixture {
    let data = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    let root = docs.path().to_path_buf();

    write(
        &root.join("contracts/договор_аренды.txt"),
        "Договор аренды нежилого помещения.\nАрендатор обязуется вносить арендную плату ежемесячно."
            .as_bytes(),
    );
    write(
        &root.join("contracts/report.md"),
        b"# Quarterly report\n\nSearching documents quickly is the goal of this project.",
    );
    write(
        &root.join("kz/kitap.txt"),
        "Кітаптарымызда Қазақстан тарихы туралы көп мәлімет бар. Балалар мектепке барады.".as_bytes(),
    );
    let (cp1251, _, _) =
        encoding_rs::WINDOWS_1251.encode("Старый отчёт о продажах за 1998 год. Продажи выросли.");
    write(&root.join("legacy/otchet_1998.txt"), &cp1251);
    write(
        &root.join("office/Протокол совещания.docx"),
        &docx(&["Протокол совещания №7", "Обсуждали договор аренды и бюджет."]),
    );
    write(
        &root.join("node_modules/pkg/index.js"),
        "договор аренды inside node_modules must be excluded".as_bytes(),
    );
    write(&root.join("bin/tool.exe"), b"MZ\0\0\0\0\0\0\x01\x02\x03\0\0\0\0\0");
    write(&root.join("notes/todo.txt"), b"buy milk\ncall mom\n");

    let mut settings = Settings::default();
    settings.roots.push(IndexRoot { path: root.clone(), enabled: true });
    settings.watch_changes = false;
    settings.worker_threads = 2;
    settings.writer_memory_mb = 64;
    let paths = AppPaths::in_dir(data.path());
    let engine = Engine::open_with_settings(paths, settings).unwrap();
    Fixture { _data: data, docs, engine }
}

fn req(query: &str) -> SearchRequest {
    SearchRequest { query: query.to_string(), limit: 50, ..SearchRequest::default() }
}

#[test]
fn full_cycle() {
    let f = fixture();
    let e = &f.engine;
    let summary = e.index_blocking(false).unwrap();
    assert_eq!(summary.indexed, 7, "{summary:?}"); // node_modules excluded
    assert_eq!(summary.name_only, 1); // tool.exe
    assert_eq!(e.num_docs(), 7);

    // --- Smart search: inflected Russian forms ---
    let r = e.search(&req("договоры аренда")).unwrap();
    let paths: Vec<&str> = r.hits.iter().map(|h| h.path.as_str()).collect();
    assert_eq!(r.total, 2, "{paths:?}");
    assert!(paths.iter().any(|p| p.ends_with("договор_аренды.txt")));
    assert!(paths.iter().any(|p| p.ends_with("Протокол совещания.docx")));
    assert!(!paths.iter().any(|p| p.contains("node_modules")));
    let hit = r.hits.iter().find(|h| h.path.ends_with("договор_аренды.txt")).unwrap();
    assert!(hit.snippet.contains("<mark>Договор</mark>"), "{}", hit.snippet);
    assert!(hit.snippet.contains("<mark>аренды</mark>"), "{}", hit.snippet);
    assert_eq!(hit.category, FileCategory::Text);
    assert_eq!(hit.encoding.as_deref(), Some("UTF-8"));

    // --- Exact mode does not match other forms ---
    let r = e.search(&SearchRequest { mode: SearchMode::Exact, ..req("договоры") }).unwrap();
    assert_eq!(r.total, 0);
    let r = e.search(&SearchRequest { mode: SearchMode::Exact, ..req("договор") }).unwrap();
    assert_eq!(r.total, 2);

    // --- Kazakh morphology + Russian keyboard folding ---
    let r = e.search(&req("кітап")).unwrap();
    assert_eq!(r.total, 1, "{:?}", r.interpretation);
    let r = e.search(&req("Казакстан")).unwrap();
    assert_eq!(r.total, 1);
    let r = e.search(&req("мектеп")).unwrap();
    assert_eq!(r.total, 1);

    // --- English stemming ---
    let r = e.search(&req("search document")).unwrap();
    assert_eq!(r.total, 1);
    assert!(r.hits[0].path.ends_with("report.md"));

    // --- Legacy cp1251 ---
    let r = e.search(&req("отчет продажи")).unwrap();
    assert_eq!(r.total, 1);
    assert_eq!(r.hits[0].encoding.as_deref(), Some("windows-1251"));

    // --- Phrase, exclusion, OR, prefix, fuzzy, field filters ---
    assert_eq!(e.search(&req("\"договор аренды\"")).unwrap().total, 2);
    assert_eq!(e.search(&req("\"аренды договор\"")).unwrap().total, 0);
    assert_eq!(e.search(&req("договор -протокол")).unwrap().total, 1);
    assert_eq!(e.search(&req("milk OR бюджет")).unwrap().total, 2);
    assert_eq!(e.search(&req("догов*")).unwrap().total, 2);
    assert_eq!(e.search(&req("дгоовор~2")).unwrap().total, 2);
    assert_eq!(e.search(&req("ext:docx")).unwrap().total, 1);
    assert_eq!(e.search(&req("name:todo")).unwrap().total, 1);
    assert_eq!(e.search(&req("path:legacy")).unwrap().total, 1);
    assert_eq!(e.search(&req("content:протокол")).unwrap().total, 1);
    // File name match without content match.
    assert_eq!(e.search(&req("tool")).unwrap().total, 1);

    // --- Structured filters + facets ---
    let r = e.search(&SearchRequest { categories: vec![FileCategory::Document], ..req("") }).unwrap();
    assert_eq!(r.total, 1);
    let r = e.search(&SearchRequest { extensions: vec!["txt".into()], ..req("") }).unwrap();
    assert_eq!(r.total, 4);
    let r = e.search(&req("")).unwrap();
    assert_eq!(r.total, 7);
    let txt = r.facets.extensions.iter().find(|f| f.value == "txt").unwrap();
    assert_eq!(txt.count, 4);
    assert!(r.facets.categories.iter().any(|f| f.value == "document" && f.count == 1));
    assert_eq!(r.facets.roots.len(), 1);
    let r = e.search(&SearchRequest { with_content_only: true, ..req("") }).unwrap();
    assert_eq!(r.total, 6);
    let r = e.search(&SearchRequest { size_min: Some(200), ..req("") }).unwrap();
    assert!(r.total >= 1 && r.total < 7);

    // --- Sorting ---
    let r = e.search(&SearchRequest { sort: SortOrder::NameAsc, ..req("") }).unwrap();
    let names: Vec<String> = r.hits.iter().map(|h| h.name.to_lowercase()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
    let r = e.search(&SearchRequest { sort: SortOrder::SizeDesc, ..req("") }).unwrap();
    assert!(r.hits.windows(2).all(|w| w[0].size >= w[1].size));

    // --- Paging ---
    let r = e.search(&SearchRequest { limit: 3, offset: 0, sort: SortOrder::NameAsc, ..req("") }).unwrap();
    assert_eq!(r.hits.len(), 3);
    let r2 = e.search(&SearchRequest { limit: 3, offset: 3, sort: SortOrder::NameAsc, ..req("") }).unwrap();
    assert_eq!(r2.hits.len(), 3);
    assert_ne!(r.hits[0].path, r2.hits[0].path);

    // --- Preview ---
    let p = e.preview(&hit.path, "договоры аренда", SearchMode::Smart).unwrap();
    assert_eq!(p.source, "index");
    assert_eq!(p.total_matches, 2, "{p:?}"); // `арендную` is an adjective: different stem
    let highlighted: Vec<&str> = p.segments.iter().filter(|s| s.highlight).map(|s| s.text.as_str()).collect();
    assert_eq!(highlighted, vec!["Договор", "аренды"]);
    let p = e.preview(&hit.path, "арендная", SearchMode::Smart).unwrap();
    assert_eq!(p.total_matches, 1);

    // --- Stats ---
    let st = e.stats().unwrap();
    assert_eq!(st.documents, 7);
    assert_eq!(st.with_content, 6);
    assert!(st.index_size_bytes > 0);
    assert!(st.last_indexed.is_some());
    assert_eq!(st.roots[0].documents, 7);

    // --- Incremental run: nothing changed ---
    let s2 = e.index_blocking(false).unwrap();
    assert_eq!(s2.indexed, 0);
    assert_eq!(s2.unchanged, 7);
    assert_eq!(e.num_docs(), 7);

    // --- Modify, add, delete → incremental ---
    let root = f.docs.path().to_path_buf();
    std::thread::sleep(std::time::Duration::from_millis(1100)); // mtime granularity
    write(&root.join("notes/todo.txt"), b"buy milk\ncall mom\nsign the lease agreement");
    write(&root.join("notes/new.txt"), "Совершенно новый документ".as_bytes());
    std::fs::remove_file(root.join("contracts/report.md")).unwrap();
    let s3 = e.index_blocking(false).unwrap();
    assert_eq!(s3.indexed, 2, "{s3:?}");
    assert_eq!(s3.deleted, 1);
    assert_eq!(e.num_docs(), 7);
    assert_eq!(e.search(&req("lease")).unwrap().total, 1);
    assert_eq!(e.search(&req("новый")).unwrap().total, 1);
    assert_eq!(e.search(&req("quarterly")).unwrap().total, 0);

    // --- update_paths (watcher path) ---
    write(&root.join("notes/hot.txt"), "горячее обновление".as_bytes());
    let n = e.update_paths(&[root.join("notes/hot.txt")]).unwrap();
    assert_eq!(n, 1);
    assert_eq!(e.search(&req("горячий")).unwrap().total, 1);
    std::fs::remove_dir_all(root.join("notes")).unwrap();
    let n = e.update_paths(&[root.join("notes")]).unwrap();
    assert!(n >= 1);
    assert_eq!(e.search(&req("горячий")).unwrap().total, 0);
    assert_eq!(e.search(&req("path:notes")).unwrap().total, 0);

    // --- Root removal drops its documents on the next run ---
    let mut s = e.settings();
    s.roots.clear();
    e.update_settings(s).unwrap();
    let s4 = e.index_blocking(false).unwrap();
    assert!(s4.deleted >= 1);
    assert_eq!(e.num_docs(), 0);

    // --- Clear ---
    e.clear_index().unwrap();
    assert_eq!(e.num_docs(), 0);
}

#[test]
fn full_reindex_and_cancel() {
    let f = fixture();
    let e = &f.engine;
    e.index_blocking(false).unwrap();
    let s = e.index_blocking(true).unwrap();
    assert_eq!(s.indexed, 7);
    assert_eq!(s.unchanged, 0);
    // Cancel immediately: must not error out with anything but Cancelled and
    // must leave the index consistent.
    e.start_indexing(true).unwrap();
    e.cancel_indexing();
    e.wait_idle();
    assert_eq!(e.num_docs(), 7);
    assert!(e.search(&req("договор")).unwrap().total >= 1);
}

#[test]
fn settings_persist_and_reopen() {
    let data = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    write(&docs.path().join("a.txt"), "persistence test".as_bytes());
    let paths = AppPaths::in_dir(data.path());
    {
        let mut s = Settings::default();
        s.roots.push(IndexRoot { path: docs.path().to_path_buf(), enabled: true });
        s.watch_changes = false;
        let e = Engine::open_with_settings(paths.clone(), s.clone()).unwrap();
        e.update_settings(s).unwrap();
        e.index_blocking(false).unwrap();
        assert_eq!(e.num_docs(), 1);
    }
    let e = Engine::open(paths).unwrap();
    assert_eq!(e.settings().roots.len(), 1);
    assert_eq!(e.num_docs(), 1);
    assert_eq!(e.search(&req("persistence")).unwrap().total, 1);
    let s = e.index_blocking(false).unwrap();
    assert_eq!(s.unchanged, 1);
}
