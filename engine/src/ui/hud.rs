use crate::hud::{HudState, HudStyle};

fn color32(rgb: [f32; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(
        (rgb[0].clamp(0.0, 1.0) * 255.0) as u8,
        (rgb[1].clamp(0.0, 1.0) * 255.0) as u8,
        (rgb[2].clamp(0.0, 1.0) * 255.0) as u8,
    )
}

/// Draws the HUD as a top-left overlay (position/size/colors from `style`):
/// an optional title, each bar as an `egui::ProgressBar`, and toasts
/// stacked below, auto-fading as their remaining time runs low. Call from a
/// game's Play-mode-only render path — see `Sandbox::render`/`draw_hud` in
/// the sandbox for the exact wiring (the same `EguiState` already used for
/// the editor panels, just a different `egui::Area`, drawn only outside the
/// editor).
pub fn draw_hud(ctx: &egui::Context, hud: &HudState, style: &HudStyle) {
    egui::Area::new("hud_overlay".into())
        .anchor(
            egui::Align2::LEFT_TOP,
            egui::vec2(style.anchor_offset[0], style.anchor_offset[1]),
        )
        .show(ctx, |ui| {
            if let Some(title) = &hud.title {
                ui.label(egui::RichText::new(title).size(style.title_font_size).strong());
            }
            for (name, fraction) in &hud.bars {
                ui.add(
                    egui::ProgressBar::new(*fraction)
                        .text(format!("{name}: {:.0}%", fraction * 100.0))
                        .desired_width(style.bar_width)
                        .fill(color32(style.bar_fill_color)),
                );
            }
            let toast_rgb = style.toast_color;
            for (message, remaining) in hud.toasts() {
                let alpha = remaining.min(1.0);
                let base = color32(toast_rgb);
                let color = egui::Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), (alpha * 255.0) as u8);
                ui.colored_label(color, message);
            }
        });
}
