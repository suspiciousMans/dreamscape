//! Where things live. Dev (debug) builds keep everything in the repo, as
//! always. Release builds read assets from next to the exe and write saves
//! to the player's data folder, so an installed copy never writes into its
//! own install directory (Steam may replace or verify it).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Every repo-relative path in the game starts with this.
pub const REPO_PREFIX: &str = "games/dreamscape/";

/// The folder the running exe lives in (asset root of an installed copy).
fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// `rel` with the repo prefix swapped for `root`.
pub fn resolve(rel: &str, root: &Path) -> PathBuf {
    root.join(rel.strip_prefix(REPO_PREFIX).unwrap_or(rel))
}

/// A repo-relative asset path: as-is when running from the repo (dev,
/// `cargo run`), otherwise next to the exe (a packaged or installed copy).
pub fn asset(rel: &str) -> PathBuf {
    if Path::new(rel).exists() {
        PathBuf::from(rel)
    } else {
        resolve(rel, &exe_dir())
    }
}

pub fn asset_path(p: &Path) -> PathBuf {
    asset(&p.to_string_lossy().replace('\\', "/"))
}

/// The player's data folder for `os` (`std::env::consts::OS`).
pub fn platform_data_dir(os: &str, env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    match os {
        "windows" => env("APPDATA").map(|d| PathBuf::from(d).join("Dreamscape")),
        "macos" => {
            env("HOME").map(|h| PathBuf::from(h).join("Library/Application Support/Dreamscape"))
        }
        _ => env("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| env("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .map(|d| d.join("dreamscape")),
    }
}

/// Where saves, settings, logs and crash reports go.
pub fn data_dir() -> PathBuf {
    if let Ok(d) = crate::dev::var("DREAMSCAPE_DATA_DIR") {
        return PathBuf::from(d);
    }
    if cfg!(target_os = "emscripten") {
        return PathBuf::from("/persist");
    }
    if cfg!(debug_assertions) {
        return PathBuf::from(REPO_PREFIX.trim_end_matches('/'));
    }
    platform_data_dir(std::env::consts::OS, |k| std::env::var_os(k)).unwrap_or_else(exe_dir)
}

/// A file in the data folder (the folder is created on first use).
pub fn save_file(name: &str) -> PathBuf {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    dir.join(name)
}

/// Release builds: carry a dev checkout's saves over on first launch, so the
/// collection someone built while testing isn't lost.
pub fn migrate_repo_saves() {
    if cfg!(debug_assertions) {
        return;
    }
    for name in [
        "booklet.ron",
        "saved_run.ron",
        "best_depth.txt",
        "settings.ron",
    ] {
        let old = Path::new(REPO_PREFIX).join(name);
        let new = save_file(name);
        if old.exists() && !new.exists() {
            match std::fs::copy(&old, &new) {
                Ok(_) => log::info!("Migrated {old:?} -> {new:?}"),
                Err(e) => log::warn!("Could not migrate {old:?}: {e}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn env(k: &str) -> Option<OsString> {
        match k {
            "APPDATA" => Some("C:/Users/p/AppData/Roaming".into()),
            "HOME" => Some("/home/p".into()),
            _ => None,
        }
    }

    #[test]
    fn repo_asset_paths_resolve_next_to_the_exe() {
        let root = Path::new("C:/Games/Dreamscape");
        assert_eq!(
            resolve("games/dreamscape/assets/shaders/mesh.vert", root),
            root.join("assets/shaders/mesh.vert")
        );
        assert_eq!(
            resolve("games/dreamscape/profiles", root),
            root.join("profiles")
        );
        assert_eq!(
            resolve("elsewhere/x.txt", root),
            root.join("elsewhere/x.txt")
        );
    }

    #[test]
    fn saves_go_to_each_platforms_data_folder() {
        assert_eq!(
            platform_data_dir("windows", env),
            Some(PathBuf::from("C:/Users/p/AppData/Roaming/Dreamscape"))
        );
        assert_eq!(
            platform_data_dir("linux", env),
            Some(PathBuf::from("/home/p/.local/share/dreamscape"))
        );
        assert_eq!(
            platform_data_dir("macos", env),
            Some(PathBuf::from(
                "/home/p/Library/Application Support/Dreamscape"
            ))
        );
        let xdg = |k: &str| (k == "XDG_DATA_HOME").then(|| OsString::from("/x"));
        assert_eq!(
            platform_data_dir("linux", xdg),
            Some(PathBuf::from("/x/dreamscape"))
        );
        assert_eq!(platform_data_dir("windows", |_| None), None);
    }

    #[test]
    fn dev_builds_keep_saves_in_the_repo() {
        if cfg!(debug_assertions) && crate::dev::var("DREAMSCAPE_DATA_DIR").is_err() {
            assert_eq!(
                save_file("booklet.ron"),
                PathBuf::from("games/dreamscape/booklet.ron")
            );
        }
    }
}
