//! Tauri commands — the API surface used by the Svelte frontend.
//!
//! Every command returns `Result<T, String>` so the webview receives a plain,
//! localizable-by-context error message.

use std::path::{Path, PathBuf};

use qidir_core::config::Settings;
use qidir_core::engine::{IndexProgress, IndexStats, Preview};
use qidir_core::fs_util::DriveInfo;
use qidir_core::search::{SearchMode, SearchRequest, SearchResponse};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;

use crate::AppState;

type CmdResult<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub log_dir: String,
    pub platform: String,
}

// ------------------------------------------------------------------ settings

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> CmdResult<Settings> {
    Ok(state.engine.settings())
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> CmdResult<Settings> {
    state.engine.update_settings(settings).map_err(err)
}

#[tauri::command]
pub fn list_drives() -> CmdResult<Vec<DriveInfo>> {
    Ok(qidir_core::fs_util::list_drives())
}

/// Native multi-folder picker. Runs off the main thread (async command) as
/// required by the dialog plugin.
#[tauri::command]
pub async fn pick_folders(app: AppHandle) -> CmdResult<Vec<String>> {
    let picked = app.dialog().file().set_title("QIDIR").blocking_pick_folders();
    let mut out = Vec::new();
    for fp in picked.unwrap_or_default() {
        match fp.into_path() {
            Ok(p) => out.push(qidir_core::fs_util::path_key(&p)),
            Err(e) => tracing::warn!(error = %e, "unsupported picked path"),
        }
    }
    Ok(out)
}

// ------------------------------------------------------------------ indexing

#[tauri::command]
pub fn start_indexing(state: State<'_, AppState>, full: bool) -> CmdResult<()> {
    state.engine.start_indexing(full).map_err(err)
}

#[tauri::command]
pub fn cancel_indexing(state: State<'_, AppState>) -> CmdResult<()> {
    state.engine.cancel_indexing();
    Ok(())
}

#[tauri::command]
pub fn get_progress(state: State<'_, AppState>) -> CmdResult<IndexProgress> {
    Ok(state.engine.progress())
}

#[tauri::command]
pub fn clear_index(state: State<'_, AppState>) -> CmdResult<()> {
    state.engine.clear_index().map_err(err)
}

/// Index or remove specific paths (e.g. files dropped onto the window).
#[tauri::command]
pub async fn update_paths(state: State<'_, AppState>, paths: Vec<String>) -> CmdResult<usize> {
    let engine = state.engine.clone();
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    tauri::async_runtime::spawn_blocking(move || engine.update_paths(&paths)).await.map_err(err)?.map_err(err)
}

// ------------------------------------------------------------------ search

#[tauri::command]
pub async fn search(state: State<'_, AppState>, request: SearchRequest) -> CmdResult<SearchResponse> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || engine.search(&request)).await.map_err(err)?.map_err(err)
}

#[tauri::command]
pub async fn get_preview(
    state: State<'_, AppState>,
    path: String,
    query: String,
    mode: SearchMode,
) -> CmdResult<Preview> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || engine.preview(&path, &query, mode))
        .await
        .map_err(err)?
        .map_err(err)
}

#[tauri::command]
pub fn get_stats(state: State<'_, AppState>) -> CmdResult<IndexStats> {
    state.engine.stats().map_err(err)
}

#[tauri::command]
pub fn analyze_text(state: State<'_, AppState>, text: String, mode: SearchMode) -> CmdResult<Vec<String>> {
    Ok(state.engine.analyze_text(&text, mode))
}

// ------------------------------------------------------------------ shell

fn ensure_exists(path: &str) -> CmdResult<&Path> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("Файл не найден / File not found: {path}"));
    }
    Ok(p)
}

/// Open the file with its default application.
#[tauri::command]
pub fn open_file(path: String) -> CmdResult<()> {
    ensure_exists(&path)?;
    tauri_plugin_opener::open_path(&path, None::<&str>).map_err(err)
}

/// Select the file in Windows Explorer (or the platform file manager).
#[tauri::command]
pub fn reveal_in_explorer(path: String) -> CmdResult<()> {
    ensure_exists(&path)?;
    tauri_plugin_opener::reveal_item_in_dir(&path).map_err(err)
}

/// Show the Windows "Open with…" dialog for the file.
#[tauri::command]
pub fn open_with(path: String) -> CmdResult<()> {
    ensure_exists(&path)?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("rundll32.exe")
            .arg("shell32.dll,OpenAs_RunDLL")
            .arg(&path)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map(|_| ())
            .map_err(err)
    }
    #[cfg(not(windows))]
    {
        tauri_plugin_opener::open_path(&path, None::<&str>).map_err(err)
    }
}

#[tauri::command]
pub fn copy_text(app: AppHandle, text: String) -> CmdResult<()> {
    app.clipboard().write_text(text).map_err(err)
}

// ------------------------------------------------------------------ misc

#[tauri::command]
pub fn get_app_info(state: State<'_, AppState>) -> CmdResult<AppInfo> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        data_dir: state.paths.data_dir.display().to_string(),
        log_dir: state.paths.log_dir.display().to_string(),
        platform: std::env::consts::OS.to_string(),
    })
}

#[tauri::command]
pub fn open_logs_folder(state: State<'_, AppState>) -> CmdResult<()> {
    let _ = std::fs::create_dir_all(&state.paths.log_dir);
    tauri_plugin_opener::open_path(state.paths.log_dir.display().to_string(), None::<&str>).map_err(err)
}

#[tauri::command]
pub fn open_data_folder(state: State<'_, AppState>) -> CmdResult<()> {
    tauri_plugin_opener::open_path(state.paths.data_dir.display().to_string(), None::<&str>).map_err(err)
}
