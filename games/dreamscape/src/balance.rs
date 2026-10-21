//! Balance log: with `DREAMSCAPE_BALANCE=path.csv` (dev builds), one row per
//! finished dream so a run can be charted afterwards.

use std::io::Write;
use std::path::Path;

pub const HEADER: &str = "depth,theme,difficulty,seconds,catches,upgrades";

pub struct Row<'a> {
    pub depth: u32,
    pub theme: &'a str,
    pub difficulty: f32,
    pub seconds: f32,
    /// Times caught so far this run.
    pub catches: u32,
    pub upgrades: usize,
}

pub fn line(r: &Row) -> String {
    format!(
        "{},{},{:.3},{:.1},{},{}",
        r.depth,
        r.theme.replace(',', " "),
        r.difficulty,
        r.seconds,
        r.catches,
        r.upgrades
    )
}

/// Appends a row, writing the header first if the file is new or empty.
pub fn append(path: &Path, r: &Row) -> std::io::Result<()> {
    let fresh = std::fs::metadata(path).map_or(true, |m| m.len() == 0);
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    if fresh {
        writeln!(f, "{HEADER}")?;
    }
    writeln!(f, "{}", line(r))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> Row<'static> {
        Row {
            depth: 4,
            theme: "Lobby",
            difficulty: 1.23456,
            seconds: 61.26,
            catches: 2,
            upgrades: 3,
        }
    }

    #[test]
    fn line_matches_header_columns() {
        assert_eq!(line(&row()).split(',').count(), HEADER.split(',').count());
        assert_eq!(line(&row()), "4,Lobby,1.235,61.3,2,3");
    }

    #[test]
    fn commas_in_a_theme_cannot_break_the_row() {
        let mut r = row();
        r.theme = "a,b";
        assert_eq!(line(&r).split(',').count(), 6);
    }

    #[test]
    fn append_writes_header_once() {
        let p = std::env::temp_dir().join(format!("ds_bal_{}.csv", std::process::id()));
        let _ = std::fs::remove_file(&p);
        append(&p, &row()).unwrap();
        append(&p, &row()).unwrap();
        let t = std::fs::read_to_string(&p).unwrap();
        let _ = std::fs::remove_file(&p);
        assert_eq!(t.lines().count(), 3);
        assert_eq!(t.lines().next().unwrap(), HEADER);
    }
}
