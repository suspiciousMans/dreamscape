use serde::{Deserialize, Serialize};

/// Visual styling for `engine::ui::hud::draw_hud`'s overlay — bundled into
/// `engine::profile::RenderParams` (and thus a `ShaderProfile`) so a game's
/// HUD look is saved/loaded/cycled the same way as its render settings,
/// editable live via the F1 panel's "HUD" section
/// (`engine::ui::panels::hud_style_editor`). Kept separate from `HudState`
/// itself: this is authored "look" data, `HudState` is live per-frame
/// content (title/bars/toasts), the same split `RenderParams` (look) vs.
/// the ECS scene (content) already draws elsewhere.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct HudStyle {
    /// Offset from the top-left corner, pixels.
    pub anchor_offset: [f32; 2],
    pub bar_width: f32,
    pub bar_fill_color: [f32; 3],
    pub title_font_size: f32,
    pub toast_color: [f32; 3],
}

impl Default for HudStyle {
    fn default() -> Self {
        Self {
            anchor_offset: [12.0, 12.0],
            bar_width: 160.0,
            bar_fill_color: [0.3, 0.7, 0.3],
            title_font_size: 20.0,
            toast_color: [1.0, 1.0, 1.0],
        }
    }
}

/// The in-game HUD's entire state: an optional title, named progress bars
/// (health, stamina, ...), and transient fading toast messages. A single
/// `HudState` is global to the game (there's one HUD, not one per entity),
/// so unlike most engine state this isn't an ECS component — a game owns
/// one directly (see `Sandbox::hud` and `draw_hud` in the sandbox) and
/// scripts/native `Behavior`s reach it through `ScriptApi::set_hud_bar`/
/// `show_toast` rather than touching it directly.
#[derive(Default)]
pub struct HudState {
    pub title: Option<String>,
    pub bars: Vec<(String, f32)>,
    toasts: Vec<(String, f32)>,
}

impl HudState {
    /// Sets (or creates) a named bar's fill fraction, clamped to `0..=1`.
    pub fn set_bar(&mut self, name: &str, fraction: f32) {
        let fraction = fraction.clamp(0.0, 1.0);
        match self.bars.iter_mut().find(|(existing, _)| existing == name) {
            Some(entry) => entry.1 = fraction,
            None => self.bars.push((name.to_string(), fraction)),
        }
    }

    /// Queues a toast that fades after `seconds`.
    pub fn show_toast(&mut self, message: &str, seconds: f32) {
        self.toasts.push((message.to_string(), seconds.max(0.0)));
    }

    /// Counts down every toast's remaining time and drops expired ones —
    /// call once per frame regardless of Edit/Play mode so toasts always
    /// finish fading even if the HUD itself is only drawn in Play mode.
    pub fn tick(&mut self, dt: f32) {
        for (_, remaining) in &mut self.toasts {
            *remaining -= dt;
        }
        self.toasts.retain(|(_, remaining)| *remaining > 0.0);
    }

    pub fn toasts(&self) -> &[(String, f32)] {
        &self.toasts
    }
}
