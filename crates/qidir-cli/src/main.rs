//! `qidir` — command line interface to the QIDIR search engine.
//!
//! Useful for scripting, headless servers and for diagnosing the analyzers.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use qidir_core::config::{AppPaths, IndexRoot};
use qidir_core::extract::{ExtractOptions, FileCategory};
use qidir_core::fs_util::human_size;
use qidir_core::search::{SearchMode, SearchRequest, SortOrder};
use qidir_core::Engine;

#[derive(Parser)]
#[command(name = "qidir", version, about = "QIDIR — local full-text search (RU/KZ/EN)")]
struct Cli {
    /// Data directory (index, manifest, settings). Defaults to the per-user
    /// application data folder.
    #[arg(long, global = true, env = "QIDIR_DATA_DIR")]
    data_dir: Option<PathBuf>,

    /// Verbose logging (-v, -vv).
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Clone, Copy, ValueEnum)]
enum SortArg {
    Relevance,
    Modified,
    Size,
    Name,
}

#[derive(Subcommand)]
enum Cmd {
    /// Add a folder to the indexed locations.
    AddRoot { path: PathBuf },
    /// Remove a folder from the indexed locations.
    RemoveRoot { path: PathBuf },
    /// Show effective settings as JSON.
    Settings,
    /// Set a setting: key value (e.g. `maxFileSizeBytes 10485760`).
    Set { key: String, value: String },
    /// Index all configured roots (incremental by default).
    Index {
        /// Re-extract every file even if unchanged.
        #[arg(long)]
        full: bool,
        /// Keep running and apply filesystem changes as they happen.
        #[arg(long)]
        watch: bool,
        /// One-off roots to index instead of the configured ones.
        #[arg(long = "root")]
        roots: Vec<PathBuf>,
    },
    /// Search the index.
    Search {
        query: Vec<String>,
        /// Exact word forms instead of morphology-aware matching.
        #[arg(long, short = 'e')]
        exact: bool,
        #[arg(long, short = 'n', default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = 0)]
        offset: usize,
        /// Restrict to extensions (comma separated).
        #[arg(long)]
        ext: Option<String>,
        /// Restrict to categories (document, spreadsheet, pdf, text, code, ...).
        #[arg(long)]
        category: Option<String>,
        #[arg(long, value_enum, default_value_t = SortArg::Relevance)]
        sort: SortArg,
        /// Print JSON instead of a human readable list.
        #[arg(long)]
        json: bool,
    },
    /// Show index statistics.
    Stats,
    /// Delete every indexed document.
    Clear,
    /// Show the highlighted text of one indexed file.
    Preview {
        path: PathBuf,
        query: Vec<String>,
        #[arg(long, short = 'e')]
        exact: bool,
    },
    /// Extract and print the text QIDIR sees in a file (no index needed).
    Extract { path: PathBuf },
    /// Show how text is tokenized/stemmed (no index needed).
    Analyze {
        text: Vec<String>,
        #[arg(long, short = 'e')]
        exact: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let level = match cli.verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| level.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    // Commands that do not need an engine.
    match &cli.cmd {
        Cmd::Extract { path } => {
            let meta = std::fs::metadata(path).with_context(|| format!("cannot stat {}", path.display()))?;
            let ext = qidir_core::fs_util::extension_of(path);
            let opts = ExtractOptions { max_file_size: u64::MAX, ..ExtractOptions::default() };
            match qidir_core::extract::extract_file(path, &ext, meta.len(), &opts)? {
                Some(e) => {
                    if let Some(enc) = e.encoding {
                        eprintln!("encoding: {enc}");
                    }
                    eprintln!("category: {:?}, {} chars", e.category, e.text.chars().count());
                    println!("{}", e.text);
                }
                None => eprintln!("unsupported format: indexed by name only"),
            }
            return Ok(());
        }
        Cmd::Analyze { text, exact } => {
            let mode = if *exact {
                qidir_core::analysis::AnalysisMode::Exact
            } else {
                qidir_core::analysis::AnalysisMode::Stem
            };
            for t in qidir_core::analysis::analyze(&text.join(" "), mode) {
                println!("{:>3}  {:<24} {:?}", t.position, t.term, t.lang);
            }
            return Ok(());
        }
        _ => {}
    }

    let paths = match &cli.data_dir {
        Some(d) => AppPaths::in_dir(d.clone()),
        None => AppPaths::default_paths()?,
    };
    let engine = Engine::open(paths).context("cannot open index")?;

    match cli.cmd {
        Cmd::AddRoot { path } => {
            let path = std::fs::canonicalize(&path).unwrap_or(path);
            let mut s = engine.settings();
            if !s.roots.iter().any(|r| r.path == path) {
                s.roots.push(IndexRoot { path: path.clone(), enabled: true });
            }
            engine.update_settings(s)?;
            println!("added {}", path.display());
        }
        Cmd::RemoveRoot { path } => {
            let path = std::fs::canonicalize(&path).unwrap_or(path);
            let mut s = engine.settings();
            s.roots.retain(|r| r.path != path);
            engine.update_settings(s)?;
            println!("removed {} (run `qidir index` to drop its documents)", path.display());
        }
        Cmd::Settings => println!("{}", serde_json::to_string_pretty(&engine.settings())?),
        Cmd::Set { key, value } => {
            let mut json = serde_json::to_value(engine.settings())?;
            let obj = json.as_object_mut().context("settings is not an object")?;
            let parsed: serde_json::Value =
                serde_json::from_str(&value).unwrap_or(serde_json::Value::String(value));
            anyhow::ensure!(obj.contains_key(&key), "unknown setting `{key}`");
            obj.insert(key, parsed);
            let s: qidir_core::Settings = serde_json::from_value(json).context("invalid value")?;
            engine.update_settings(s)?;
            println!("ok");
        }
        Cmd::Index { full, watch, roots } => {
            if !roots.is_empty() {
                let mut s = engine.settings();
                s.roots = roots
                    .into_iter()
                    .map(|p| IndexRoot { path: std::fs::canonicalize(&p).unwrap_or(p), enabled: true })
                    .collect();
                engine.update_settings(s)?;
            }
            if engine.settings().roots.is_empty() {
                anyhow::bail!("no roots configured; use `qidir add-root <folder>` or `--root <folder>`");
            }
            let sink = Arc::new(|s: &qidir_core::index::indexer::IndexSummary| {
                eprint!(
                    "\r{:?}: scanned {} · indexed {} · unchanged {} · name-only {} · failed {} · {}   ",
                    s.phase,
                    s.scanned,
                    s.indexed,
                    s.unchanged,
                    s.name_only,
                    s.failed,
                    human_size(s.bytes)
                );
            });
            engine.set_progress_sink(sink);
            let started = std::time::Instant::now();
            let summary = engine.index_blocking(full)?;
            eprintln!();
            println!(
                "done in {:.1}s: {} indexed, {} unchanged, {} name-only, {} failed, {} deleted",
                started.elapsed().as_secs_f64(),
                summary.indexed,
                summary.unchanged,
                summary.name_only,
                summary.failed,
                summary.deleted
            );
            if watch {
                engine.start_watching()?;
                println!("watching for changes, press Ctrl+C to stop");
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(3600));
                }
            }
        }
        Cmd::Search { query, exact, limit, offset, ext, category, sort, json } => {
            let req = SearchRequest {
                query: query.join(" "),
                mode: if exact { SearchMode::Exact } else { SearchMode::Smart },
                extensions: ext
                    .map(|e| e.split(',').map(|s| s.trim().to_lowercase()).collect())
                    .unwrap_or_default(),
                categories: category
                    .map(|c| c.split(',').filter_map(|s| FileCategory::parse(s.trim())).collect())
                    .unwrap_or_default(),
                sort: match sort {
                    SortArg::Relevance => SortOrder::Relevance,
                    SortArg::Modified => SortOrder::ModifiedDesc,
                    SortArg::Size => SortOrder::SizeDesc,
                    SortArg::Name => SortOrder::NameAsc,
                },
                offset,
                limit,
                ..SearchRequest::default()
            };
            let res = engine.search(&req)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&res)?);
            } else {
                println!("{} results in {} ms  [{}]", res.total, res.took_ms, res.interpretation);
                for (i, h) in res.hits.iter().enumerate() {
                    let date = chrono::DateTime::from_timestamp(h.modified, 0)
                        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_default();
                    println!("{:>3}. {}  ({}, {})", offset + i + 1, h.path, human_size(h.size), date);
                    if !h.snippet.is_empty() {
                        println!("     {}", strip_marks(&h.snippet));
                    }
                }
                if !res.facets.categories.is_empty() {
                    let f: Vec<String> =
                        res.facets.categories.iter().map(|c| format!("{} {}", c.value, c.count)).collect();
                    println!("types: {}", f.join(", "));
                }
            }
        }
        Cmd::Stats => println!("{}", serde_json::to_string_pretty(&engine.stats()?)?),
        Cmd::Clear => {
            engine.clear_index()?;
            println!("index cleared");
        }
        Cmd::Preview { path, query, exact } => {
            let key = qidir_core::fs_util::path_key(&std::fs::canonicalize(&path).unwrap_or(path));
            let mode = if exact { SearchMode::Exact } else { SearchMode::Smart };
            let p = engine.preview(&key, &query.join(" "), mode)?;
            eprintln!(
                "{} matches, source={}, {} chars{}",
                p.total_matches,
                p.source,
                p.chars,
                if p.truncated { " (truncated)" } else { "" }
            );
            for s in p.segments {
                if s.highlight {
                    print!("[{}]", s.text);
                } else {
                    print!("{}", s.text);
                }
            }
            println!();
        }
        Cmd::Extract { .. } | Cmd::Analyze { .. } => unreachable!(),
    }
    Ok(())
}

/// Turn `<mark>` HTML into terminal-friendly brackets.
fn strip_marks(html: &str) -> String {
    html.replace("<mark>", "[")
        .replace("</mark>", "]")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
}
