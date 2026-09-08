//! Filesystem watcher: turns `notify` events into debounced path updates.

use std::path::PathBuf;
use std::time::Duration;

use crossbeam_channel as chan;
use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};

use crate::error::{Error, Result};

/// Running watcher; dropping it stops watching.
pub struct Watcher {
    _debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>,
}

impl Watcher {
    /// Watch `roots` recursively and send affected paths to `tx` after a short
    /// quiet period (so a file being written is indexed once, when finished).
    pub fn start(roots: &[PathBuf], tx: chan::Sender<Vec<PathBuf>>) -> Result<Self> {
        let mut debouncer =
            new_debouncer(Duration::from_millis(1500), None, move |res: DebounceEventResult| match res {
                Ok(events) => {
                    let mut paths: Vec<PathBuf> = Vec::new();
                    for ev in events {
                        for p in &ev.paths {
                            if !paths.contains(p) {
                                paths.push(p.clone());
                            }
                        }
                    }
                    if !paths.is_empty() {
                        let _ = tx.send(paths);
                    }
                }
                Err(errors) => {
                    for e in errors {
                        tracing::warn!(error = %e, "watch error");
                    }
                }
            })
            .map_err(|e| Error::Other(format!("cannot create watcher: {e}")))?;

        for root in roots {
            if root.exists() {
                if let Err(e) = debouncer.watch(root, RecursiveMode::Recursive) {
                    tracing::warn!(root = %root.display(), error = %e, "cannot watch root");
                }
            }
        }
        Ok(Self { _debouncer: debouncer })
    }
}
