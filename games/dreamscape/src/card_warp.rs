//! Card warp: a slow liquid wobble over a card's ART only, driven by the
//! card's dream theme. Names, plates and text stay flat.
//!
//! Pure: no GL, no clock, no global RNG. Callers pass the time and an eased
//! hover amount. `UiJuice::Off` is the identity; `reduced_motion` freezes the
//! field at t = 0 (a static soft warp, nothing moves).

use crate::dream::DreamTheme;
use crate::ui_motion::{smoothstep, UiJuice};
use crate::worth::mix;
use engine::ui::egui;
use engine::ui::egui::{epaint::Vertex, Color32, Mesh, Pos2, Rect};

/// Largest displacement, as a fraction of card size, at hover = 1 and Full.
pub const MAX_DISP: f32 = 0.035;

/// Resting strength as a fraction of the hovered strength (hover is x1.8).
const REST: f32 = 1.0 / 1.8;

#[derive(Clone, Copy, Debug)]
pub struct WarpParams {
    pub theme: DreamTheme,
    pub seed: u64,
    pub t: f32,
    /// 0..1, eased by the caller (see `ui_motion::Spring`).
    pub hover: f32,
    pub juice: UiJuice,
    pub reduced_motion: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WarpGrid {
    pub cols: usize,
    pub rows: usize,
}

impl WarpGrid {
    pub fn vertices(self) -> usize {
        (self.cols + 1) * (self.rows + 1)
    }
    pub fn cells(self) -> usize {
        self.cols * self.rows
    }
}

/// How a theme moves its cards.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Character {
    /// Spatial frequency (waves across the card).
    pub freq: f32,
    /// Strength, 0.4..=1.0 of `MAX_DISP`.
    pub amp: f32,
    /// Direction the art sags or drifts, each axis -1..=1.
    pub drift: (f32, f32),
    /// Swirl sign and strength, -1..=1.
    pub swirl: f32,
}

const fn ch(freq: f32, amp: f32, dx: f32, dy: f32, swirl: f32) -> Character {
    Character { freq, amp, drift: (dx, dy), swirl }
}

pub fn theme_character(theme: DreamTheme) -> Character {
    use DreamTheme::*;
    match theme {
        Lobby => ch(1.0, 0.5, 0.0, 0.3, 0.2),
        LiminalOffice => ch(0.8, 0.4, 0.0, 0.0, 0.1),
        VoidPlatforms => ch(0.9, 0.6, 0.0, 0.8, -0.3),
        Garden => ch(1.1, 0.7, 0.3, -0.5, 0.5),
        NightmareFactory => ch(1.4, 0.9, -0.4, 0.6, -0.6),
        Awakening => ch(0.7, 0.6, 0.0, -0.9, 0.2),
        CursedForest => ch(1.2, 0.8, 0.5, 0.5, -0.7),
        DrownedLibrary => ch(0.9, 0.8, 0.0, 0.7, 0.6),
        SkyStairs => ch(0.8, 0.6, 0.6, -0.6, -0.2),
        MirrorHall => ch(2.6, 0.4, 0.0, 0.0, 0.0),
        MyceliumGrove => ch(1.3, 0.7, 0.2, 0.4, 0.8),
        TheTunnel => ch(1.0, 0.8, 0.0, 0.2, 1.0),
        FractalCathedral => ch(1.8, 0.6, 0.0, -0.3, -0.5),
        Elfworks => ch(1.5, 0.5, 0.4, -0.4, 0.4),
        AfterimageFields => ch(1.1, 0.9, -0.7, 0.0, 0.3),
        SynesthesiaHall => ch(2.0, 0.8, 0.3, 0.3, -0.9),
        MeltingClockworks => ch(0.9, 1.0, 0.0, 1.0, 0.2),
        JellyfishSky => ch(0.8, 0.9, 0.2, -1.0, 0.5),
        WatchingWallpaper => ch(1.6, 0.7, 0.0, 0.1, -0.4),
        WhiteDissolve => ch(0.7, 1.0, 0.0, 0.5, 0.0),
    }
}

/// Mesh density for an art face about `size_px` wide: 3x4 small .. 8x10 large.
/// `Off` is a plain quad.
pub fn grid_for(size_px: f32, juice: UiJuice) -> WarpGrid {
    if juice == UiJuice::Off {
        return WarpGrid { cols: 1, rows: 1 };
    }
    let t = ((size_px - 60.0) / 300.0).clamp(0.0, 1.0);
    WarpGrid {
        cols: (3.0 + 5.0 * t).round() as usize,
        rows: (4.0 + 6.0 * t).round() as usize,
    }
}

fn phase(seed: u64, salt: u64) -> f32 {
    (mix(seed, salt) % 100_000) as f32 / 100_000.0 * std::f32::consts::TAU
}

/// Displacement `(du, dv)` of card-space point `(u, v)` (both 0..1), as
/// fractions of the card size. Zero on the border and when juice is Off.
pub fn offset(p: &WarpParams, u: f32, v: f32) -> (f32, f32) {
    if p.juice == UiJuice::Off {
        return (0.0, 0.0);
    }
    let c = theme_character(p.theme);
    let hover = p.hover.clamp(0.0, 1.0);
    let t = if p.reduced_motion { 0.0 } else { p.t * (1.0 + 0.5 * hover) };
    // Same strength on every card; hover x1.8; Full is 1.4 against Soft's 1.0.
    let mag = MAX_DISP * c.amp * (p.juice.scale() / 1.4) * (REST + (1.0 - REST) * hover);
    let (ph1, ph2, ph3) = (phase(p.seed, 1), phase(p.seed, 2), phase(p.seed, 3));
    let tau = std::f32::consts::TAU;
    let f = c.freq;
    // Slow sag toward the drift direction (one-sided, like a melt)...
    let sag = 0.5 + 0.5 * (f * tau * (0.9 * u + 0.4 * v) + t * 0.7 + ph1).sin();
    // ...a second, finer octave...
    let wob_a = (2.3 * f * tau * (0.8 * v - 0.5 * u) - t * 0.5 + ph2).sin();
    let wob_b = (2.3 * f * tau * (0.7 * u + 0.6 * v) + t * 0.45 + ph3).sin();
    // ...and a slower swirl about the card centre.
    let (cx, cy) = (u - 0.5, v - 0.5);
    let r = (cx * cx + cy * cy).sqrt();
    let sw = (t * 0.4 + ph3 + r * f * tau).sin();
    let du = mag * (0.5 * wob_a + 0.3 * c.drift.0 * sag + 0.2 * c.swirl * (-2.0 * cy) * sw);
    let dv = mag * (0.5 * wob_b + 0.3 * c.drift.1 * sag + 0.2 * c.swirl * (2.0 * cx) * sw);
    // Pin the border so frames and plates stay aligned.
    let edge = smoothstep(min4(u, 1.0 - u, v, 1.0 - v) / 0.12);
    (
        (du * edge).clamp(-MAX_DISP, MAX_DISP),
        (dv * edge).clamp(-MAX_DISP, MAX_DISP),
    )
}

fn min4(a: f32, b: f32, c: f32, d: f32) -> f32 {
    a.min(b).min(c).min(d)
}

/// Warped grid for `rect` in the caller's local space (caller rotates):
/// positions, uv in 0..1 card space (unwarped: the picture moves with the
/// mesh), and triangle indices (2 per cell).
pub fn mesh_points(rect: Rect, p: &WarpParams) -> (Vec<Pos2>, Vec<Pos2>, Vec<u32>) {
    let g = grid_for(rect.width(), p.juice);
    let mut pos = Vec::with_capacity(g.vertices());
    let mut uvs = Vec::with_capacity(g.vertices());
    for j in 0..=g.rows {
        let v = j as f32 / g.rows as f32;
        for i in 0..=g.cols {
            let u = i as f32 / g.cols as f32;
            let (du, dv) = offset(p, u, v);
            pos.push(Pos2::new(
                rect.left() + (u + du) * rect.width(),
                rect.top() + (v + dv) * rect.height(),
            ));
            uvs.push(Pos2::new(u, v));
        }
    }
    let stride = (g.cols + 1) as u32;
    let mut idx = Vec::with_capacity(g.cells() * 6);
    for j in 0..g.rows as u32 {
        for i in 0..g.cols as u32 {
            let a = j * stride + i;
            idx.extend_from_slice(&[a, a + 1, a + stride + 1, a, a + stride + 1, a + stride]);
        }
    }
    (pos, uvs, idx)
}

/// An egui mesh of the warped art. `uv` is the texture rect the face shows;
/// `to_screen` maps a local point to the screen (identity, or a card turn).
pub fn art_mesh(
    id: egui::TextureId,
    rect: Rect,
    uv: Rect,
    tint: Color32,
    p: &WarpParams,
    to_screen: impl Fn(Pos2) -> Pos2,
) -> Mesh {
    let (pos, uvs, idx) = mesh_points(rect, p);
    let mut mesh = Mesh::with_texture(id);
    for (pt, q) in pos.into_iter().zip(uvs) {
        mesh.vertices.push(Vertex {
            pos: to_screen(pt),
            uv: Pos2::new(
                uv.left() + q.x * uv.width(),
                uv.top() + q.y * uv.height(),
            ),
            color: tint,
        });
    }
    mesh.indices = idx;
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::ALL_THEMES;

    fn params() -> WarpParams {
        WarpParams {
            theme: DreamTheme::MeltingClockworks,
            seed: 42,
            t: 3.3,
            hover: 0.0,
            juice: UiJuice::Full,
            reduced_motion: false,
        }
    }

    const R: fn() -> Rect = || Rect::from_min_size(Pos2::new(10.0, 20.0), egui::vec2(120.0, 60.0));

    #[test]
    fn same_seed_and_time_is_deterministic() {
        let a = mesh_points(R(), &params());
        let b = mesh_points(R(), &params());
        assert_eq!(a, b);
    }

    #[test]
    fn different_seeds_differ() {
        let mut q = params();
        q.seed = 43;
        assert_ne!(mesh_points(R(), &params()).0, mesh_points(R(), &q).0);
    }

    #[test]
    fn time_moves_the_warp() {
        let mut q = params();
        q.t = 9.0;
        assert_ne!(mesh_points(R(), &params()).0, mesh_points(R(), &q).0);
    }

    #[test]
    fn offset_is_bounded_for_random_samples() {
        let mut z = 7u64;
        let mut next = || {
            z = mix(z, 99);
            (z % 1_000_000) as f32 / 1_000_000.0
        };
        for _ in 0..10_000 {
            let mut q = params();
            q.theme = ALL_THEMES[(next() * 20.0) as usize % 20];
            q.seed = (next() * 1e6) as u64;
            q.t = next() * 500.0;
            q.hover = next();
            q.juice = [UiJuice::Soft, UiJuice::Full][(next() * 2.0) as usize % 2];
            let (du, dv) = offset(&q, next(), next());
            assert!(du.abs() <= MAX_DISP && dv.abs() <= MAX_DISP, "{du} {dv}");
        }
    }

    #[test]
    fn border_offsets_are_exactly_zero() {
        for k in 0..=20 {
            let s = k as f32 / 20.0;
            for (u, v) in [(s, 0.0), (s, 1.0), (0.0, s), (1.0, s)] {
                assert_eq!(offset(&params(), u, v), (0.0, 0.0));
            }
        }
    }

    #[test]
    fn off_is_a_plain_quad_with_identity_points() {
        let mut q = params();
        q.juice = UiJuice::Off;
        let (pos, uv, idx) = mesh_points(R(), &q);
        let r = R();
        assert_eq!(grid_for(r.width(), UiJuice::Off), WarpGrid { cols: 1, rows: 1 });
        assert_eq!(pos, vec![r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()]);
        assert_eq!(uv.len(), 4);
        assert_eq!(idx.len(), 6);
    }

    #[test]
    fn reduced_motion_ignores_time_but_still_warps() {
        let mut a = params();
        a.reduced_motion = true;
        let mut b = a;
        b.t = 123.0;
        assert_eq!(mesh_points(R(), &a), mesh_points(R(), &b));
        let plain = {
            let mut o = a;
            o.juice = UiJuice::Off;
            mesh_points(R(), &o).0
        };
        assert_ne!(plain.len(), mesh_points(R(), &a).0.len());
        let (pos, uv, _) = mesh_points(R(), &a);
        let moved = pos.iter().zip(&uv).any(|(p, q)| {
            (p.x - (R().left() + q.x * R().width())).abs() > 1e-4
        });
        assert!(moved);
    }

    #[test]
    fn hover_strengthens_the_warp() {
        let mut lo = params();
        lo.reduced_motion = true;
        let mut hi = lo;
        hi.hover = 1.0;
        let sum = |q: &WarpParams| {
            let mut s = 0.0;
            for i in 1..20 {
                for j in 1..20 {
                    let (a, b) = offset(q, i as f32 / 20.0, j as f32 / 20.0);
                    s += a.abs() + b.abs();
                }
            }
            s
        };
        let ratio = sum(&hi) / sum(&lo);
        assert!((ratio - 1.8).abs() < 0.01, "ratio {ratio}");
    }

    #[test]
    fn grid_stays_within_budget() {
        for juice in [UiJuice::Soft, UiJuice::Full] {
            assert_eq!(grid_for(10.0, juice), WarpGrid { cols: 3, rows: 4 });
            assert_eq!(grid_for(2000.0, juice), WarpGrid { cols: 8, rows: 10 });
            let g = grid_for(150.0, juice);
            assert!(g.cols >= 3 && g.cols <= 8 && g.rows >= 4 && g.rows <= 10);
            assert!(g.cells() <= 80);
        }
        let (pos, uv, idx) = mesh_points(R(), &params());
        let g = grid_for(R().width(), UiJuice::Full);
        assert_eq!(pos.len(), g.vertices());
        assert_eq!(uv.len(), g.vertices());
        assert_eq!(idx.len(), g.cells() * 6);
        assert!(idx.iter().all(|&i| (i as usize) < pos.len()));
    }

    #[test]
    fn every_theme_has_a_character_in_range() {
        for t in ALL_THEMES {
            let c = theme_character(t);
            assert!((0.4..=1.0).contains(&c.amp), "{t:?}");
            assert!(c.freq > 0.0);
            assert!(c.swirl.abs() <= 1.0 && c.drift.0.abs() <= 1.0 && c.drift.1.abs() <= 1.0);
        }
    }

    #[test]
    fn melting_sags_down_and_jellyfish_drifts_up() {
        let mean_dv = |theme| {
            let mut q = params();
            q.theme = theme;
            q.reduced_motion = true;
            let mut s = 0.0;
            for seed in 0..40u64 {
                q.seed = seed;
                for i in 1..10 {
                    for j in 1..10 {
                        s += offset(&q, i as f32 / 10.0, j as f32 / 10.0).1;
                    }
                }
            }
            s
        };
        assert!(mean_dv(DreamTheme::MeltingClockworks) > 0.0);
        assert!(mean_dv(DreamTheme::JellyfishSky) < 0.0);
    }

    #[test]
    fn art_mesh_maps_uv_and_applies_the_turn() {
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 0.5));
        let m = art_mesh(egui::TextureId::default(), R(), uv, Color32::WHITE, &params(), |p| {
            p + egui::vec2(1.0, 0.0)
        });
        let (pos, _, idx) = mesh_points(R(), &params());
        assert_eq!(m.indices, idx);
        assert_eq!(m.vertices[0].pos, pos[0] + egui::vec2(1.0, 0.0));
        assert_eq!(m.vertices.last().unwrap().uv, Pos2::new(1.0, 0.5));
    }
}
