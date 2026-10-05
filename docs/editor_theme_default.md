# Editor theme default — derived from Godot 4 dark theme

Captured 2026-09-16. These are the egui-side defaults the editor's
`EditorTheme::builtin_default()` will use. Theme source: Godot 4 dark theme,
adapted for egui color space. Font defaults to egui's system font; can be
replaced with Hermes desktop app font when theming UI is added.

| egui role | source | hex | RGB (0-1) | note |
|-----------|--------|-----|-----------|------|
| background | Godot dark bg | #2b2b2b | (0.17, 0.17, 0.17) | main panel bg |
| foreground | Godot dark fg | #ffffff | (1.0, 1.0, 1.0) | text fg |
| muted_foreground | Godot dark muted | #888888 | (0.53, 0.53, 0.53) | labels, secondary text |
| accent | Godot dark accent | #5cccee | (0.36, 0.80, 0.93) | selection, highlights, active tabs |
| border | Godot dark border | #1a1a1a | (0.10, 0.10, 0.10) | panel separators, edges |
| panel | Godot dark panel | #3c3c3c | (0.24, 0.24, 0.24) | panel backgrounds |
| panel_header | Godot dark header | #242424 | (0.14, 0.14, 0.14) | dock title bars |
| font | egui default | — | — | system font; no override yet |
| font_size | egui default | — | 12.0 | editor chrome font size |

**Fallback note**: if you need to customize to match Hermes desktop app exactly,
read the app's CSS vars (`--background`, `--foreground`, `--muted-foreground`,
`--accent`, `--border`, `--card`) at runtime and update `EditorTheme::builtin_default()`
to use them. For now, Godot dark theme is a reasonable baseline for an in-engine editor.
