use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::backend::EguiState;
use crate::texture::{GpuTexture, TextureFilter};

const TEXTURE_EXTENSIONS: [&str; 3] = ["png", "jpg", "jpeg"];

/// A lightweight in-editor replacement for the OS file-picker dialog,
/// scoped to texture assets specifically — the only asset kind where a
/// thumbnail is cheap and worth showing (model/audio/script pickers keep
/// using `rfd::FileDialog`, no meaningful thumbnail to show for those).
///
/// Two-phase per frame by necessity: `EguiState::register_texture` needs
/// `&mut EguiState`, which is already exclusively borrowed for the
/// duration of its own `run(...)` closure. So thumbnails must be loaded
/// via `ensure_thumbnails_loaded` *before* that call; `show`, called
/// *inside* the closure, only ever reads the already-registered
/// `TextureId`s.
#[derive(Default)]
pub struct AssetBrowserState {
    open: bool,
    files: Vec<PathBuf>,
    thumbnails: HashMap<PathBuf, egui::TextureId>,
    /// Files that failed to load once — negatively cached so a single
    /// corrupt/unreadable image isn't re-read and re-decoded from disk
    /// (with a warn log) on every frame the browser is open. Cleared on
    /// `open` so a re-scan retries them.
    failed: HashSet<PathBuf>,
}

impl AssetBrowserState {
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Opens the browser, (re)scanning `asset_root` recursively for image
    /// files. Cheap enough to call every open — a handful of texture files
    /// per game, not a large asset library.
    pub fn open(&mut self, asset_root: &Path) {
        self.files = scan_for_extensions(asset_root, &TEXTURE_EXTENSIONS);
        self.failed.clear();
        self.open = true;
    }

    /// Loads+registers a GL texture (and an egui `TextureId` for it) for
    /// any scanned file that doesn't have one cached yet. Must be called
    /// *before* `EguiState::run` — see the struct doc comment.
    pub fn ensure_thumbnails_loaded(&mut self, gl: &glow::Context, ui_state: &mut EguiState) {
        if !self.open {
            return;
        }
        let missing: Vec<PathBuf> = self
            .files
            .iter()
            .filter(|path| !self.thumbnails.contains_key(*path) && !self.failed.contains(*path))
            .cloned()
            .collect();
        for path in missing {
            match GpuTexture::load_from_file(gl, &path, TextureFilter::Nearest) {
                Ok(texture) => {
                    let id = ui_state.register_texture(texture.handle);
                    self.thumbnails.insert(path, id);
                }
                Err(err) => {
                    log::warn!("asset browser: failed to load thumbnail {path:?}: {err}");
                    // Negatively cache so this file isn't re-read every frame.
                    self.failed.insert(path);
                }
            }
        }
    }

    /// Draws the browser window if open. Returns the picked file's path
    /// (relative to `asset_root`) the moment a thumbnail is clicked, and
    /// closes the browser.
    pub fn show(&mut self, egui_ctx: &egui::Context, asset_root: &Path) -> Option<PathBuf> {
        if !self.open {
            return None;
        }
        let mut picked = None;
        let mut still_open = true;
        egui::Window::new("Asset Browser: Textures")
            .open(&mut still_open)
            .default_size(egui::vec2(360.0, 420.0))
            .show(egui_ctx, |ui| {
                if self.files.is_empty() {
                    ui.label("No image files found under the asset root.");
                    return;
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("asset_browser_grid").num_columns(3).show(ui, |ui| {
                        for (i, path) in self.files.iter().enumerate() {
                            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                            if let Some(&tex_id) = self.thumbnails.get(path) {
                                let clicked = ui
                                    .add(egui::ImageButton::new((tex_id, egui::vec2(64.0, 64.0))))
                                    .on_hover_text(&name)
                                    .clicked();
                                if clicked {
                                    picked = Some(crate::level::relativize(path, asset_root));
                                }
                            } else {
                                ui.label(&name);
                            }
                            if (i + 1) % 3 == 0 {
                                ui.end_row();
                            }
                        }
                    });
                });
            });
        self.open = still_open && picked.is_none();
        picked
    }
}

fn scan_for_extensions(root: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut results = Vec::new();
    scan_dir(root, extensions, &mut results);
    results.sort();
    results
}

fn scan_dir(dir: &Path, extensions: &[&str], results: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir(&path, extensions, results);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| extensions.contains(&ext.to_lowercase().as_str()))
        {
            results.push(path);
        }
    }
}
