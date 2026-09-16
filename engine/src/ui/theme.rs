use serde::{Deserialize, Serialize};

/// Editor chrome theme — colors for egui panels, dock headers,
/// toolbar, text, and borders. Serializable so a user can save/load
/// a theme file later; for now there is no persistence UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorTheme {
    /// Main panel / viewport background.
    pub background: [f32; 3],
    /// Primary text foreground.
    pub foreground: [f32; 3],
    /// Muted text (labels, secondary info, off-state).
    pub muted_foreground: [f32; 3],
    /// Accent (selection, active tab underline, toggle on, hover highlights).
    pub accent: [f32; 3],
    /// Panel separator / border color.
    pub border: [f32; 3],
    /// Panel background (can equal `background` if you want flat; slightly
    /// different for a dock-header feel).
    pub panel: [f32; 3],
    /// Panel header background (dock title bars).
    pub panel_header: [f32; 3],
    /// Font family for the editor chrome (empty = egui default).
    pub font: String,
    /// Base font size in points.
    pub font_size: f32,
}

impl EditorTheme {
    /// Default theme — Godot 4 dark palette, adapted for egui.
    /// RGB values are in the range 0.0..=1.0.
    pub fn builtin_default() -> Self {
        Self {
            // Godot dark theme defaults (from docs/editor_theme_default.md)
            background: [0.17, 0.17, 0.17],           // #2b2b2b
            foreground: [1.0, 1.0, 1.0],              // #ffffff
            muted_foreground: [0.53, 0.53, 0.53],     // #888888
            accent: [0.36, 0.80, 0.93],               // #5cccee
            border: [0.10, 0.10, 0.10],               // #1a1a1a
            panel: [0.24, 0.24, 0.24],                // #3c3c3c
            panel_header: [0.14, 0.14, 0.14],         // #242424
            font: String::new(),                       // egui default
            font_size: 12.0,
        }
    }

    /// Convert RGB [0..1] to egui Color32.
    pub fn rgb_to_color32(&self, rgb: [f32; 3]) -> egui::Color32 {
        let r = (rgb[0].clamp(0.0, 1.0) * 255.0) as u8;
        let g = (rgb[1].clamp(0.0, 1.0) * 255.0) as u8;
        let b = (rgb[2].clamp(0.0, 1.0) * 255.0) as u8;
        egui::Color32::from_rgb(r, g, b)
    }

    /// Background as egui Color32.
    pub fn background_color(&self) -> egui::Color32 {
        self.rgb_to_color32(self.background)
    }

    /// Foreground text as egui Color32.
    pub fn foreground_color(&self) -> egui::Color32 {
        self.rgb_to_color32(self.foreground)
    }

    /// Muted text as egui Color32.
    pub fn muted_color(&self) -> egui::Color32 {
        self.rgb_to_color32(self.muted_foreground)
    }

    /// Accent highlight as egui Color32.
    pub fn accent_color(&self) -> egui::Color32 {
        self.rgb_to_color32(self.accent)
    }

    /// Border/separator as egui Color32.
    pub fn border_color(&self) -> egui::Color32 {
        self.rgb_to_color32(self.border)
    }

    /// Panel background as egui Color32.
    pub fn panel_color(&self) -> egui::Color32 {
        self.rgb_to_color32(self.panel)
    }

    /// Panel header background as egui Color32.
    pub fn panel_header_color(&self) -> egui::Color32 {
        self.rgb_to_color32(self.panel_header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_theme_is_valid() {
        let theme = EditorTheme::builtin_default();
        // All RGB values must be in [0..1]
        assert!(theme.background.iter().all(|&v| v >= 0.0 && v <= 1.0));
        assert!(theme.foreground.iter().all(|&v| v >= 0.0 && v <= 1.0));
        assert!(theme.muted_foreground.iter().all(|&v| v >= 0.0 && v <= 1.0));
        assert!(theme.accent.iter().all(|&v| v >= 0.0 && v <= 1.0));
        assert!(theme.border.iter().all(|&v| v >= 0.0 && v <= 1.0));
        assert!(theme.panel.iter().all(|&v| v >= 0.0 && v <= 1.0));
        assert!(theme.panel_header.iter().all(|&v| v >= 0.0 && v <= 1.0));
    }

    #[test]
    fn test_default_theme_bg_fg_differ() {
        let t = EditorTheme::builtin_default();
        // Background and foreground must be visibly different
        assert_ne!(
            (t.background[0] as i32, t.background[1] as i32, t.background[2] as i32),
            (t.foreground[0] as i32, t.foreground[1] as i32, t.foreground[2] as i32),
            "default theme background must differ from foreground"
        );
    }

    #[test]
    fn test_color_conversion_roundtrip() {
        let theme = EditorTheme::builtin_default();
        let bg_color = theme.background_color();
        // egui Color32 is stored as [r, g, b, a], so we can read it back
        let r = bg_color.r() as f32 / 255.0;
        let g = bg_color.g() as f32 / 255.0;
        let b = bg_color.b() as f32 / 255.0;
        // Within rounding error (~1/255), should match the original
        assert!((r - theme.background[0]).abs() < 0.01);
        assert!((g - theme.background[1]).abs() < 0.01);
        assert!((b - theme.background[2]).abs() < 0.01);
    }
}
