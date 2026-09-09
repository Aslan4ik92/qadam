//! QIDIR desktop application (Tauri v2 backend).
//!
//! The backend is intentionally thin: it owns a [`qidir_core::Engine`], exposes
//! it through Tauri commands (see [`commands`]) and forwards indexing progress
//! to the webview as events.

mod commands;
mod startup;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use qidir_core::config::AppPaths;
use qidir_core::Engine;
use tauri::{Emitter, Manager};

/// Shared application state.
pub struct AppState {
    pub engine: Arc<Engine>,
    pub paths: AppPaths,
}

/// Event names shared with the frontend.
pub const EVENT_PROGRESS: &str = "index-progress";
pub const EVENT_FINISHED: &str = "index-finished";

fn init_logging(paths: &AppPaths, console: bool) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::prelude::*;
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,tantivy=warn,qidir_core=info".into());
    let _ = std::fs::create_dir_all(&paths.log_dir);
    // A rolling file appender that cannot be created (read-only profile,
    // broken permissions) must not take the application down.
    let file = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("qidir.log")
        .max_log_files(14)
        .build(&paths.log_dir)
        .ok();
    let (file_layer, guard) = match file {
        Some(f) => {
            let (writer, guard) = tracing_appender::non_blocking(f);
            (Some(tracing_subscriber::fmt::layer().with_ansi(false).with_writer(writer)), Some(guard))
        }
        None => (None, None),
    };
    let stderr_layer = if console || cfg!(debug_assertions) {
        Some(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
    } else {
        None
    };
    let _ = tracing_subscriber::registry().with(filter).with(file_layer).with(stderr_layer).try_init();
    guard
}

/// Serve the UI to the default browser instead of a native window.
fn run_browser_mode(paths: AppPaths) {
    let engine = match Engine::open(paths.clone()) {
        Ok(e) => e,
        Err(e) => startup::fatal(&format!(
            "Не удалось открыть индекс QIDIR / cannot open the QIDIR index:\n\n{e}\n\nПапка данных / data folder: {}",
            paths.data_dir.display()
        )),
    };
    let settings = engine.settings();
    if settings.watch_changes {
        if let Err(e) = engine.start_watching() {
            tracing::warn!(error = %e, "cannot start watcher");
        }
    }
    if settings.reindex_on_start && settings.roots.iter().any(|r| r.enabled) {
        let _ = engine.start_indexing(false);
    }
    let port: u16 = std::env::var("QIDIR_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(0);
    let cfg = qidir_server::ServerConfig { port, open_browser: true, ..Default::default() };
    match qidir_server::serve(engine.clone(), paths, cfg) {
        Ok(running) => {
            tracing::info!(url = %running.url, "browser mode ready");
            running.wait();
        }
        Err(e) => startup::fatal(&format!(
            "Не удалось запустить локальный сервер QIDIR / cannot start the local server:\n\n{e}"
        )),
    }
    engine.cancel_indexing();
    engine.stop_watching();
    engine.wait_idle();
    tracing::info!("QIDIR (browser mode) exiting");
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
    let console = startup::maybe_attach_console();
    let paths = match AppPaths::default_paths() {
        Ok(p) => p,
        Err(e) => startup::fatal(&format!(
            "Не удалось определить папку данных пользователя / cannot determine the user data directory:\n\n{e}"
        )),
    };
    let _log_guard = init_logging(&paths, console);
    startup::install_panic_hook(&paths.log_dir);
    tracing::info!(version = qidir_core::VERSION, data_dir = %paths.data_dir.display(), "QIDIR starting");

    // Browser mode: no WebView2 needed. Chosen explicitly (`--browser`) or
    // automatically when the WebView2 runtime is missing.
    let forced_browser = std::env::args().any(|a| a == "--browser");
    if forced_browser || !startup::webview_runtime_available() {
        if !forced_browser {
            startup::notify_browser_fallback();
        }
        run_browser_mode(paths);
        return;
    }

    let stop_ticker = Arc::new(AtomicBool::new(false));
    let stop_ticker_setup = stop_ticker.clone();
    let log_dir = paths.log_dir.clone();

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
                    let msg = match e {
                        qidir_core::Error::IndexLocked => format!(
                            "Индекс QIDIR уже используется другим процессом: возможно, запущена вторая копия программы или qidir.exe в консоли. Закройте её и запустите QIDIR снова.\n\nQIDIR index is already in use by another process.\n\n{}",
                            paths.data_dir.display()
                        ),
                        other => format!(
                            "Не удалось открыть индекс QIDIR / cannot open the QIDIR index:\n\n{other}\n\nПапка данных / data folder: {}\nЕсли ошибка повторяется, удалите папку index и файл manifest.redb — индекс будет построен заново.",
                            paths.data_dir.display()
                        ),
                    };
                    startup::fatal(&msg);
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
        .unwrap_or_else(|e| {
            startup::fatal(&format!(
                "Не удалось создать окно QIDIR / cannot create the QIDIR window:\n\n{e}\n\nПроверьте, что установлена среда Microsoft Edge WebView2, и посмотрите лог в {}",
                log_dir.display()
            ))
        })
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
