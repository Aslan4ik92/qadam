//! JSON API: one endpoint per desktop command, same names and argument
//! shapes as the Tauri commands so the frontend transport is a thin switch.

use std::path::PathBuf;
use std::sync::atomic::Ordering;

use qidir_core::config::Settings;
use qidir_core::search::{SearchMode, SearchRequest};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::ServerState;

fn arg<T: for<'de> Deserialize<'de>>(args: &Value, name: &str) -> Result<T, String> {
    let v = args.get(name).cloned().ok_or_else(|| format!("missing argument `{name}`"))?;
    serde_json::from_value(v).map_err(|e| format!("invalid argument `{name}`: {e}"))
}

fn ok<T: serde::Serialize>(v: T) -> Result<Value, String> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

pub fn dispatch(state: &ServerState, cmd: &str, args: Value) -> Result<Value, String> {
    let engine = &state.engine;
    match cmd {
        "ping" => Ok(json!({ "ok": true, "version": qidir_core::VERSION })),
        "shutdown" => {
            state.shutdown.store(true, Ordering::SeqCst);
            Ok(Value::Null)
        }
        "get_settings" => ok(engine.settings()),
        "save_settings" => {
            let settings: Settings = arg(&args, "settings")?;
            ok(engine.update_settings(settings).map_err(|e| e.to_string())?)
        }
        "list_drives" => ok(qidir_core::fs_util::list_drives()),
        "pick_folders" => ok(crate::shell::pick_folders()?),
        "start_indexing" => {
            let full: bool = arg(&args, "full").unwrap_or(false);
            engine.start_indexing(full).map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "cancel_indexing" => {
            engine.cancel_indexing();
            Ok(Value::Null)
        }
        "get_progress" => ok(engine.progress()),
        "search" => {
            let request: SearchRequest = arg(&args, "request")?;
            ok(engine.search(&request).map_err(|e| e.to_string())?)
        }
        "get_preview" => {
            let path: String = arg(&args, "path")?;
            let query: String = arg(&args, "query").unwrap_or_default();
            let mode: SearchMode = arg(&args, "mode").unwrap_or_default();
            ok(engine.preview(&path, &query, mode).map_err(|e| e.to_string())?)
        }
        "get_stats" => ok(engine.stats().map_err(|e| e.to_string())?),
        "clear_index" => {
            engine.clear_index().map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "update_paths" => {
            let paths: Vec<PathBuf> = arg(&args, "paths")?;
            ok(engine.update_paths(&paths).map_err(|e| e.to_string())?)
        }
        "open_file" => {
            crate::shell::open_path(&arg::<String>(&args, "path")?)?;
            Ok(Value::Null)
        }
        "reveal_in_explorer" => {
            crate::shell::reveal(&arg::<String>(&args, "path")?)?;
            Ok(Value::Null)
        }
        "open_with" => {
            crate::shell::open_with(&arg::<String>(&args, "path")?)?;
            Ok(Value::Null)
        }
        "copy_text" => Err("clipboard is handled by the browser in this mode".into()),
        "analyze_text" => {
            let text: String = arg(&args, "text")?;
            let mode: SearchMode = arg(&args, "mode").unwrap_or_default();
            ok(engine.analyze_text(&text, mode))
        }
        "get_app_info" => Ok(json!({
            "version": qidir_core::VERSION,
            "dataDir": state.paths.data_dir.display().to_string(),
            "logDir": state.paths.log_dir.display().to_string(),
            "platform": std::env::consts::OS,
            "mode": "browser",
        })),
        "open_logs_folder" => {
            let _ = std::fs::create_dir_all(&state.paths.log_dir);
            crate::shell::open_path(&state.paths.log_dir.display().to_string())?;
            Ok(Value::Null)
        }
        "open_data_folder" => {
            crate::shell::open_path(&state.paths.data_dir.display().to_string())?;
            Ok(Value::Null)
        }
        other => Err(format!("unknown command `{other}`")),
    }
}
