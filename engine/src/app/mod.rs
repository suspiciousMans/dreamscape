mod game;

pub use game::Game;

use std::sync::Arc;

use sdl2::event::Event;

use crate::input::Input;
use crate::platform::Platform;
use crate::script::Interpreter;
use crate::testkit::SmokeTestHost;
use crate::time::Time;

pub struct Context {
    pub platform: Platform,
    pub input: Input,
    pub time: Time,
    /// Set this to request the main loop exit after the current frame finishes.
    pub should_quit: bool,
}

impl Context {
    pub fn gl(&self) -> &glow::Context {
        &self.platform.gl
    }

    /// A cheap `Arc::clone` for consumers that need shared ownership (e.g.
    /// `egui_glow::Painter`), not a copy of the underlying GL context.
    pub fn gl_arc(&self) -> Arc<glow::Context> {
        Arc::clone(&self.platform.gl)
    }

    pub fn drawable_size(&self) -> (u32, u32) {
        self.platform.drawable_size()
    }

    pub fn aspect_ratio(&self) -> f32 {
        let (w, h) = self.drawable_size();
        w as f32 / h.max(1) as f32
    }
}

pub struct App;

impl App {
    pub fn run<G: Game>(title: &str, width: u32, height: u32, mut game: G) -> anyhow::Result<()> {
        let platform = Platform::new(title, width, height)?;
        let mut ctx = Context {
            platform,
            input: Input::new(),
            time: Time::new(),
            should_quit: false,
        };

        game.init(&mut ctx)?;

        'running: loop {
            ctx.input.begin_frame();

            let events: Vec<Event> = ctx.platform.event_pump.poll_iter().collect();
            for event in events {
                if matches!(event, Event::Quit { .. }) {
                    ctx.should_quit = true;
                }
                ctx.input.handle_event(&event, &ctx.platform.game_controller);
                game.handle_event(&mut ctx, &event);
            }
            if ctx.should_quit {
                break 'running;
            }

            ctx.time.tick();
            let dt = ctx.time.delta;

            game.update(&mut ctx, dt)?;
            game.render(&mut ctx)?;

            ctx.platform.swap_window();
        }

        Ok(())
    }

    /// A separate loop from `run` (not a flag on it), so the production path
    /// stays completely untouched — used by a game's `--smoke-test <path>`
    /// CLI flag to drive itself with a compiled `.pss` script instead of a
    /// human. There's no headless mode in this engine, so a real window
    /// still opens; run under `SDL_VIDEODRIVER=dummy` for CI.
    ///
    /// Each frame: real SDL events are still polled first (so window-close
    /// still works), then the script's own `tick()` function — if it
    /// defines one — is called once via `script.call("tick", ...)`. `tick`
    /// is the driver: it decides what to do this frame by calling
    /// `SmokeTestHost`'s natives (`press`/`release`/`assert_true`/
    /// `assert_eq`/`finish`/`log`, see `engine::testkit`). Key presses it
    /// queues are then fed through the same `ctx.input.handle_event` +
    /// `game.handle_event` path a real keypress would take, before the
    /// normal `update`/`render` for the frame runs. The run ends — and this
    /// function returns `Ok(0)` (pass) or `Ok(1)` (fail) — once the script
    /// calls `finish(...)`, an assertion fails, or the window is closed
    /// without either (treated as a failure: the test never finished).
    pub fn run_scripted<G: Game>(
        title: &str,
        width: u32,
        height: u32,
        mut game: G,
        mut script: Interpreter,
    ) -> anyhow::Result<i32> {
        let platform = Platform::new(title, width, height)?;
        let mut ctx = Context {
            platform,
            input: Input::new(),
            time: Time::new(),
            should_quit: false,
        };

        game.init(&mut ctx)?;

        let mut host = SmokeTestHost::new();
        if !script.has_function("tick") {
            log::error!("smoke test script defines no 'tick' function — nothing to drive the run");
            return Ok(1);
        }

        'running: loop {
            ctx.input.begin_frame();

            let events: Vec<Event> = ctx.platform.event_pump.poll_iter().collect();
            for event in events {
                if matches!(event, Event::Quit { .. }) {
                    ctx.should_quit = true;
                }
                ctx.input.handle_event(&event, &ctx.platform.game_controller);
                game.handle_event(&mut ctx, &event);
            }
            if ctx.should_quit {
                break 'running;
            }

            ctx.time.tick();
            let dt = ctx.time.delta;

            host.set_elapsed(ctx.time.elapsed);
            if let Err(err) = script.call("tick", &[], &mut host) {
                log::error!("smoke test script error in 'tick': {err}");
                break 'running;
            }
            for event in host.drain_events() {
                ctx.input.handle_event(&event, &ctx.platform.game_controller);
                game.handle_event(&mut ctx, &event);
            }

            game.update(&mut ctx, dt)?;
            game.render(&mut ctx)?;

            ctx.platform.swap_window();

            if let Some(passed) = host.result() {
                return Ok(if passed { 0 } else { 1 });
            }
        }

        log::warn!("smoke test ended without calling finish(...) — treating as a failure");
        Ok(1)
    }
}
