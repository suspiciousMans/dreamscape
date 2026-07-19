use crate::script::{Host, Interpreter, ScriptError, Value};

/// What a script (or a native Rust `Behavior`) can do to the entity it's
/// attached to, without needing direct access to the ECS `World` or
/// `AudioContext` — the embedder builds a concrete implementation of this
/// per dispatch, scoped to just that one entity.
pub trait ScriptApi {
    fn log(&mut self, message: &str);
    fn position(&self) -> (f32, f32, f32);
    fn set_position(&mut self, x: f32, y: f32, z: f32);
    fn move_by(&mut self, dx: f32, dy: f32, dz: f32);
    fn play_tone(&mut self, frequency_hz: f32, duration_secs: f32);
    /// Plays a sound file once, fire-and-forget. `path` is relative to the
    /// game's asset root, same convention as script/texture paths.
    fn play_sfx(&mut self, path: &str);
    /// Seconds since the game started — handy for scripts driving their own
    /// motion (`sin(time() * speed)`) without tracking a local phase.
    fn elapsed(&self) -> f32;
    /// Sets (or creates) a named HUD progress bar, `fraction` clamped to
    /// `0.0..=1.0` — e.g. a health or stamina bar.
    fn set_hud_bar(&mut self, name: &str, fraction: f32);
    /// Shows a transient HUD message that fades after `seconds`.
    fn show_toast(&mut self, message: &str, seconds: f32);
}

/// Programmable per-entity behavior. The exact same trait is implemented
/// either by `ScriptBehavior` (running an interpreted `.pss` script) or by a
/// plain Rust type (see `NativeBobBehavior` below) — the game dispatches
/// through `Behavior` uniformly and never needs to know which kind backed a
/// given entity. This is what "use Rust the same way the scripting language
/// would" means in practice: prototype as a script, port to Rust later, and
/// nothing about how it's wired into the game has to change.
pub trait Behavior: Send + Sync {
    fn on_ready(&mut self, api: &mut dyn ScriptApi) {
        let _ = api;
    }
    fn on_update(&mut self, api: &mut dyn ScriptApi, dt: f32) {
        let _ = (api, dt);
    }
    fn on_interact(&mut self, api: &mut dyn ScriptApi) {
        let _ = api;
    }
    fn on_trigger_enter(&mut self, api: &mut dyn ScriptApi, other_name: &str) {
        let _ = (api, other_name);
    }
    fn on_trigger_exit(&mut self, api: &mut dyn ScriptApi, other_name: &str) {
        let _ = (api, other_name);
    }
}

/// The ECS component: wraps whichever `Behavior` an entity has, script or
/// native, behind one boxed trait object.
pub struct BehaviorSlot(pub Box<dyn Behavior>);

/// A `Behavior` backed by a compiled script. Bridges the interpreter's
/// generic `Host` mechanism to the concrete `ScriptApi` for this entity —
/// see `HostAdapter` below for the exact function names a script can call.
pub struct ScriptBehavior {
    interpreter: Interpreter,
}

impl ScriptBehavior {
    pub fn from_source(source: &str) -> Result<Self, Vec<ScriptError>> {
        Ok(Self { interpreter: Interpreter::compile(source)? })
    }

    /// Calls a hook function if the script defines one; a script that
    /// doesn't define e.g. `interact` just does nothing when interacted
    /// with. Runtime errors are logged and otherwise swallowed — a bug in
    /// one script's `update` should never crash the game.
    fn dispatch(&mut self, name: &str, args: &[Value], api: &mut dyn ScriptApi) {
        let mut adapter = HostAdapter { api };
        if let Err(err) = self.interpreter.call(name, args, &mut adapter) {
            log::error!("script error in '{name}': {err}");
        }
    }
}

impl Behavior for ScriptBehavior {
    fn on_ready(&mut self, api: &mut dyn ScriptApi) {
        self.dispatch("ready", &[], api);
    }

    fn on_update(&mut self, api: &mut dyn ScriptApi, dt: f32) {
        self.dispatch("update", &[Value::Number(dt as f64)], api);
    }

    fn on_interact(&mut self, api: &mut dyn ScriptApi) {
        self.dispatch("interact", &[], api);
    }

    fn on_trigger_enter(&mut self, api: &mut dyn ScriptApi, other_name: &str) {
        self.dispatch("trigger_enter", &[Value::Str(other_name.to_string())], api);
    }

    fn on_trigger_exit(&mut self, api: &mut dyn ScriptApi, other_name: &str) {
        self.dispatch("trigger_exit", &[Value::Str(other_name.to_string())], api);
    }
}

/// Translates the interpreter's generic native-call mechanism into concrete
/// `ScriptApi` calls. This is the entire "standard library" a `.pss` script
/// can see: `log(...)`, `get_x/get_y/get_z()`, `set_position(x,y,z)`,
/// `move_by(dx,dy,dz)`, `play_tone(freq,duration)`, `play_sfx(path)`,
/// `time()`, `hud_bar(name,fraction)`, `toast(message,seconds)`. `Value` has
/// no vector/tuple type, so position is read as three separate scalar calls
/// rather than one call returning a triple.
struct HostAdapter<'a> {
    api: &'a mut dyn ScriptApi,
}

impl Host for HostAdapter<'_> {
    fn call_native(&mut self, name: &str, args: &[Value]) -> Result<Value, ScriptError> {
        fn num(args: &[Value], index: usize, fn_name: &str) -> Result<f32, ScriptError> {
            args.get(index)
                .and_then(Value::as_number)
                .map(|n| n as f32)
                .ok_or_else(|| ScriptError {
                    message: format!("'{fn_name}' expects a numeric argument at position {index}"),
                    line: 0,
                })
        }

        match name {
            "log" => {
                let message = args.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ");
                self.api.log(&message);
                Ok(Value::Nil)
            }
            "get_x" => Ok(Value::Number(self.api.position().0 as f64)),
            "get_y" => Ok(Value::Number(self.api.position().1 as f64)),
            "get_z" => Ok(Value::Number(self.api.position().2 as f64)),
            "set_position" => {
                self.api.set_position(num(args, 0, name)?, num(args, 1, name)?, num(args, 2, name)?);
                Ok(Value::Nil)
            }
            "move_by" => {
                self.api.move_by(num(args, 0, name)?, num(args, 1, name)?, num(args, 2, name)?);
                Ok(Value::Nil)
            }
            "play_tone" => {
                self.api.play_tone(num(args, 0, name)?, num(args, 1, name)?);
                Ok(Value::Nil)
            }
            "play_sfx" => {
                let Some(Value::Str(path)) = args.first() else {
                    return Err(ScriptError { message: "'play_sfx' expects a string path first".to_string(), line: 0 });
                };
                self.api.play_sfx(path);
                Ok(Value::Nil)
            }
            "time" => Ok(Value::Number(self.api.elapsed() as f64)),
            "hud_bar" => {
                let Some(Value::Str(bar_name)) = args.first() else {
                    return Err(ScriptError { message: "'hud_bar' expects a string name first".to_string(), line: 0 });
                };
                self.api.set_hud_bar(bar_name, num(args, 1, name)?);
                Ok(Value::Nil)
            }
            "toast" => {
                let Some(Value::Str(message)) = args.first() else {
                    return Err(ScriptError { message: "'toast' expects a string message first".to_string(), line: 0 });
                };
                self.api.show_toast(message, num(args, 1, name)?);
                Ok(Value::Nil)
            }
            _ => Err(ScriptError { message: format!("unknown function '{name}'"), line: 0 }),
        }
    }
}

/// Native-Rust parity example: the identical `Behavior` trait, zero
/// interpreter involved. Sine-bobs vertically and logs+tones on interact —
/// deliberately mirroring what `bob_demo.pss` does as a script, so
/// interacting with both demo cubes side by side makes the parity concrete.
pub struct NativeBobBehavior {
    phase: f32,
    amplitude: f32,
    period_secs: f32,
}

impl NativeBobBehavior {
    pub fn new(amplitude: f32, period_secs: f32) -> Self {
        Self { phase: 0.0, amplitude, period_secs }
    }
}

impl Behavior for NativeBobBehavior {
    fn on_update(&mut self, api: &mut dyn ScriptApi, dt: f32) {
        // `move_by`, not `set_position`: `ScriptApi` has no notion of a
        // "base" position, only the entity's current one, so motion is
        // expressed as the delta between this frame's and last frame's
        // point on the sine wave rather than an absolute offset.
        let previous_phase = self.phase;
        self.phase += dt;
        let wave = |p: f32| (p / self.period_secs * std::f32::consts::TAU).sin();
        api.move_by(0.0, self.amplitude * (wave(self.phase) - wave(previous_phase)), 0.0);
    }

    fn on_interact(&mut self, api: &mut dyn ScriptApi) {
        api.log("Native behavior cube says hi!");
        api.play_tone(750.0, 0.1);
    }
}
