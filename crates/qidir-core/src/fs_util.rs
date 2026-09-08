//! Filesystem helpers with Windows-specific behaviour where it matters.

use std::path::{Path, PathBuf};

/// A logical drive / volume that can be indexed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    /// Root path, e.g. `C:\` (Windows) or `/` (other platforms).
    pub path: PathBuf,
    /// Human readable label, if available.
    pub label: String,
}

/// Enumerate drives available to the user.
///
/// On Windows this probes `A:\` … `Z:\`. On other platforms the filesystem root
/// and the user's home directory are returned so the application remains
/// usable during development on Linux/macOS.
pub fn list_drives() -> Vec<DriveInfo> {
    #[cfg(windows)]
    {
        let mut out = Vec::new();
        for letter in b'A'..=b'Z' {
            let root = format!("{}:\\", letter as char);
            let p = PathBuf::from(&root);
            if std::fs::metadata(&p).is_ok() {
                out.push(DriveInfo { path: p, label: root });
            }
        }
        out
    }
    #[cfg(not(windows))]
    {
        let mut out = vec![DriveInfo { path: PathBuf::from("/"), label: "/".to_string() }];
        if let Some(home) = directories::UserDirs::new().map(|u| u.home_dir().to_path_buf()) {
            out.push(DriveInfo { label: home.display().to_string(), path: home });
        }
        out
    }
}

/// Whether the entry is hidden according to the platform convention.
pub fn is_hidden(path: &Path, meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
        let attrs = meta.file_attributes();
        if attrs & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM) != 0 {
            // Drive roots carry the hidden+system attributes; never hide them.
            return path.parent().is_some();
        }
        false
    }
    #[cfg(not(windows))]
    {
        let _ = meta;
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.') && n != "." && n != "..")
            .unwrap_or(false)
    }
}

/// Lower-case file extension without the leading dot. Empty when absent.
pub fn extension_of(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default()
}

/// Canonical string form of a path used as the document identity in the index.
///
/// Windows paths are stored with backslashes exactly as the OS reports them;
/// the `\\?\` verbatim prefix produced by `canonicalize` is stripped for
/// readability.
pub fn path_key(path: &Path) -> String {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{stripped}")
    } else if let Some(stripped) = s.strip_prefix(r"\\?\") {
        stripped.to_string()
    } else {
        s.into_owned()
    }
}

/// Normalize a path to forward slashes for glob matching.
pub fn glob_form(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Convert a `SystemTime` into UNIX seconds (0 for pre-epoch / errors).
pub fn to_unix_secs(t: std::time::SystemTime) -> i64 {
    t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Format bytes for humans (`1.2 МБ` style is done in the UI; this is ASCII).
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_verbatim_prefix() {
        assert_eq!(path_key(Path::new(r"\\?\C:\Users\a.txt")), r"C:\Users\a.txt");
        assert_eq!(path_key(Path::new(r"\\?\UNC\server\share\f")), r"\\server\share\f");
        assert_eq!(path_key(Path::new("/home/u/f")), "/home/u/f");
    }

    #[test]
    fn extension() {
        assert_eq!(extension_of(Path::new("A.DOCX")), "docx");
        assert_eq!(extension_of(Path::new("noext")), "");
        assert_eq!(extension_of(Path::new("archive.tar.gz")), "gz");
    }

    #[test]
    fn sizes() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(3 * 1024 * 1024), "3.0 MB");
    }
}
