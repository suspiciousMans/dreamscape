//! A tiny frame profiler, off unless `ENGINE_PROF` is set in the environment.
//!
//! `let _t = perf::scope("render");` times the enclosing block; every
//! `REPORT_EVERY` frames [`frame_end`] logs, per scope, the average and worst
//! time per frame and how many times it ran, then resets. Scopes nest freely
//! (a parent's time includes its children's). Disabled, a scope costs one
//! relaxed atomic load.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Instant;

/// Frames between reports.
pub const REPORT_EVERY: u32 = 300;

static STATE: AtomicU8 = AtomicU8::new(0); // 0 unknown, 1 off, 2 on

/// Is profiling on? (The environment is read once.)
pub fn enabled() -> bool {
    match STATE.load(Ordering::Relaxed) {
        1 => false,
        2 => true,
        _ => {
            let on = std::env::var_os("ENGINE_PROF").is_some();
            STATE.store(if on { 2 } else { 1 }, Ordering::Relaxed);
            on
        }
    }
}

#[derive(Default, Clone, Copy)]
struct Stat {
    total_ms: f64,
    worst_ms: f64,
    calls: u32,
    frame_ms: f64,
}

#[derive(Default)]
struct Table {
    rows: Vec<(&'static str, Stat)>,
    frames: u32,
}

thread_local! {
    static TABLE: RefCell<Table> = RefCell::new(Table::default());
}

/// Times from creation to drop.
pub struct Scope {
    name: &'static str,
    start: Option<Instant>,
}

pub fn scope(name: &'static str) -> Scope {
    Scope {
        name,
        start: enabled().then(Instant::now),
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        let Some(start) = self.start else { return };
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        TABLE.with(|t| {
            let mut t = t.borrow_mut();
            let i = match t.rows.iter().position(|r| r.0 == self.name) {
                Some(i) => i,
                None => {
                    t.rows.push((self.name, Stat::default()));
                    t.rows.len() - 1
                }
            };
            let s = &mut t.rows[i].1;
            s.total_ms += ms;
            s.calls += 1;
            s.frame_ms += ms;
        });
    }
}

/// Call once per frame (after the swap). Logs and resets every
/// [`REPORT_EVERY`] frames.
pub fn frame_end() {
    if !enabled() {
        return;
    }
    TABLE.with(|t| {
        let mut t = t.borrow_mut();
        t.frames += 1;
        for r in t.rows.iter_mut() {
            r.1.worst_ms = r.1.worst_ms.max(r.1.frame_ms);
            r.1.frame_ms = 0.0;
        }
        if t.frames < REPORT_EVERY {
            return;
        }
        let frames = t.frames as f64;
        let mut lines = vec![format!("perf over {} frames (avg ms/frame, worst frame, calls/frame):", t.frames)];
        let mut rows = t.rows.clone();
        rows.sort_by(|a, b| b.1.total_ms.total_cmp(&a.1.total_ms));
        for (name, s) in rows {
            lines.push(format!(
                "  {name:<28} {:>7.3} avg  {:>7.3} worst  {:>6.1} calls",
                s.total_ms / frames,
                s.worst_ms,
                s.calls as f64 / frames
            ));
        }
        log::info!("{}", lines.join("\n"));
        t.rows.clear();
        t.frames = 0;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scope_records_time_when_enabled_and_nothing_when_not() {
        STATE.store(2, Ordering::Relaxed);
        {
            let _a = scope("a");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        TABLE.with(|t| {
            let t = t.borrow();
            let a = t.rows.iter().find(|r| r.0 == "a").expect("row");
            assert_eq!(a.1.calls, 1);
            assert!(a.1.total_ms >= 1.0);
        });
        STATE.store(1, Ordering::Relaxed);
        let before = TABLE.with(|t| t.borrow().rows.len());
        drop(scope("b"));
        assert_eq!(TABLE.with(|t| t.borrow().rows.len()), before, "off = no row");
        STATE.store(0, Ordering::Relaxed);
    }
}
