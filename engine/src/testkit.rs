use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};

use crate::script::{Host, ScriptError, Value};

/// Bridges a compiled `.pss` smoke-test script to a live game — the
/// script's own `tick()` function is the driver, called back into these
/// natives once per frame by `engine::app::run_scripted`. `press`/`release`
/// queue synthetic SDL key events rather than applying input directly, so
/// `run_scripted` can feed them through the exact same
/// `ctx.input.handle_event`/`game.handle_event` path a real keypress would
/// take.
pub struct SmokeTestHost {
    elapsed: f32,
    result: Option<bool>,
    pending_events: Vec<Event>,
}

impl SmokeTestHost {
    pub fn new() -> Self {
        Self {
            elapsed: 0.0,
            result: None,
            pending_events: Vec::new(),
        }
    }

    /// Called once per frame by `run_scripted` before invoking `tick`, so
    /// the script's `time()` calls read the same clock the real game loop
    /// uses.
    pub fn set_elapsed(&mut self, elapsed: f32) {
        self.elapsed = elapsed;
    }

    /// `Some(passed)` once the script has called `finish(...)` (or an
    /// assertion has failed) — `run_scripted` checks this after every
    /// `tick` call to decide whether to end the run.
    pub fn result(&self) -> Option<bool> {
        self.result
    }

    /// Drains the synthetic events `press`/`release` queued this frame.
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.pending_events)
    }

    fn queue_key_event(&mut self, name: &str, keycode: Keycode) {
        let event = if name == "press" {
            Event::KeyDown {
                timestamp: 0,
                window_id: 0,
                keycode: Some(keycode),
                scancode: None,
                keymod: Mod::empty(),
                repeat: false,
            }
        } else {
            Event::KeyUp {
                timestamp: 0,
                window_id: 0,
                keycode: Some(keycode),
                scancode: None,
                keymod: Mod::empty(),
                repeat: false,
            }
        };
        self.pending_events.push(event);
    }
}

impl Default for SmokeTestHost {
    fn default() -> Self {
        Self::new()
    }
}

impl Host for SmokeTestHost {
    /// The smoke-test script's entire "standard library": `press(key)`/
    /// `release(key)` (SDL key name, e.g. `"W"`, `"E"`, `"Space"`), `time()`
    /// (seconds since the game started), `assert_true(cond, message)`,
    /// `assert_eq(a, b, message)`, `finish(passed)` (ends the run), and
    /// `log(...)`.
    fn call_native(&mut self, name: &str, args: &[Value]) -> Result<Value, ScriptError> {
        match name {
            "press" | "release" => {
                let Some(Value::Str(key_name)) = args.first() else {
                    return Err(ScriptError {
                        message: format!("'{name}' expects a string key name first"),
                        line: 0,
                    });
                };
                let Some(keycode) = Keycode::from_name(key_name) else {
                    return Err(ScriptError {
                        message: format!("'{name}': unknown key '{key_name}'"),
                        line: 0,
                    });
                };
                self.queue_key_event(name, keycode);
                Ok(Value::Nil)
            }
            "time" => Ok(Value::Number(self.elapsed as f64)),
            "assert_true" => {
                let Some(cond) = args.first() else {
                    return Err(ScriptError {
                        message: "'assert_true' expects a condition".to_string(),
                        line: 0,
                    });
                };
                let message = args.get(1).map(Value::to_string).unwrap_or_default();
                if !cond.truthy() {
                    log::error!("[smoke test] assert_true failed: {message}");
                    self.result = Some(false);
                }
                Ok(Value::Nil)
            }
            "assert_eq" => {
                let (Some(a), Some(b)) = (args.first(), args.get(1)) else {
                    return Err(ScriptError {
                        message: "'assert_eq' expects two values".to_string(),
                        line: 0,
                    });
                };
                let message = args.get(2).map(Value::to_string).unwrap_or_default();
                if a != b {
                    log::error!("[smoke test] assert_eq failed ({a} != {b}): {message}");
                    self.result = Some(false);
                }
                Ok(Value::Nil)
            }
            "finish" => {
                let passed = args.first().map(Value::truthy).unwrap_or(false);
                // An assertion failure earlier this run always wins, even if
                // the script goes on to call `finish(true)` regardless.
                if self.result != Some(false) {
                    self.result = Some(passed);
                }
                Ok(Value::Nil)
            }
            "log" => {
                let message = args
                    .iter()
                    .map(Value::to_string)
                    .collect::<Vec<_>>()
                    .join(" ");
                log::info!("[smoke test] {message}");
                Ok(Value::Nil)
            }
            _ => Err(ScriptError {
                message: format!("unknown function '{name}'"),
                line: 0,
            }),
        }
    }
}
