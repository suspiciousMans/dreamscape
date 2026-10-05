use std::path::Path;

/// A scrollable text buffer that accumulates log/output lines for the
/// Output dock. Backed by an in-panel `Vec<String>`; the sandbox pushes to
/// it from `log::info!/error!` via a small helper (not wired to the engine's
/// `log` crate directly — the plan specifies an in-panel buffer).
#[derive(Default)]
pub struct OutputPanel {
    lines: Vec<String>,
}

impl OutputPanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, line: &str) {
        self.lines.push(line.to_string());
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn line_count_label(&self) -> String {
        let n = self.lines.len();
        if n == 1 {
            "1 message".to_string()
        } else {
            format!("{} messages", n)
        }
    }
}

/// Scan `dir` recursively for files with one of the given extensions.
/// Returns absolute paths sorted by file name. Mirrors the asset browser's
/// `scan_for_extensions` so both share the same traversal logic.
///
/// Public so the asset-browser tests can call it directly without a GL
/// context or egui `Context`.
pub fn scan_for_files(dir: &Path, extensions: &[&str]) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();
    scan_dir(dir, extensions, &mut results);
    results.sort();
    results
}

fn scan_dir(dir: &Path, extensions: &[&str], results: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_scan_for_files_lists_files_from_real_directory() {
        let tmp = std::env::temp_dir().join("jame_test_asset_dir");
        let _ = fs::create_dir_all(&tmp);

        // Create a couple of files with known extensions.
        fs::write(tmp.join("bg.png"), b"fake-png").unwrap();
        fs::write(tmp.join("icon.jpg"), b"fake-jpg").unwrap();
        fs::write(tmp.join("readme.txt"), b"not-an-asset").unwrap();

        let found = scan_for_files(&tmp, &["png", "jpg"]);

        // sorted by path, so bg.png before icon.jpg
        assert_eq!(found.len(), 2);
        assert!(found[0]
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("bg.png"));
        assert!(found[1]
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("icon.jpg"));

        // tidy up
        let _ = fs::remove_file(tmp.join("bg.png"));
        let _ = fs::remove_file(tmp.join("icon.jpg"));
        let _ = fs::remove_file(tmp.join("readme.txt"));
        let _ = fs::remove_dir(&tmp);
    }

    #[test]
    fn test_scan_for_files_empty_directory() {
        let tmp = std::env::temp_dir().join("jame_test_empty_dir");
        let _ = fs::create_dir_all(&tmp);

        let found = scan_for_files(&tmp, &["png", "jpg"]);
        assert!(found.is_empty());

        let _ = fs::remove_dir(&tmp);
    }
}
