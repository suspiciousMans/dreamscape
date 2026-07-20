use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, TryRecvError};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

/// Watches an asset directory recursively for file changes, non-blocking —
/// a game drains it once per frame via `poll_events` and reacts to
/// whatever paths changed (see the hot-reload block in `Game::update` in
/// `games/*/src/main.rs`). Scoped by the caller to shaders/scripts/
/// textures only; level/profile RON files are edited live in the F2 panel
/// and are deliberately never watched here, since a filesystem write
/// racing an in-progress edit (or the undo stack) would be more confusing
/// than helpful.
pub struct HotReloadWatcher {
    // Never read directly — keeping the watcher alive is what keeps the
    // channel receiving events; dropping it stops the watch.
    _watcher: RecommendedWatcher,
    rx: Receiver<notify::Result<Event>>,
}

impl HotReloadWatcher {
    pub fn new(asset_root: &Path) -> anyhow::Result<Self> {
        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.watch(asset_root, RecursiveMode::Recursive)?;
        Ok(Self { _watcher: watcher, rx })
    }

    /// Drains every pending filesystem event, de-duplicating paths that
    /// fired more than once in this call (editors commonly write a file
    /// 2-3 times per save — a rename-into-place plus metadata touches).
    pub fn poll_events(&mut self) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(Ok(event)) => {
                    for path in event.paths {
                        if !changed.contains(&path) {
                            changed.push(path);
                        }
                    }
                }
                Ok(Err(err)) => log::warn!("hot-reload watch error: {err}"),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        changed
    }
}
