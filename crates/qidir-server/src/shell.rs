//! Desktop integration used in browser mode (no Tauri plugins available).

use std::path::Path;

/// Open a URL in the default browser.
pub fn open_url(url: &str) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("rundll32.exe").args(["url.dll,FileProtocolHandler", url]).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

/// Open a file or folder with its default application.
pub fn open_path(path: &str) -> Result<(), String> {
    exists(path)?;
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe").arg(path).spawn().map(|_| ()).map_err(|e| e.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn().map(|_| ()).map_err(|e| e.to_string())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(path).spawn().map(|_| ()).map_err(|e| e.to_string())
    }
}

/// Select the file in the file manager.
pub fn reveal(path: &str) -> Result<(), String> {
    exists(path)?;
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe")
            .arg(format!("/select,{path}"))
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let parent =
            Path::new(path).parent().map(|p| p.display().to_string()).unwrap_or_else(|| path.to_string());
        open_path(&parent)
    }
}

/// Windows "Open with…" dialog; elsewhere the default application.
pub fn open_with(path: &str) -> Result<(), String> {
    exists(path)?;
    #[cfg(windows)]
    {
        std::process::Command::new("rundll32.exe")
            .args(["shell32.dll,OpenAs_RunDLL", path])
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        open_path(path)
    }
}

/// Native multi-folder picker (Windows). Other platforms return an error and
/// the UI falls back to typing a path.
pub fn pick_folders() -> Result<Vec<String>, String> {
    #[cfg(windows)]
    {
        let picked = rfd::FileDialog::new().set_title("QIDIR").pick_folders().unwrap_or_default();
        Ok(picked.iter().map(|p| qidir_core::fs_util::path_key(p)).collect())
    }
    #[cfg(not(windows))]
    {
        Err("native folder picker is not available in browser mode on this platform; add folders with `qidir add-root`".into())
    }
}

fn exists(path: &str) -> Result<(), String> {
    if Path::new(path).exists() {
        Ok(())
    } else {
        Err(format!("Файл не найден / file not found: {path}"))
    }
}
