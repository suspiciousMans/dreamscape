use crate::net::NetId;

/// What the player chose from the multiplayer panel this frame — `None`
/// means no button was clicked.
pub enum MultiplayerAction {
    Host,
    Join,
    Disconnect,
    /// Host-only: the "Can switch levels" checkbox for one connected
    /// player was toggled — see `Sandbox.remote_level_switch_permission`.
    SetLevelSwitchPermission {
        net_id: NetId,
        allowed: bool,
    },
    /// Client-side: bails out of an automatic reconnect sequence early —
    /// see `Sandbox.net_reconnect_attempts_left`.
    CancelReconnect,
}

/// Draws a small panel alongside the pause menu for starting/joining a
/// listen-server session — reachable in both debug and release builds
/// (unlike the F1/F2 editor panels, which are dev-only), since a shipped
/// game still needs a way to host/join. Stateless like `draw_pause_menu`:
/// just a function of the current frame plus the two text buffers the
/// caller owns (`join_address`/`player_name`, persisted on `Sandbox`
/// across frames the same way `level_save_as_name` already is) — the
/// caller decides when to call it and acts on whatever it returns.
///
/// `players` is host-only (`&[]` for a client or when offline) — a
/// client isn't shown the roster at all, since level-switch permission
/// is enforced entirely host-side and there's currently no message
/// telling a client its own permission status; it just discovers the
/// exit trigger works once the host grants it.
pub fn draw_multiplayer_panel(
    ctx: &egui::Context,
    status_text: &str,
    connected: bool,
    reconnecting: bool,
    join_address: &mut String,
    player_name: &mut String,
    players: &[(NetId, String, bool)],
) -> Option<MultiplayerAction> {
    let mut action = None;
    egui::Window::new("Multiplayer")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 220.0))
        .show(ctx, |ui| {
            ui.set_min_width(240.0);
            ui.label(status_text);
            ui.separator();

            if reconnecting {
                if ui.button("Cancel").clicked() {
                    action = Some(MultiplayerAction::CancelReconnect);
                }
            } else if connected {
                if ui.button("Disconnect").clicked() {
                    action = Some(MultiplayerAction::Disconnect);
                }
            } else {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(player_name);
                });
                if ui.button("Host Game").clicked() {
                    action = Some(MultiplayerAction::Host);
                }
                ui.horizontal(|ui| {
                    ui.label("Address:");
                    ui.text_edit_singleline(join_address);
                });
                if ui.button("Join Game").clicked() {
                    action = Some(MultiplayerAction::Join);
                }
            }

            if !players.is_empty() {
                ui.separator();
                ui.label("Players");
                for (net_id, name, allowed) in players {
                    ui.horizontal(|ui| {
                        ui.label(name);
                        let mut checked = *allowed;
                        if ui.checkbox(&mut checked, "Can switch levels").changed() {
                            action = Some(MultiplayerAction::SetLevelSwitchPermission {
                                net_id: *net_id,
                                allowed: checked,
                            });
                        }
                    });
                }
            }
        });
    action
}
