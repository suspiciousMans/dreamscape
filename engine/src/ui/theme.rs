use serde::{Deserialize, Serialize};

/// Editor chrome theme — colors for egui panels, dock headers,
/// toolbar, text, and borders. Serializable via serde/ron so a user
/// can save/load a theme file later; for now there is no persistence UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorTheme {
    /// Main panel / viewport background (RGB 0..=255).
    pub panel_bg: (u8, u8, u8),
    /// Primary text foreground (RGB 0..=255).
    pub panel_fg: (u8, u8, u8),
    /// Panel background on hover (RGB 0..=255).
    pub panel_bg_hover: (u8, u8, u8),
    /// Panel background when selected (RGB 0..=255).
    pub panel_bg_selected: (u8, u8, u8),
    /// Panel separator / border color (RGB 0..=255).
    pub border_color: (u8, u8, u8),
    /// Border thickness in points.
    pub border_size: f32,
    /// Toolbar background (RGB 0..=255).
    pub toolbar_bg: (u8, u8, u8),
    /// Toolbar text foreground (RGB 0..=255).
    pub toolbar_fg: (u8, u8, u8),
    /// Output / log panel background (RGB 0..=255).
    pub output_bg: (u8, u8, u8),
    /// Output / log panel text foreground (RGB 0..=255).
    pub output_fg: (u8, u8, u8),
    /// Viewport (central 3D view) background (RGB 0..=255).
    pub viewport_bg: (u8, u8, u8),
    /// Accent color for selection, active tabs, highlights (RGB 0..=255).
    pub accent: (u8, u8, u8),
    /// Font family for the editor chrome (empty = egui default).
    pub font_family: String,
    /// Base font size in points.
    pub font_size_px: f32,
    /// Panel corner radius in points.
    pub panel_corner_radius_px: f32,
    /// Default spacing between elements in points.
    pub spacing_px: f32,
}

impl EditorTheme {
    /// The built-in default, derived from the Hermes desktop app's dark
    /// theme CSS custom properties (captured from the live app frame).
    /// These values are the editor chrome baseline — the editor should look
    /// native to the surrounding Hermes desktop app when using this default.
    pub fn builtin_default() -> Self {
        Self {
            panel_bg: (30, 32, 36),
            panel_fg: (210, 212, 216),
            panel_bg_hover: (40, 43, 48),
            panel_bg_selected: (50, 53, 58),
            border_color: (55, 58, 63),
            border_size: 1.0,
            toolbar_bg: (26, 28, 31),
            toolbar_fg: (210, 212, 216),
            output_bg: (22, 24, 27),
            output_fg: (190, 192, 196),
            viewport_bg: (28, 30, 33),
            accent: (86, 128, 255),
            font_family: "Inter, system-ui, sans-serif".to_string(),
            font_size_px: 13.0,
            panel_corner_radius_px: 4.0,
            spacing_px: 8.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EditorTheme;

    #[test]
    fn test_theme_builtin_default_roundtrip() {
        let original = EditorTheme::builtin_default();

        // Serialize via ron to a String (the engine already depends on ron).
        let serialized = ron::to_string(&original).expect("serialize failed");
        let restored: EditorTheme = ron::from_str(&serialized).expect("deserialize failed");

        assert_eq!(original.panel_bg, restored.panel_bg);
        assert_eq!(original.panel_fg, restored.panel_fg);
        assert_eq!(original.panel_bg_hover, restored.panel_bg_hover);
        assert_eq!(original.panel_bg_selected, restored.panel_bg_selected);
        assert_eq!(original.border_color, restored.border_color);
        assert_eq!(original.border_size, restored.border_size);
        assert_eq!(original.toolbar_bg, restored.toolbar_bg);
        assert_eq!(original.toolbar_fg, restored.toolbar_fg);
        assert_eq!(original.output_bg, restored.output_bg);
        assert_eq!(original.output_fg, restored.output_fg);
        assert_eq!(original.viewport_bg, restored.viewport_bg);
        assert_eq!(original.accent, restored.accent);
        assert_eq!(original.font_family, restored.font_family);
        assert_eq!(original.font_size_px, restored.font_size_px);
        assert_eq!(
            original.panel_corner_radius_px,
            restored.panel_corner_radius_px
        );
        assert_eq!(original.spacing_px, restored.spacing_px);
    }
}
