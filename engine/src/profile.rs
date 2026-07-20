use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::hud::HudStyle;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightingMode {
    Unlit,
    VertexLit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextureFilterMode {
    Nearest,
    Bilinear,
}

/// The visual "look" knobs a profile controls. Resolution/fog/dither/lighting
/// are plain uniforms; `affine_texture_mapping` selects a compiled shader
/// variant (see `engine::shader::ShaderVariantCache`) since `noperspective`
/// interpolation is a compile-time GLSL qualifier, not something a uniform
/// can toggle.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RenderParams {
    pub resolution_scale: f32,
    pub fixed_resolution: Option<(u32, u32)>,
    pub fog_color: [f32; 3],
    pub fog_start: f32,
    pub fog_end: f32,
    pub color_levels: u32,
    pub dither_strength: f32,
    pub lighting_mode: LightingMode,
    pub ambient_color: [f32; 3],
    /// Direction *toward* the directional "sun" light (not normalized on
    /// save — the shader normalizes it itself, same as the old hardcoded
    /// constant this replaces). `#[serde(default)]` so profiles saved
    /// before this field existed still load with the same look they always
    /// had, rather than snapping to black/no directional light.
    #[serde(default = "default_light_dir")]
    pub light_dir: [f32; 3],
    pub vertex_snap_amount: f32,
    pub affine_texture_mapping: bool,
    pub texture_filter: TextureFilterMode,
    /// The skybox gradient — mixed by the view ray's Y component, horizon
    /// at the bottom, zenith at the top. `#[serde(default)]` (falling back
    /// to black) so profiles saved before this field existed still load;
    /// re-save to pick up a real sky.
    #[serde(default)]
    pub sky_horizon_color: [f32; 3],
    #[serde(default)]
    pub sky_zenith_color: [f32; 3],
    /// Skips drawing the side of a triangle facing away from the camera —
    /// for a closed mesh this changes nothing visible from outside it (the
    /// front face already occludes the back one via the depth buffer), but
    /// it's what keeps the camera from seeing a wall's/object's *interior*
    /// surface when it clips very close to or slightly inside solid
    /// geometry. `#[serde(default)]` (false) so profiles saved before this
    /// field existed keep their exact old look; re-save to opt in.
    #[serde(default)]
    pub backface_culling: bool,
    /// In-game HUD look (position, bar width/color, title size, toast
    /// color) — edited live via the F1 panel's "HUD" section
    /// (`engine::ui::panels::hud_style_editor`) and drawn by
    /// `engine::ui::hud::draw_hud`. `#[serde(default)]` so profiles saved
    /// before this field existed still load with the exact same HUD look
    /// they always had (the defaults match the old hardcoded values).
    #[serde(default)]
    pub hud: HudStyle,
}

fn default_light_dir() -> [f32; 3] {
    [0.4, 0.8, 0.5]
}

impl Default for RenderParams {
    fn default() -> Self {
        Self {
            resolution_scale: 0.5,
            fixed_resolution: None,
            fog_color: [0.1, 0.1, 0.15],
            fog_start: 5.0,
            fog_end: 20.0,
            color_levels: 32,
            dither_strength: 0.0,
            lighting_mode: LightingMode::VertexLit,
            ambient_color: [0.25, 0.25, 0.3],
            light_dir: default_light_dir(),
            vertex_snap_amount: 0.0,
            affine_texture_mapping: false,
            texture_filter: TextureFilterMode::Nearest,
            sky_horizon_color: [0.55, 0.65, 0.75],
            sky_zenith_color: [0.1, 0.2, 0.45],
            backface_culling: true,
            hud: HudStyle::default(),
        }
    }
}

/// A savable "look": which shader files to use plus the `RenderParams` to
/// drive them. Loading/saving to RON and hotkey-cycling between saved
/// profiles is implemented in a later revision; this is the data shape both
/// the hardcoded (current) and file-driven (future) paths share.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShaderProfile {
    pub name: String,
    pub vertex_shader: PathBuf,
    pub fragment_shader: PathBuf,
    pub post_fragment_shader: PathBuf,
    pub render: RenderParams,
}

pub fn load_from_file(path: &Path) -> anyhow::Result<ShaderProfile> {
    let text = std::fs::read_to_string(path)?;
    Ok(ron::from_str(&text)?)
}

pub fn save_to_file(profile: &ShaderProfile, path: &Path) -> anyhow::Result<()> {
    let text = ron::ser::to_string_pretty(profile, ron::ser::PrettyConfig::default())?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Loads every `*.ron` file in `dir`, sorted by filename for a stable
/// cycling order. Files that fail to parse are logged and skipped rather
/// than failing the whole scan.
pub fn load_dir(dir: &Path) -> anyhow::Result<Vec<ShaderProfile>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ron"))
        .collect();
    paths.sort();

    let mut profiles = Vec::with_capacity(paths.len());
    for path in paths {
        match load_from_file(&path) {
            Ok(profile) => profiles.push(profile),
            Err(err) => log::warn!("skipping shader profile {path:?}: {err}"),
        }
    }
    Ok(profiles)
}

/// Cycles through a fixed set of loaded profiles via `next`/`prev`.
pub struct ProfileCycler {
    profiles: Vec<ShaderProfile>,
    index: usize,
}

impl ProfileCycler {
    pub fn new(profiles: Vec<ShaderProfile>) -> anyhow::Result<Self> {
        anyhow::ensure!(!profiles.is_empty(), "at least one shader profile is required");
        Ok(Self { profiles, index: 0 })
    }

    pub fn current(&self) -> &ShaderProfile {
        &self.profiles[self.index]
    }

    pub fn next(&mut self) -> &ShaderProfile {
        self.index = (self.index + 1) % self.profiles.len();
        self.current()
    }

    pub fn prev(&mut self) -> &ShaderProfile {
        self.index = (self.index + self.profiles.len() - 1) % self.profiles.len();
        self.current()
    }

    pub fn current_index(&self) -> usize {
        self.index
    }

    pub fn all(&self) -> &[ShaderProfile] {
        &self.profiles
    }

    /// Appends a profile and makes it the active one (e.g. after a GUI "Save As").
    pub fn add(&mut self, profile: ShaderProfile) {
        self.profiles.push(profile);
        self.index = self.profiles.len() - 1;
    }

    pub fn select(&mut self, index: usize) -> &ShaderProfile {
        self.index = index.min(self.profiles.len() - 1);
        self.current()
    }
}
