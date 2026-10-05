use egui_dock;
use crate::ui::theme::EditorTheme;

/// The main editor workspace — a docked layout with Scene/Inspector/FileSystem docks
/// and a central viewport, plus a top toolbar and bottom panels.
pub struct EditorDocks {
    pub style: egui_dock::Style,
    /// Theme colors for the docked layout.
    pub theme: EditorTheme,
    /// Visibility toggles for each dock.
    pub visibility: DockVisibility,
}

#[derive(Debug, Clone)]
pub struct DockVisibility {
    pub scene: bool,
    pub inspector: bool,
    pub filesystem: bool,
    pub output: bool,
    pub profiler: bool,
}

impl Default for DockVisibility {
    fn default() -> Self {
        Self {
            scene: true,
            inspector: true,
            filesystem: false,
            output: true,
            profiler: false,
        }
    }
}

impl EditorDocks {
    pub fn new(theme: EditorTheme) -> Self {
        // For now, just use the default egui_dock style.
        // Task 5 will wire the theme colors to egui::Visuals.
        let style = egui_dock::Style::default();
        
        Self {
            style,
            theme,
            visibility: DockVisibility::default(),
        }
    }

    /// Draw the docked editor shell.
    /// This is the main entry point called from the sandbox's render pass.
    /// For now, just render placeholder panels in the viewport area.
    /// Task 3+ will fill in the actual Scene/Inspector/etc. panel content.
    pub fn draw_dock_shell(&mut self, ctx: &egui::Context, _world: &hecs::World) {
        // Top panel: toolbar with Play/Stop/editor-toggle (will be wired in Task 5)
        egui::TopBottomPanel::top("editor_toolbar")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Editor Toolbar (placeholder)");
                    ui.separator();
                    if ui.button("Play").clicked() {
                        // TODO: wire to existing is_paused, editor_mode
                    }
                    if ui.button("Stop").clicked() {
                        // TODO: wire to existing is_paused, editor_mode
                    }
                });
            });

        // Bottom panel: tabs for Output/Profiler
        egui::TopBottomPanel::bottom("editor_bottom")
            .resizable(true)
            .show(ctx, |ui| {
                ui.label("Output/Profiler (placeholder)");
            });

        // Left panel: Scene hierarchy
        if self.visibility.scene {
            egui::SidePanel::left("scene_dock")
                .resizable(true)
                .default_width(250.0)
                .show(ctx, |ui| {
                    ui.label("Scene Hierarchy (placeholder)");
                });
        }

        // Right panel: Inspector
        if self.visibility.inspector {
            egui::SidePanel::right("inspector_dock")
                .resizable(true)
                .default_width(300.0)
                .show(ctx, |ui| {
                    ui.label("Inspector (placeholder)");
                });
        }

        // Central area: viewport (unchanged from the existing render pass)
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Game Viewport (3D rendering happens here, UI stays out of the way)");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docks_creation() {
        let theme = EditorTheme::builtin_default();
        let docks = EditorDocks::new(theme);
        assert!(docks.visibility.scene);
        assert!(docks.visibility.inspector);
        assert!(!docks.visibility.filesystem);
        assert!(docks.visibility.output);
    }

    #[test]
    fn test_visibility_toggle() {
        let mut vis = DockVisibility::default();
        vis.scene = false;
        assert!(!vis.scene);
        assert!(vis.inspector);
    }
}
