//! Startup diagnostics: the app is built without a console window, so every
//! failure before or during initialization must be made visible to the user
//! (native message box) and written to the log. Nothing here may panic.

use std::path::Path;

/// Show a blocking native error box (Windows) or print to stderr elsewhere.
pub fn show_error(title: &str, message: &str) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            MessageBoxW, MB_ICONERROR, MB_OK, MB_SETFOREGROUND,
        };
        let to_wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
        let title = to_wide(title);
        let message = to_wide(message);
        // SAFETY: both buffers are valid NUL-terminated UTF-16 strings that
        // outlive the call; a null owner window is allowed.
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                message.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
            );
        }
    }
    #[cfg(not(windows))]
    {
        eprintln!("{title}: {message}");
    }
}

/// Log, show and exit. Used for unrecoverable startup errors.
pub fn fatal(message: &str) -> ! {
    tracing::error!("{message}");
    show_error("QIDIR", message);
    std::process::exit(1)
}

/// Attach a console when started with `--console` (or when `QIDIR_CONSOLE`
/// is set) so that logs are visible in the terminal. Returns `true` when a
/// console is available.
pub fn maybe_attach_console() -> bool {
    let wanted = std::env::args().any(|a| a == "--console") || std::env::var_os("QIDIR_CONSOLE").is_some();
    if !wanted {
        return false;
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::{AllocConsole, AttachConsole, ATTACH_PARENT_PROCESS};
        // SAFETY: plain Win32 calls without pointer arguments.
        unsafe {
            if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
                AllocConsole();
            }
        }
    }
    true
}

/// Route panics to the log and to a message box instead of dying silently.
pub fn install_panic_hook(log_dir: &Path) {
    let log_dir = log_dir.display().to_string();
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic".to_string());
        let msg = format!(
            "QIDIR завершился с внутренней ошибкой / QIDIR crashed:\n\n{payload}\n\nat {location}\n\nЛог / log: {log_dir}"
        );
        tracing::error!("panic: {payload} at {location}");
        default(info);
        show_error("QIDIR", &msg);
    }));
}

/// Make sure the WebView2 runtime is present (Windows only). Without it the
/// window cannot be created at all; explain what to do instead of failing.
pub fn check_webview_runtime() {
    match tauri::webview_version() {
        Ok(v) => tracing::info!(webview = %v, "webview runtime detected"),
        Err(e) => {
            tracing::error!(error = %e, "webview runtime not available");
            if cfg!(windows) {
                let url = "https://developer.microsoft.com/microsoft-edge/webview2/#download";
                let msg = format!(
                    "Для работы QIDIR нужна среда выполнения Microsoft Edge WebView2, а она не найдена на этом компьютере.\n\n\
                     Сейчас откроется страница загрузки: установите «Evergreen Bootstrapper» (около 2 МБ) и запустите QIDIR снова.\n\n\
                     QIDIR needs the Microsoft Edge WebView2 Runtime, which was not found. The download page will open: install the Evergreen Bootstrapper and start QIDIR again.\n\n\
                     {url}\n\nТехническая информация / details: {e}"
                );
                show_error("QIDIR — требуется WebView2", &msg);
                open_url(url);
                std::process::exit(2);
            }
        }
    }
}

fn open_url(url: &str) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("rundll32.exe").args(["url.dll,FileProtocolHandler", url]).spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}
