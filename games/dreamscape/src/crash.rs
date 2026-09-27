//! What a customer sees when something breaks: a message box pointing at a
//! saved report, plus a rolling log file (release builds have no console).

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT: &str = env!("DREAMSCAPE_GIT_HASH");

/// The current run's seed, for reproducing a crash.
pub static SEED: AtomicU64 = AtomicU64::new(0);

pub fn crash_file_name(unix_secs: u64) -> String {
    format!("crash-{unix_secs}.txt")
}

pub fn report(what: &str, backtrace: &str) -> String {
    format!(
        "Dreamscape {VERSION} ({GIT})\nOS: {} {}\nRun seed: {}\n\n{what}\n\nBacktrace:\n{backtrace}\n",
        std::env::consts::OS,
        std::env::consts::ARCH,
        SEED.load(Ordering::Relaxed),
    )
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn save(report: &str) -> Option<PathBuf> {
    let path = crate::paths::save_file(&crash_file_name(now()));
    std::fs::write(&path, report).ok().map(|_| path)
}

fn tell_player(title: &str, body: &str) {
    use engine::sdl2::messagebox::{show_simple_message_box, MessageBoxFlag};
    let _ = show_simple_message_box(MessageBoxFlag::ERROR, title, body, None);
}

/// Panics: log, save a report, tell the player where it is.
pub fn install() {
    std::panic::set_hook(Box::new(|info| {
        let bt = std::backtrace::Backtrace::force_capture().to_string();
        let r = report(&info.to_string(), &bt);
        log::error!("{r}");
        let msg = match save(&r) {
            Some(p) => format!(
                "Dreamscape crashed. Sorry!\n\nA report was saved to:\n{}\n\nPlease send it to us with a note about what you were doing.",
                p.display()
            ),
            None => "Dreamscape crashed. Sorry!".into(),
        };
        tell_player("Dreamscape", &msg);
    }));
}

/// An error that stopped the game (most often: no OpenGL 3.3 at startup).
pub fn fatal(e: &anyhow::Error) {
    let r = report(&format!("{e:#}"), "");
    log::error!("{r}");
    let saved = save(&r)
        .map(|p| format!("\n\nDetails: {}", p.display()))
        .unwrap_or_default();
    tell_player(
        "Dreamscape could not start",
        &format!("{e}\n\nIf this happened at startup, your graphics driver may not support OpenGL 3.3. Updating it usually fixes this.{saved}"),
    );
}

/// Release builds: a log file (the previous one kept as `.prev.log`).
pub fn open_log() -> Option<std::fs::File> {
    let path = crate::paths::save_file("dreamscape.log");
    let _ = std::fs::rename(&path, path.with_extension("prev.log"));
    let mut f = std::fs::File::create(&path).ok()?;
    let _ = writeln!(f, "Dreamscape {VERSION} ({GIT})");
    Some(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_says_what_where_and_which_build() {
        SEED.store(4242, std::sync::atomic::Ordering::Relaxed);
        let r = report("panicked at src/main.rs:10:5: boom", "0: main");
        for want in [
            VERSION,
            GIT,
            "4242",
            "boom",
            "0: main",
            std::env::consts::OS,
        ] {
            assert!(r.contains(want), "missing {want:?} in\n{r}");
        }
    }

    #[test]
    fn crash_files_are_named_by_time() {
        assert_eq!(crash_file_name(1234), "crash-1234.txt");
    }
}
