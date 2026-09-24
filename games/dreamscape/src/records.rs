//! Best-depth record, stored as a single number in a text file.

use std::path::Path;

/// Relative to the repo root, which is where `cargo run -p dreamscape` runs.
pub const RECORD_PATH: &str = "games/dreamscape/best_depth.txt";

pub fn parse(text: &str) -> u32 {
    text.trim().parse().unwrap_or(0)
}

/// Missing or garbled file = no record yet.
pub fn load(path: &Path) -> u32 {
    std::fs::read_to_string(path).map_or(0, |t| parse(&t))
}

pub fn save(path: &Path, depth: u32) -> std::io::Result<()> {
    std::fs::write(path, format!("{depth}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_numbers_and_tolerates_garbage() {
        assert_eq!(parse("12\n"), 12);
        assert_eq!(parse("  7 "), 7);
        assert_eq!(parse("banana"), 0);
        assert_eq!(parse(""), 0);
    }

    #[test]
    fn missing_file_is_zero() {
        assert_eq!(load(Path::new("definitely/not/here.txt")), 0);
    }

    #[test]
    fn save_then_load_round_trips() {
        let path = std::env::temp_dir().join(format!("dreamscape_best_{}.txt", std::process::id()));
        save(&path, 23).unwrap();
        assert_eq!(load(&path), 23);
        let _ = std::fs::remove_file(&path);
    }
}
