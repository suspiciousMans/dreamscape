/// A snapshot of this frame's performance numbers, computed by the game
/// (frame-time averaging, entity count, draw calls) and handed to
/// `draw_profiler_overlay` for display — this module only draws, it
/// doesn't measure anything itself.
pub struct ProfilerStats {
    pub fps: f32,
    pub frame_time_ms: f32,
    pub entity_count: usize,
    pub draw_calls: u32,
}

/// Draws a small always-on-top-right `egui::Area` with FPS/frame-time/
/// entity-count/draw-call numbers — a debug tool, not gameplay UI, so
/// (unlike `draw_hud`) it's not gated to Play mode and has no configurable
/// `HudStyle`-style look.
pub fn draw_profiler_overlay(ctx: &egui::Context, stats: &ProfilerStats) {
    egui::Area::new("profiler_overlay".into())
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 12.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(format!(
                    "{:.0} FPS ({:.2} ms)",
                    stats.fps, stats.frame_time_ms
                ));
                ui.label(format!("Entities: {}", stats.entity_count));
                ui.label(format!("Draw calls: {}", stats.draw_calls));
            });
        });
}
