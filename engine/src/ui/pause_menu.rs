/// What the player chose from the pause menu this frame — `None` means no
/// button was clicked.
pub enum PauseMenuAction {
    Resume,
    /// Only offered when `editor_available` is true (see `draw_pause_menu`)
    /// — despawns the player and returns to the dev-time level editor.
    ExitToEditor,
    /// Offered instead of `ExitToEditor` when there's no editor to exit
    /// to (a shipped/release build) — resets the current Play session.
    RestartLevel,
    QuitGame,
}

/// Draws a centered, modal-style pause overlay with Resume/(Exit to
/// Editor or Restart Level)/Quit buttons. Stateless — like `draw_hud`,
/// just a function of the current frame's `egui::Context` — the caller
/// decides when to call it (typically gated on a `paused` flag) and acts
/// on whatever it returns. `editor_available` picks the middle button: a
/// shipped game has no editor to exit to, so it offers a restart instead.
pub fn draw_pause_menu(ctx: &egui::Context, editor_available: bool) -> Option<PauseMenuAction> {
    let mut action = None;
    egui::Window::new("Paused")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_min_width(180.0);
            if ui.button("Resume").clicked() {
                action = Some(PauseMenuAction::Resume);
            }
            if editor_available {
                if ui.button("Exit to Editor").clicked() {
                    action = Some(PauseMenuAction::ExitToEditor);
                }
            } else if ui.button("Restart Level").clicked() {
                action = Some(PauseMenuAction::RestartLevel);
            }
            if ui.button("Quit Game").clicked() {
                action = Some(PauseMenuAction::QuitGame);
            }
        });
    action
}
