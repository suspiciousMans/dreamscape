//! Tunnel vision: the dream only exists near you. Two layers:
//! 1. World (mesh.frag): past `sight_radius` from the player, surfaces sink
//!    into darkness. Things you steer by — you, shards, beacons, the portal,
//!    the wake door — carry `Lit` and glow through it.
//! 2. Screen (egui, under the HUD): a soft vignette ring centred on the
//!    player's screen position, plus animated film grain.
//! Everything here is pure (geometry, curves, noise) and unit-tested; the
//! drawing call is `draw_overlay`.

use crate::gameplay::CELL;
use engine::ui::egui::{self, Color32, Mesh, Pos2, Rect, TextureId};

/// World units you can see clearly at strangeness 0.
pub const SIGHT_CLEAR: f32 = 4.2 * CELL;
/// How much the sight shrinks at full strangeness.
pub const SIGHT_SHRINK: f32 = 0.9 * CELL;
/// Fraction of the sight radius over which things fade to black.
pub const SIGHT_FADE: f32 = 0.4;
/// The darkness colour is the fog colour scaled by this.
pub const DARKNESS: f32 = 0.10;

/// Radius (world units) at which the world is fully dark.
pub fn sight_radius(strangeness: f32) -> f32 {
    SIGHT_CLEAR - SIGHT_SHRINK * strangeness.clamp(0.0, 1.0)
}

/// The colour beyond the edge of sight (also the clear colour).
pub fn darkness(fog: [f32; 3]) -> [f32; 3] {
    fog.map(|c| c * DARKNESS)
}

/// Screen vignette: inner radius (clear) and outer radius (fully dim) as a
/// fraction of the screen's half-diagonal, breathing slowly.
pub fn vignette_radii(strangeness: f32, time: f32) -> (f32, f32) {
    let s = strangeness.clamp(0.0, 1.0);
    let breathe = 0.02 * (time * 0.9).sin();
    let inner = 0.34 - 0.08 * s + breathe;
    let outer = 0.86 - 0.12 * s + breathe;
    (inner, outer)
}

/// Max darkness of the screen vignette (0..1 alpha).
pub const VIGNETTE_MAX: f32 = 0.72;

/// Vignette alpha at a normalised distance `d` (0 = centre) given radii.
pub fn vignette_alpha(d: f32, inner: f32, outer: f32) -> f32 {
    let t = ((d - inner) / (outer - inner).max(1e-4)).clamp(0.0, 1.0);
    VIGNETTE_MAX * t * t * (3.0 - 2.0 * t)
}

/// A ring-shaped mesh: `rings` concentric circles of `segments` vertices
/// from radius 0 out past the screen corner, coloured by `alpha(d)` where d
/// is the radius / `scale`. The centre vertex is shared.
pub fn vignette_mesh(
    centre: Pos2,
    scale: f32,
    radii: &[f32],
    segments: u32,
    rgb: [u8; 3],
    alpha: impl Fn(f32) -> f32,
) -> Mesh {
    let mut m = Mesh::default();
    let col = |a: f32| Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], (a * 255.0) as u8);
    m.colored_vertex(centre, col(alpha(0.0)));
    for &r in radii {
        for k in 0..segments {
            let a = std::f32::consts::TAU * k as f32 / segments as f32;
            let p = centre + egui::vec2(a.cos(), a.sin()) * (r * scale);
            m.colored_vertex(p, col(alpha(r)));
        }
    }
    let ring = |i: usize, k: u32| 1 + i as u32 * segments + (k % segments);
    // Centre fan.
    for k in 0..segments {
        m.add_triangle(0, ring(0, k), ring(0, k + 1));
    }
    // Quads between rings.
    for i in 0..radii.len().saturating_sub(1) {
        for k in 0..segments {
            let (a, b, c, d) = (
                ring(i, k),
                ring(i, k + 1),
                ring(i + 1, k),
                ring(i + 1, k + 1),
            );
            m.add_triangle(a, b, c);
            m.add_triangle(b, d, c);
        }
    }
    m
}

/// Grain texture side (pixels). Tiled across the screen at `GRAIN_TEXEL` px per texel.
pub const GRAIN_SIZE: usize = 128;
/// Strongest a single grain speck gets (alpha).
pub const GRAIN_ALPHA: f32 = 0.08;
/// Screen pixels per grain texel (finer = subtler).
pub const GRAIN_TEXEL: f32 = 1.5;
/// The grain pattern re-rolls this many times a second (film frame rate).
pub const GRAIN_FPS: f32 = 24.0;

/// RGBA noise that averages out to *nothing*: each speck is either black or
/// white, with alpha proportional to how far its noise value is from the
/// middle. So grain darkens and lightens in equal measure and never lays a
/// grey haze over the picture (a flat grey overlay washes blacks out).
/// Most specks are faint (the noise is squared toward zero).
pub fn grain_pixels(seed: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(GRAIN_SIZE * GRAIN_SIZE * 4);
    for i in 0..(GRAIN_SIZE * GRAIN_SIZE) as u32 {
        let v = crate::hud::hash01(i, seed) * 2.0 - 1.0;
        let a = (v * v * GRAIN_ALPHA * 255.0) as u8;
        let c = if v < 0.0 { 0 } else { 255 };
        out.extend_from_slice(&[c, c, c, a]);
    }
    out
}

/// The grain as an egui image. Premultiplied by hand, in gamma space (the
/// space egui blends in): `ColorImage::from_rgba_unmultiplied` premultiplies
/// in *linear* space, which turns a 6%-alpha white speck into a +27% one and
/// lays a bright haze over everything.
pub fn grain_image(seed: u32) -> egui::ColorImage {
    let pixels = grain_pixels(seed)
        .chunks(4)
        .map(|p| {
            let c = (p[0] as u32 * p[3] as u32 / 255) as u8;
            Color32::from_rgba_premultiplied(c, c, c, p[3])
        })
        .collect();
    egui::ColorImage {
        size: [GRAIN_SIZE, GRAIN_SIZE],
        pixels,
    }
}

/// Where the grain tile is offset this frame (UV units). Jumps GRAIN_FPS
/// times a second so it flickers like film, not like a sliding texture.
pub fn grain_offset(time: f32) -> (f32, f32) {
    let frame = (time * GRAIN_FPS) as u32;
    (crate::hud::hash01(frame, 11), crate::hud::hash01(frame, 12))
}

/// Draws the vignette and grain into `p`, beneath the HUD.
pub fn draw_overlay(
    p: &egui::Painter,
    screen: Rect,
    player_px: Pos2,
    strangeness: f32,
    time: f32,
    grain: Option<TextureId>,
    grain_strength: f32,
    vignette_strength: f32,
) {
    let half_diag = screen.size().length() * 0.5;
    let (inner, outer) = vignette_radii(strangeness, time);
    // Rings: dense through the fade, then one far ring past every corner.
    let steps = 10;
    let mut radii: Vec<f32> = (0..=steps)
        .map(|i| inner + (outer - inner) * i as f32 / steps as f32)
        .collect();
    radii.insert(0, inner * 0.5);
    radii.push(3.0);
    let mesh = vignette_mesh(player_px, half_diag, &radii, 48, [0, 0, 0], |d| {
        vignette_alpha(d, inner, outer) * vignette_strength.clamp(0.0, 1.0)
    });
    p.add(egui::Shape::mesh(mesh));
    let g = (grain_strength.clamp(0.0, 1.0) * 255.0) as u8;
    if let (Some(tex), true) = (grain, g > 0) {
        let (ox, oy) = grain_offset(time);
        let texel = GRAIN_TEXEL;
        let uv = Rect::from_min_size(
            Pos2::new(ox, oy),
            egui::vec2(
                screen.width() / (GRAIN_SIZE as f32 * texel),
                screen.height() / (GRAIN_SIZE as f32 * texel),
            ),
        );
        let mut m = Mesh::with_texture(tex);
        // Premultiplied tint: scales the whole speck, colour and alpha alike.
        m.add_rect_with_uv(screen, uv, Color32::from_rgba_premultiplied(g, g, g, g));
        p.add(egui::Shape::mesh(m));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sight_shrinks_with_strangeness_but_always_outreaches_a_chase() {
        assert!(sight_radius(1.0) < sight_radius(0.0));
        assert_eq!(sight_radius(5.0), sight_radius(1.0), "clamped");
        let chase = crate::gameplay::chase_radius(u32::MAX);
        let clear = sight_radius(1.0) * (1.0 - SIGHT_FADE);
        assert!(
            clear > chase + crate::gameplay::ENEMY_TOUCH_RADIUS,
            "an enemy that notices you must already be visible: clear {clear} vs chase {chase}"
        );
    }

    #[test]
    fn darkness_is_a_dim_version_of_the_fog() {
        let d = darkness([0.8, 0.4, 0.2]);
        assert!(d
            .iter()
            .zip([0.8, 0.4, 0.2])
            .all(|(a, b)| *a < b && *a >= 0.0));
    }

    #[test]
    fn vignette_is_clear_in_the_middle_and_dark_at_the_edge() {
        for s in [0.0, 0.5, 1.0] {
            for t in [0.0, 1.7, 9.0] {
                let (i, o) = vignette_radii(s, t);
                assert!(0.2 < i && i < o && o < 1.0, "{i} {o}");
                assert_eq!(vignette_alpha(0.0, i, o), 0.0);
                assert_eq!(vignette_alpha(i, i, o), 0.0);
                assert!((vignette_alpha(o, i, o) - VIGNETTE_MAX).abs() < 1e-6);
                assert!((vignette_alpha(2.0, i, o) - VIGNETTE_MAX).abs() < 1e-6);
                let mut prev = 0.0;
                for k in 0..=20 {
                    let a = vignette_alpha(k as f32 / 20.0, i, o);
                    assert!(a >= prev - 1e-6, "not monotonic");
                    prev = a;
                }
            }
        }
        assert!(
            vignette_radii(1.0, 0.0).0 < vignette_radii(0.0, 0.0).0,
            "tighter when strange"
        );
    }

    #[test]
    fn vignette_mesh_is_well_formed() {
        let radii = [0.2, 0.5, 1.0, 3.0];
        let m = vignette_mesh(Pos2::new(100.0, 50.0), 10.0, &radii, 16, [0, 0, 0], |d| {
            vignette_alpha(d, 0.2, 1.0)
        });
        assert_eq!(m.vertices.len(), 1 + 4 * 16);
        assert_eq!(m.indices.len(), 3 * (16 + 3 * 16 * 2));
        assert!(m.indices.iter().all(|&i| (i as usize) < m.vertices.len()));
        assert_eq!(m.vertices[0].color.a(), 0);
        assert!(m.vertices.last().unwrap().color.a() as f32 > VIGNETTE_MAX * 255.0 * 0.9);
        let far = m.vertices.last().unwrap().pos;
        assert!((far - Pos2::new(100.0, 50.0)).length() > 29.0);
    }

    #[test]
    fn grain_is_deterministic_faint_and_balanced() {
        let a = grain_pixels(1);
        assert_eq!(a.len(), GRAIN_SIZE * GRAIN_SIZE * 4);
        assert_eq!(a, grain_pixels(1));
        assert_ne!(a, grain_pixels(2));
        let px: Vec<&[u8]> = a.chunks(4).collect();
        assert!(px
            .iter()
            .all(|p| (p[0] == 0 || p[0] == 255) && p[0] == p[1] && p[1] == p[2]));
        let max_a = (GRAIN_ALPHA * 255.0) as u8;
        assert!(px.iter().all(|p| p[3] <= max_a));
        let mean_a = px.iter().map(|p| p[3] as f32).sum::<f32>() / px.len() as f32;
        assert!(
            mean_a < 0.45 * max_a as f32,
            "grain too heavy: mean alpha {mean_a}"
        );
        // Darkening and lightening balance out: no net grey cast.
        let signed: f32 = px
            .iter()
            .map(|p| {
                if p[0] == 0 {
                    -(p[3] as f32)
                } else {
                    p[3] as f32
                }
            })
            .sum::<f32>()
            / px.len() as f32;
        assert!(signed.abs() < 1.0, "net cast {signed}");
    }

    #[test]
    fn grain_image_adds_no_more_light_than_its_alpha() {
        let img = grain_image(7);
        assert_eq!(img.pixels.len(), GRAIN_SIZE * GRAIN_SIZE);
        let max_a = (GRAIN_ALPHA * 255.0) as u8;
        for p in &img.pixels {
            assert!(p.r() <= p.a() && p.a() <= max_a, "{p:?}");
        }
    }

    #[test]
    fn grain_jumps_between_film_frames_and_holds_within_one() {
        let f = 1.0 / GRAIN_FPS;
        assert_eq!(grain_offset(0.1 * f), grain_offset(0.9 * f));
        assert_ne!(grain_offset(0.5 * f), grain_offset(1.5 * f));
    }

    #[test]
    fn the_mesh_shader_knows_about_sight() {
        let frag = include_str!("../assets/shaders/mesh.frag");
        for u in [
            "uniform vec3 uPlayer;",
            "uniform float uSight;",
            "uniform float uLit;",
            "uniform vec3 uDark;",
        ] {
            assert!(frag.contains(u), "mesh.frag is missing `{u}`");
        }
    }
}
