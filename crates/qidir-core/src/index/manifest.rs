//! Fingerprints of indexed files, used to skip unchanged files on
//! incremental runs and to detect deletions.
//!
//! Backed by `redb`, an embedded ACID key-value store: crash-safe and fast
//! enough for millions of entries.

use std::path::Path;

use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};

use crate::error::{Error, Result};

const FILES: TableDefinition<&str, &[u8]> = TableDefinition::new("files");
const META: TableDefinition<&str, &str> = TableDefinition::new("meta");

/// Fingerprint used to decide whether a file changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fingerprint {
    pub modified: i64,
    pub size: u64,
    /// Whether text was extracted (files that failed extraction are retried
    /// on the next full run only).
    pub has_content: bool,
}

impl Fingerprint {
    fn to_bytes(self) -> [u8; 17] {
        let mut b = [0u8; 17];
        b[..8].copy_from_slice(&self.modified.to_le_bytes());
        b[8..16].copy_from_slice(&self.size.to_le_bytes());
        b[16] = self.has_content as u8;
        b
    }

    fn from_bytes(b: &[u8]) -> Option<Self> {
        if b.len() < 16 {
            return None;
        }
        Some(Self {
            modified: i64::from_le_bytes(b[..8].try_into().ok()?),
            size: u64::from_le_bytes(b[8..16].try_into().ok()?),
            has_content: b.get(16).copied().unwrap_or(1) != 0,
        })
    }
}

pub struct Manifest {
    db: Database,
}

impl Manifest {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let db = Database::create(path)?;
        // Make sure tables exist.
        let tx = db.begin_write()?;
        {
            tx.open_table(FILES)?;
            tx.open_table(META)?;
        }
        tx.commit()?;
        Ok(Self { db })
    }

    pub fn get(&self, path: &str) -> Result<Option<Fingerprint>> {
        let tx = self.db.begin_read()?;
        let t = tx.open_table(FILES)?;
        Ok(t.get(path)?
            .and_then(|v| Fingerprint::from_bytes(v.value())))
    }

    /// Insert or update many fingerprints in one transaction.
    pub fn put_many<'a>(
        &self,
        items: impl IntoIterator<Item = (&'a str, Fingerprint)>,
    ) -> Result<()> {
        let tx = self.db.begin_write()?;
        {
            let mut t = tx.open_table(FILES)?;
            for (p, fp) in items {
                t.insert(p, fp.to_bytes().as_slice())?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn remove_many<'a>(&self, paths: impl IntoIterator<Item = &'a str>) -> Result<()> {
        let tx = self.db.begin_write()?;
        {
            let mut t = tx.open_table(FILES)?;
            for p in paths {
                t.remove(p)?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// All known paths (sorted).
    pub fn all_paths(&self) -> Result<Vec<String>> {
        let tx = self.db.begin_read()?;
        let t = tx.open_table(FILES)?;
        let mut out = Vec::new();
        for entry in t.iter()? {
            let (k, _) = entry?;
            out.push(k.value().to_string());
        }
        Ok(out)
    }

    /// Paths starting with `prefix` (used when a directory is removed).
    pub fn paths_with_prefix(&self, prefix: &str) -> Result<Vec<String>> {
        let tx = self.db.begin_read()?;
        let t = tx.open_table(FILES)?;
        let mut out = Vec::new();
        for entry in t.range(prefix..)? {
            let (k, _) = entry?;
            let k = k.value();
            if !k.starts_with(prefix) {
                break;
            }
            out.push(k.to_string());
        }
        Ok(out)
    }

    pub fn len(&self) -> Result<u64> {
        let tx = self.db.begin_read()?;
        let t = tx.open_table(FILES)?;
        Ok(t.len()?)
    }

    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    pub fn clear(&self) -> Result<()> {
        let tx = self.db.begin_write()?;
        {
            tx.delete_table(FILES)?;
            tx.open_table(FILES)?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let tx = self.db.begin_read()?;
        let t = tx.open_table(META)?;
        Ok(t.get(key)?.map(|v| v.value().to_string()))
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        let tx = self.db.begin_write()?;
        {
            let mut t = tx.open_table(META)?;
            t.insert(key, value)?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let m = Manifest::open(&dir.path().join("manifest.redb")).unwrap();
        let fp = Fingerprint {
            modified: 1_700_000_000,
            size: 42,
            has_content: true,
        };
        m.put_many([
            ("C:\\a\\1.txt", fp),
            ("C:\\a\\2.txt", fp),
            ("C:\\b\\3.txt", fp),
        ])
        .unwrap();
        assert_eq!(m.get("C:\\a\\1.txt").unwrap(), Some(fp));
        assert_eq!(m.get("nope").unwrap(), None);
        assert_eq!(m.paths_with_prefix("C:\\a\\").unwrap().len(), 2);
        assert_eq!(m.len().unwrap(), 3);
        m.remove_many(["C:\\a\\1.txt"]).unwrap();
        assert_eq!(m.len().unwrap(), 2);
        m.set_meta("schema", "1").unwrap();
        assert_eq!(m.get_meta("schema").unwrap().as_deref(), Some("1"));
        m.clear().unwrap();
        assert!(m.is_empty().unwrap());
        assert_eq!(m.get_meta("schema").unwrap().as_deref(), Some("1"));
    }
}
