use crate::hud::HudState;

/// Draws the HUD as a fixed top-left overlay: an optional title, each bar
/// as an `egui::ProgressBar`, and toasts stacked below, auto-fading as
/// their remaining time runs low. Call from a game's Play-mode-only render
/// path — see `Sandbox::render`/`draw_hud` in the sandbox for the exact
/// wiring (the same `EguiState` already used for the editor panels, just a
/// different `egui::Area`, drawn only outside the editor).
pub fn draw_hud(ctx: &egui::Context, hud: &HudState) {
    egui::Area::new("hud_overlay".into())
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(12.0, 12.0))
        .show(ctx, |ui| {
            if let Some(title) = &hud.title {
                ui.heading(title);
            }
            for (name, fraction) in &hud.bars {
                ui.add(
                    egui::ProgressBar::new(*fraction)
                        .text(format!("{name}: {:.0}%", fraction * 100.0))
                        .desired_width(160.0),
                );
            }
            for (message, remaining) in hud.toasts() {
                let alpha = remaining.min(1.0);
                ui.colored_label(egui::Color32::from_white_alpha((alpha * 255.0) as u8), message);
            }
        });
}
