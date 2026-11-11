//! Relic art (R3): wear mask, etch glyph, rarity frame and foil sweep on top
//! of the relic's own `TexSpec` texture, plus a flat-shaded weapon icon.
//!
//! Pure CPU: bytes in, bytes out, deterministic from the relic's seed. The
//! cache uploads to egui once per `(theme, seed)`; the warp (M1) and the
//! relic screens (R6) draw the cached texture.

use crate::dream::{build_mesh, DreamTheme, Shape, TexSpec, TEX_SIZE};
use crate::pack_art;
use crate::relic_types::{Relic, RelicRarity};
use crate::relics::{etch_glyph, wear_tier, WearTier, ETCH_GLYPHS};
use crate::worth::mix;
use engine::ui::egui;
use std::collections::{HashMap, VecDeque};

const WEAR_PIXEL_SALT: u64 = 0x57EA_12D0_0000_0001;
const WEAR_SLOT_SALT: u64 = 0x57EA_12D0_0000_0002;
/// Etch mark opacity out of 255: faint, it should read as scratched in.
const ETCH_ALPHA: u32 = 90;
/// Border thickness in texels.
const FRAME: usize = 2;
/// Textures kept before the oldest is dropped.
pub const CACHE_CAP: usize = 48;

/// The base texture for a theme + seed (what `pull` stores in `Relic::art`).
pub fn base_spec(theme: DreamTheme, seed: u64) -> TexSpec {
    pack_art::body_spec(theme, seed)
}

/// (spatter percent, dim percent) for a wear tier.
fn wear_amounts(tier: WearTier) -> (u64, u32) {
    match tier {
        WearTier::Clean => (0, 0),
        WearTier::Light => (8, 6),
        WearTier::Heavy => (22, 18),
        WearTier::Ruined => (40, 32),
    }
}

fn accent(theme: DreamTheme) -> [u8; 3] {
    theme.spec().accents.first().copied().unwrap_or([200, 200, 255])
}

fn frame_colour(r: RelicRarity) -> [u8; 3] {
    match r {
        RelicRarity::Cracked => [96, 96, 104],
        RelicRarity::Worn => [168, 112, 64],
        RelicRarity::Pristine => [176, 200, 232],
        RelicRarity::Mythic => [240, 200, 96],
    }
}

/// Wear: chunky 2x2 spatter in one palette colour, then an overall dim.
fn apply_wear(buf: &mut [u8], tier: WearTier, palette: &[[u8; 3]], seed: u64) {
    let (spatter, dim) = wear_amounts(tier);
    if spatter == 0 {
        return;
    }
    let slot = if palette.is_empty() {
        [20, 20, 24]
    } else {
        // Darkest-leaning: pick a slot, then halve it so spatter reads as grime.
        let c = palette[(mix(seed, WEAR_SLOT_SALT) % palette.len() as u64) as usize];
        [c[0] / 2, c[1] / 2, c[2] / 2]
    };
    let n = TEX_SIZE as usize;
    for y in 0..n {
        for x in 0..n {
            let cell = ((y / 2) * n + x / 2) as u64;
            let i = (y * n + x) * 4;
            if mix(seed, WEAR_PIXEL_SALT ^ cell.wrapping_mul(0x9E37)) % 100 < spatter {
                buf[i..i + 3].copy_from_slice(&slot);
            }
            for c in 0..3 {
                buf[i + c] = (buf[i + c] as u32 * (100 - dim) / 100) as u8;
            }
        }
    }
}

/// Whether glyph `id` marks cell (x, y) of its 16x16 design grid.
fn glyph_cell(id: u8, x: i32, y: i32) -> bool {
    let (dx, dy) = (x - 8, y - 8);
    let r = ((dx * dx + dy * dy) as f32).sqrt();
    let inside = (3..=13).contains(&x) && (3..=13).contains(&y);
    match id % ETCH_GLYPHS as u8 {
        0 => inside && (dx == 0 || dy == 0),
        1 => (4.2..5.6).contains(&r),
        2 => dx.abs() + dy.abs() == 6,
        3 => y == 8 && matches!(x, 3 | 8 | 13),
        4 => inside && (y == 13 || (dx.abs() * 2 == 13 - y - 1 + 1 && y < 13) || (dx.abs() * 2 == 12 - y && y < 13)),
        5 => inside && matches!(y, 4 | 8 | 12),
        6 => inside && dx.abs() == dy.abs(),
        7 => (4.2..5.6).contains(&r) && dy <= 0,
        8 => {
            let t = (dx as f32 / 6.0).abs();
            t <= 1.0 && ((dy.abs() as f32) - 4.0 * (1.0 - t * t)).abs() < 0.7
        }
        _ => matches!(x, 4 | 8 | 12) && matches!(y, 4 | 8 | 12),
    }
}

/// The glyph as a 16x16 mask (tests).
#[cfg(test)]
pub fn glyph_mask(id: u8) -> [bool; 256] {
    let mut m = [false; 256];
    for y in 0..16 {
        for x in 0..16 {
            m[(y * 16 + x) as usize] = glyph_cell(id, x, y);
        }
    }
    m
}

/// Etch: the glyph at 2x scale, centred, in the accent colour at low alpha.
fn apply_etch(buf: &mut [u8], glyph: u8, colour: [u8; 3]) {
    let n = TEX_SIZE as usize;
    let off = (n - 32) / 2;
    for gy in 0..16 {
        for gx in 0..16 {
            if !glyph_cell(glyph, gx, gy) {
                continue;
            }
            for sy in 0..2 {
                for sx in 0..2 {
                    let x = off + gx as usize * 2 + sx;
                    let y = off + gy as usize * 2 + sy;
                    let i = (y * n + x) * 4;
                    for c in 0..3 {
                        buf[i + c] = ((buf[i + c] as u32 * (255 - ETCH_ALPHA)
                            + colour[c] as u32 * ETCH_ALPHA)
                            / 255) as u8;
                    }
                }
            }
        }
    }
}

fn apply_frame(buf: &mut [u8], colour: [u8; 3]) {
    let n = TEX_SIZE as usize;
    for y in 0..n {
        for x in 0..n {
            if x < FRAME || y < FRAME || x >= n - FRAME || y >= n - FRAME {
                let i = (y * n + x) * 4;
                buf[i..i + 3].copy_from_slice(&colour);
                buf[i + 3] = 255;
            }
        }
    }
}

/// `TEX_SIZE`² RGBA8: base texture, wear, etch, rarity frame. Deterministic.
pub fn rgba(relic: &Relic) -> Vec<u8> {
    let mut buf = relic.art.rgba();
    apply_wear(&mut buf, wear_tier(relic.seed), &relic.art.palette, relic.seed);
    apply_etch(&mut buf, etch_glyph(relic.seed), accent(relic.theme));
    apply_frame(&mut buf, frame_colour(relic.rarity));
    buf
}

/// Foil highlight 0..1 at texture coords (u, v) in 0..1. A soft diagonal band
/// that travels with `t` (seconds) and leans with the hover tilt.
pub fn foil_sweep(t: f32, u: f32, v: f32, tilt: (f32, f32)) -> f32 {
    let pos = (u + v) * 0.5 + tilt.0 * 0.25 - tilt.1 * 0.25;
    let centre = (t * 0.35).rem_euclid(1.6) - 0.3;
    let d = (pos - centre) / 0.12;
    let band = (-d * d).exp();
    // A faint second, narrower glint trailing the main band.
    let d2 = (pos - centre + 0.22) / 0.04;
    let glint = 0.35 * (-d2 * d2).exp();
    let out = band + glint;
    if out.is_finite() {
        out.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Textures uploaded to egui, keyed `(theme, seed)`, oldest dropped past the cap.
#[derive(Default)]
pub struct RelicArtCache {
    cache: HashMap<(DreamTheme, u64), egui::TextureHandle>,
    order: VecDeque<(DreamTheme, u64)>,
}

impl RelicArtCache {
    pub fn get(&mut self, ctx: &egui::Context, relic: &Relic) -> egui::TextureId {
        let key = (relic.theme, relic.seed);
        if let Some(h) = self.cache.get(&key) {
            return h.id();
        }
        while self.cache.len() >= CACHE_CAP {
            match self.order.pop_front() {
                Some(old) => {
                    self.cache.remove(&old);
                }
                None => break,
            }
        }
        let size = TEX_SIZE as usize;
        let image = egui::ColorImage::from_rgba_unmultiplied([size, size], &rgba(relic));
        let handle = ctx.load_texture(
            format!("relic-{:?}-{}", relic.theme, relic.seed),
            image,
            egui::TextureOptions::NEAREST,
        );
        let id = handle.id();
        self.cache.insert(key, handle);
        self.order.push_back(key);
        id
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.cache.len()
    }
}

/// Low-res flat-shaded silhouette of a weapon shape: `size`² RGBA8, transparent
/// background, fixed three-quarter view, painter's-order triangles.
pub fn weapon_icon_rgba(shape: Shape, tint: [u8; 3], size: usize) -> Vec<u8> {
    let mut out = vec![0u8; size * size * 4];
    if size == 0 {
        return out;
    }
    let mesh = build_mesh(shape);
    let (yaw, pitch) = (0.6f32, 0.45f32);
    let (sy, cy, sp, cp) = (yaw.sin(), yaw.cos(), pitch.sin(), pitch.cos());
    let rot = |p: [f32; 3]| -> [f32; 3] {
        let (x, y, z) = (p[0] * cy + p[2] * sy, p[1], -p[0] * sy + p[2] * cy);
        [x, y * cp - z * sp, y * sp + z * cp]
    };
    let pts: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| rot(v.position)).collect();
    // Fit the projected bounds into the icon with a small margin.
    let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
    for p in &pts {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1e-3);
    let scale = size as f32 * 0.84 / span;
    let centre = [(lo[0] + hi[0]) * 0.5, (lo[1] + hi[1]) * 0.5];
    let to_px = |p: [f32; 3]| -> [f32; 2] {
        [
            size as f32 * 0.5 + (p[0] - centre[0]) * scale,
            size as f32 * 0.5 - (p[1] - centre[1]) * scale,
        ]
    };
    let mut tris: Vec<(f32, [usize; 3])> = mesh
        .indices
        .chunks_exact(3)
        .map(|t| {
            let i = [t[0] as usize, t[1] as usize, t[2] as usize];
            let z = (pts[i[0]][2] + pts[i[1]][2] + pts[i[2]][2]) / 3.0;
            (z, i)
        })
        .collect();
    // Camera looks down -z in rotated space, so larger z is nearer: draw far first.
    tris.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let light = {
        let l = [-0.4f32, 0.7, 0.6];
        let n = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
        [l[0] / n, l[1] / n, l[2] / n]
    };
    for (_, i) in tris {
        let (a, b, c) = (pts[i[0]], pts[i[1]], pts[i[2]]);
        let (u, v) = (
            [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
            [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
        );
        let mut n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len < 1e-9 {
            continue;
        }
        for k in &mut n {
            *k /= len;
        }
        // Wound either way: shade by the absolute facing so no face goes black.
        let lit = (n[0] * light[0] + n[1] * light[1] + n[2] * light[2]).abs();
        let shade = 0.4 + 0.6 * lit;
        let col = [
            (tint[0] as f32 * shade) as u8,
            (tint[1] as f32 * shade) as u8,
            (tint[2] as f32 * shade) as u8,
        ];
        let (pa, pb, pc) = (to_px(a), to_px(b), to_px(c));
        let min_x = pa[0].min(pb[0]).min(pc[0]).floor().max(0.0) as usize;
        let max_x = (pa[0].max(pb[0]).max(pc[0]).ceil().max(0.0) as usize).min(size - 1);
        let min_y = pa[1].min(pb[1]).min(pc[1]).floor().max(0.0) as usize;
        let max_y = (pa[1].max(pb[1]).max(pc[1]).ceil().max(0.0) as usize).min(size - 1);
        let area = (pb[0] - pa[0]) * (pc[1] - pa[1]) - (pb[1] - pa[1]) * (pc[0] - pa[0]);
        if area.abs() < 1e-6 {
            continue;
        }
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let p = [x as f32 + 0.5, y as f32 + 0.5];
                let w0 = ((pb[0] - p[0]) * (pc[1] - p[1]) - (pb[1] - p[1]) * (pc[0] - p[0])) / area;
                let w1 = ((pc[0] - p[0]) * (pa[1] - p[1]) - (pc[1] - p[1]) * (pa[0] - p[0])) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    let o = (y * size + x) * 4;
                    out[o..o + 3].copy_from_slice(&col);
                    out[o + 3] = 255;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::ALL_SHAPES;

    fn relic(theme: DreamTheme, seed: u64, rarity: RelicRarity) -> Relic {
        Relic {
            id: 1,
            theme,
            seed,
            rarity,
            foil: false,
            name: String::new(),
            flavour: String::new(),
            effect: Default::default(),
            art: base_spec(theme, seed),
            weapon: None,
        }
    }

    fn theme() -> DreamTheme {
        crate::relics::pullable_themes()[0]
    }

    fn seed_with_tier(t: WearTier) -> u64 {
        (0u64..10_000).find(|&s| wear_tier(s) == t).unwrap()
    }

    #[test]
    fn same_relic_same_bytes_and_seeds_differ() {
        let a = rgba(&relic(theme(), 7, RelicRarity::Worn));
        assert_eq!(a, rgba(&relic(theme(), 7, RelicRarity::Worn)));
        assert_eq!(a.len(), (TEX_SIZE * TEX_SIZE * 4) as usize);
        assert_ne!(a, rgba(&relic(theme(), 8, RelicRarity::Worn)));
    }

    #[test]
    fn wear_tiers_differ_in_a_measurable_fraction() {
        let t = theme();
        let spec = base_spec(t, 3);
        let diff = |tier: WearTier| {
            let clean = spec.rgba();
            let mut worn = clean.clone();
            apply_wear(&mut worn, tier, &spec.palette, 3);
            let d = clean.chunks(4).zip(worn.chunks(4)).filter(|(a, b)| a != b).count();
            d as f32 / (clean.len() / 4) as f32
        };
        assert_eq!(diff(WearTier::Clean), 0.0);
        assert!(diff(WearTier::Light) > 0.05);
        assert!(diff(WearTier::Ruined) >= diff(WearTier::Light));
        assert!(diff(WearTier::Ruined) > 0.5, "ruined {}", diff(WearTier::Ruined));
        let loss = |tier: WearTier| {
            let clean = spec.rgba();
            let mut worn = clean.clone();
            apply_wear(&mut worn, tier, &spec.palette, 3);
            clean.iter().zip(&worn).map(|(a, b)| (*a as i64 - *b as i64).abs()).sum::<i64>()
        };
        assert!(loss(WearTier::Ruined) > loss(WearTier::Heavy));
        assert!(loss(WearTier::Heavy) > loss(WearTier::Light));
        assert!(loss(WearTier::Light) > 0);
        // Through the public path too.
        let s_clean = seed_with_tier(WearTier::Clean);
        let s_ruin = seed_with_tier(WearTier::Ruined);
        let n = (TEX_SIZE * TEX_SIZE) as usize;
        for s in [s_clean, s_ruin] {
            let r = relic(t, s, RelicRarity::Worn);
            let changed = r.art.rgba().chunks(4).zip(rgba(&r).chunks(4)).filter(|(a, b)| a != b).count();
            if s == s_clean {
                assert!(changed < n / 4);
            } else {
                assert!(changed > n / 2);
            }
        }
    }

    #[test]
    fn every_etch_glyph_leaves_accent_pixels() {
        let acc = [255, 255, 255];
        for id in 0..ETCH_GLYPHS as u8 {
            assert!(glyph_mask(id).iter().any(|&b| b), "glyph {id} empty");
            let mut buf = vec![0u8; (TEX_SIZE * TEX_SIZE * 4) as usize];
            apply_etch(&mut buf, id, acc);
            let marked = buf.chunks(4).filter(|p| p[0] > 0).count();
            assert!(marked > 0, "glyph {id} left nothing");
        }
        // The ten glyphs are all distinct designs.
        for a in 0..ETCH_GLYPHS as u8 {
            for b in a + 1..ETCH_GLYPHS as u8 {
                assert_ne!(glyph_mask(a), glyph_mask(b), "{a} == {b}");
            }
        }
    }

    #[test]
    fn rarity_frames_differ() {
        let a = rgba(&relic(theme(), 5, RelicRarity::Cracked));
        let b = rgba(&relic(theme(), 5, RelicRarity::Mythic));
        assert_eq!(&a[0..3], &frame_colour(RelicRarity::Cracked));
        assert_eq!(&b[0..3], &frame_colour(RelicRarity::Mythic));
    }

    #[test]
    fn foil_sweep_is_bounded_and_moves() {
        let mut seen_diff = false;
        for ti in 0..40 {
            let t = ti as f32 * 0.17;
            for ui in 0..=10 {
                for vi in 0..=10 {
                    let (u, v) = (ui as f32 / 10.0, vi as f32 / 10.0);
                    let f = foil_sweep(t, u, v, (0.3, -0.2));
                    assert!((0.0..=1.0).contains(&f), "{f}");
                    if (f - foil_sweep(t + 0.4, u, v, (0.3, -0.2))).abs() > 0.05 {
                        seen_diff = true;
                    }
                }
            }
        }
        assert!(seen_diff);
        assert!(foil_sweep(f32::NAN, 0.5, 0.5, (0.0, 0.0)) == 0.0);
        // Tilt moves the band too.
        let any = (0..=10).any(|i| {
            let u = i as f32 / 10.0;
            (foil_sweep(1.0, u, 0.5, (0.0, 0.0)) - foil_sweep(1.0, u, 0.5, (1.0, 0.0))).abs() > 0.05
        });
        assert!(any);
    }

    #[test]
    fn cache_returns_the_same_handle_and_stays_bounded() {
        let ctx = egui::Context::default();
        let mut c = RelicArtCache::default();
        let r = relic(theme(), 9, RelicRarity::Pristine);
        assert_eq!(c.get(&ctx, &r), c.get(&ctx, &r));
        assert_eq!(c.len(), 1);
        for s in 0..(CACHE_CAP as u64 + 10) {
            c.get(&ctx, &relic(theme(), 100 + s, RelicRarity::Worn));
        }
        assert!(c.len() <= CACHE_CAP);
    }

    #[test]
    fn weapon_icon_is_non_empty_for_all_shapes() {
        assert_eq!(ALL_SHAPES.len(), 11);
        for shape in ALL_SHAPES {
            let icon = weapon_icon_rgba(shape, [200, 120, 255], 24);
            assert_eq!(icon.len(), 24 * 24 * 4);
            let filled = icon.chunks(4).filter(|p| p[3] == 255).count();
            assert!(filled > 20, "{shape:?} only {filled} px");
            assert!(filled < 24 * 24, "{shape:?} fills the whole icon");
            assert_eq!(icon, weapon_icon_rgba(shape, [200, 120, 255], 24));
        }
        assert!(weapon_icon_rgba(Shape::Cube, [1, 2, 3], 0).is_empty());
    }
}
