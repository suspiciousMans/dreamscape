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
