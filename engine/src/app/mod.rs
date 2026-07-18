mod game;

pub use game::Game;

use std::sync::Arc;

use sdl2::event::Event;

use crate::input::Input;
use crate::platform::Platform;
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
}
