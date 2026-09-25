/// What the player chose from the appearance panel this frame — `None`
/// means no button was clicked.
pub enum AppearanceAction {
    /// "Import New Model..." — picks a glTF/GLB file via a native file
    /// dialog and imports it as a new rig, same pipeline as the F2 editor's
    /// "Import glTF as Rig...", then adopts it as the local player's
    /// appearance.
    ImportNewModel,
    /// Picks an already-loaded rig (by index into the caller's rig list)
    /// as the local player's appearance, without re-importing anything.
    SelectExisting(usize),
    /// Back to the default appearance (the plain gray cube other players
    /// see when nobody has chosen anything).
    ResetToDefault,
}

/// Draws a small panel alongside the pause menu for choosing what other
/// players see as your player model — reachable in both debug and
/// release builds, same as `draw_multiplayer_panel`, since a shipped
/// game still needs a way to pick an appearance. Stateless: a pure
/// function of the current frame plus the caller-owned rig list; the
/// caller decides when to call it and acts on whatever it returns.
///
/// Purely cosmetic to how *other* players see you — the local player
/// stays first-person with no self-mesh regardless of this choice, so
/// there's nothing to preview here beyond the current selection's name.
pub fn draw_appearance_panel(
    ctx: &egui::Context,
    current: Option<&str>,
    available_rigs: &[String],
) -> Option<AppearanceAction> {
    let mut action = None;
    egui::Window::new("Player Appearance")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 16.0))
        .show(ctx, |ui| {
            ui.set_min_width(220.0);
            ui.label(format!("Current: {}", current.unwrap_or("Default")));
            ui.separator();

            if ui.button("Import New Model...").clicked() {
                action = Some(AppearanceAction::ImportNewModel);
            }
            if current.is_some() && ui.button("Reset to Default").clicked() {
                action = Some(AppearanceAction::ResetToDefault);
            }

            if !available_rigs.is_empty() {
                ui.separator();
                ui.label("Available models:");
                for (index, name) in available_rigs.iter().enumerate() {
                    if ui
                        .selectable_label(Some(name.as_str()) == current, name)
                        .clicked()
                    {
                        action = Some(AppearanceAction::SelectExisting(index));
                    }
                }
            }
        });
    action
}
