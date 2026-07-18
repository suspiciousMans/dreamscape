use sdl2::event::Event;

use super::Context;

/// Implemented by a concrete game; driven by `App::run`'s main loop.
pub trait Game {
    fn init(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        let _ = ctx;
        Ok(())
    }

    /// Called once per polled SDL event, after `ctx.input` has already recorded it.
    fn handle_event(&mut self, ctx: &mut Context, event: &Event) {
        let _ = (ctx, event);
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) -> anyhow::Result<()>;

    fn render(&mut self, ctx: &mut Context) -> anyhow::Result<()>;
}
