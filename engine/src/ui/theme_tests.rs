use engine::ui::theme::EditorTheme;

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
