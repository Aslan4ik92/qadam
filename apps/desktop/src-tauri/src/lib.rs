//! QIDIR desktop application (Tauri v2 backend).
//!
//! The backend is intentionally thin: it owns a [`qidir_core::Engine`], exposes
//! it through Tauri commands (see [`commands`]) and forwards indexing progress
//! to the webview as events.

mod commands;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use qidir_core::config::AppPaths;
use qidir_core::Engine;
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

/// Shared application state.
pub struct AppState {
    pub engine: Arc<Engine>,
    pub paths: AppPaths,
}

/// Event names shared with the frontend.
pub const EVENT_PROGRESS: &str = "index-progress";
pub const EVENT_FINISHED: &str = "index-finished";

fn init_logging(paths: &AppPaths) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::prelude::*;
    let _ = std::fs::create_dir_all(&paths.log_dir);
    let file = tracing_appender::rolling::daily(&paths.log_dir, "qidir.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file);
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,tantivy=warn,qidir_core=info".into());
    let file_layer = tracing_subscriber::fmt::layer().with_ansi(false).with_writer(file_writer);
    let registry = tracing_subscriber::registry().with(filter).with(file_layer);
    #[cfg(debug_assertions)]
    {
        let stderr_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
        registry.with(stderr_layer).init();
    }
    #[cfg(not(debug_assertions))]
    {
        registry.init();
    }
    Some(guard)
}

/// Emits progress events while indexing runs, plus one `index-finished`
/// event when it stops.
fn spawn_progress_ticker(app: tauri::AppHandle, engine: Arc<Engine>, stop: Arc<AtomicBool>) {
    std::thread::Builder::new()
        .name("qidir-progress".into())
        .spawn(move || {
            let mut was_running = false;
            while !stop.load(Ordering::Relaxed) {
                let progress = engine.progress();
                if progress.running {
                    let _ = app.emit(EVENT_PROGRESS, &progress);
                    was_running = true;
                } else if was_running {
                    let _ = app.emit(EVENT_PROGRESS, &progress);
                    let _ = app.emit(EVENT_FINISHED, &progress);
                    was_running = false;
                }
                std::thread::sleep(Duration::from_millis(250));
            }
        })
        .expect("spawn progress ticker");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let paths = AppPaths::default_paths().expect("cannot determine application data directory");
    let _log_guard = init_logging(&paths);
    tracing::info!(version = qidir_core::VERSION, data_dir = %paths.data_dir.display(), "QIDIR starting");

    let stop_ticker = Arc::new(AtomicBool::new(false));
    let stop_ticker_setup = stop_ticker.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A second instance was launched: bring the existing window to front.
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(move |app| {
            let engine = match Engine::open(paths.clone()) {
                Ok(e) => e,
                Err(e) => {
                    tracing::error!(error = %e, "cannot open engine");
                    let msg = match e {
                        qidir_core::Error::IndexLocked => {
                            "Индекс QIDIR уже используется другим процессом (возможно, запущена вторая копия программы или qidir.exe в консоли).\n\nQIDIR index is already in use by another process.".to_string()
                        }
                        other => format!("Не удалось открыть индекс QIDIR / Cannot open QIDIR index:\n\n{other}"),
                    };
                    app.dialog()
                        .message(msg)
                        .title("QIDIR")
                        .kind(MessageDialogKind::Error)
                        .blocking_show();
                    std::process::exit(1);
                }
            };

            let settings = engine.settings();
            if settings.watch_changes {
                if let Err(e) = engine.start_watching() {
                    tracing::warn!(error = %e, "cannot start watcher");
                }
            }
            if settings.reindex_on_start && settings.roots.iter().any(|r| r.enabled) {
                if let Err(e) = engine.start_indexing(false) {
                    tracing::warn!(error = %e, "cannot start indexing on startup");
                }
            }

            spawn_progress_ticker(app.handle().clone(), engine.clone(), stop_ticker_setup.clone());
            app.manage(AppState { engine, paths: paths.clone() });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::list_drives,
            commands::pick_folders,
            commands::start_indexing,
            commands::cancel_indexing,
            commands::get_progress,
            commands::search,
            commands::get_preview,
            commands::get_stats,
            commands::clear_index,
            commands::open_file,
            commands::reveal_in_explorer,
            commands::open_with,
            commands::copy_text,
            commands::analyze_text,
            commands::get_app_info,
            commands::open_logs_folder,
            commands::open_data_folder,
            commands::update_paths,
        ])
        .build(tauri::generate_context!())
        .expect("error while building QIDIR")
        .run(move |app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                stop_ticker.store(true, Ordering::Relaxed);
                if let Some(state) = app.try_state::<AppState>() {
                    state.engine.cancel_indexing();
                    state.engine.stop_watching();
                    state.engine.wait_idle();
                }
                tracing::info!("QIDIR exiting");
            }
        });
}
