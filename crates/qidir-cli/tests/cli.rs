//! Black-box tests of the `qidir` binary.

use std::path::Path;
use std::process::Command;

fn qidir(data_dir: &Path, args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_qidir"))
        .arg("--data-dir")
        .arg(data_dir)
        .args(args)
        .output()
        .expect("run qidir");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn index_search_preview_roundtrip() {
    let data = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    std::fs::write(docs.path().join("a.txt"), "Договор аренды офиса. Арендная плата вносится ежемесячно.")
        .unwrap();
    std::fs::write(docs.path().join("b.md"), "# Notes\nKazakh: Қазақстан Республикасының астанасы — Астана.")
        .unwrap();
    std::fs::write(docs.path().join("c.bin"), [0u8, 1, 2, 3]).unwrap();

    let (ok, out, err) = qidir(data.path(), &["index", "--root", docs.path().to_str().unwrap()]);
    assert!(ok, "{err}");
    assert!(out.contains("3 indexed"), "{out}");

    let (ok, out, _) = qidir(data.path(), &["search", "договоры"]);
    assert!(ok);
    assert!(out.starts_with("1 results"), "{out}");
    assert!(out.contains("a.txt"));
    assert!(out.contains("[Договор]"));

    let (ok, out, _) = qidir(data.path(), &["search", "Казакстан", "--json"]);
    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["total"], 1);
    assert!(json["hits"][0]["path"].as_str().unwrap().ends_with("b.md"));

    let (ok, out, _) = qidir(data.path(), &["search", "-e", "договоры"]);
    assert!(ok);
    assert!(out.starts_with("0 results"), "{out}");

    let (ok, out, _) = qidir(data.path(), &["stats"]);
    assert!(ok);
    let stats: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(stats["documents"], 3);
    assert_eq!(stats["withContent"], 2);

    let a = std::fs::canonicalize(docs.path().join("a.txt")).unwrap();
    let (ok, out, err) = qidir(data.path(), &["preview", a.to_str().unwrap(), "плата"]);
    assert!(ok, "{err}");
    assert!(out.contains("[плата]"), "{out}");

    let (ok, out, _) = qidir(data.path(), &["index"]);
    assert!(ok);
    assert!(out.contains("0 indexed, 3 unchanged"), "{out}");

    let (ok, out, _) = qidir(data.path(), &["clear"]);
    assert!(ok);
    assert!(out.contains("cleared"));
}

#[test]
fn analyze_and_extract_do_not_need_an_index() {
    let data = tempfile::tempdir().unwrap();
    let (ok, out, _) = qidir(data.path(), &["analyze", "Кітаптарымызда searching"]);
    assert!(ok);
    assert!(out.contains("китап"), "{out}");
    assert!(out.contains("search"), "{out}");

    let docs = tempfile::tempdir().unwrap();
    let (cp1251, _, _) = encoding_rs::WINDOWS_1251
        .encode("Старый файл в кодировке Windows-1251, довольно длинный текст для детектора.");
    let f = docs.path().join("old.txt");
    std::fs::write(&f, &cp1251).unwrap();
    let (ok, out, err) = qidir(data.path(), &["extract", f.to_str().unwrap()]);
    assert!(ok);
    assert!(out.contains("Старый файл"), "{out}");
    assert!(err.contains("windows-1251"), "{err}");
}

#[test]
fn index_without_roots_fails_clearly() {
    let data = tempfile::tempdir().unwrap();
    let (ok, _, err) = qidir(data.path(), &["index"]);
    assert!(!ok);
    assert!(err.contains("no roots configured"), "{err}");
}
