//! Foil pack wrappers, painted with egui (no image files): the theme's own
//! texture as the body, crimped edges, a medallion with a centre emblem.
//! See plans/PLAN-pick-a-pack.md.
//!
//! A wrapper never hints at rarity: rarity comes from how the run went, not
//! from the pack. The crimp carries one pip per card, nothing else.

use crate::dream::{DreamTheme, TexSpec, TEX_SIZE};
use crate::hud::rgba;
use crate::packs::Emblem;
use engine::ui::egui::{self, Color32, Pos2, Rect, Shape, Stroke, Vec2};
use std::collections::HashMap;

/// Wrapper textures, uploaded to egui on first sight and kept.
#[derive(Default)]
pub struct PackArt {
    cache: HashMap<(DreamTheme, u64), egui::TextureHandle>,
}

impl PackArt {
    fn texture(&mut self, ctx: &egui::Context, theme: DreamTheme, seed: u64) -> egui::TextureId {
        self.cache
            .entry((theme, seed))
            .or_insert_with(|| {
                let size = TEX_SIZE as usize;
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [size, size],
                    &body_spec(theme, seed).rgba(),
                );
                ctx.load_texture(
                    format!("pack-{theme:?}-{seed}"),
                    image,
                    egui::TextureOptions::NEAREST,
                )
            })
            .id()
    }

    /// Textures held (for tests).
    #[cfg(test)]
    fn len(&self) -> usize {
        self.cache.len()
    }
}

/// Natural wrapper size at scale 1.0 (taller than wide, like a card).
pub const PACK_SIZE: Vec2 = Vec2::new(220.0, 320.0);

/// The wrapper's body texture: one of the theme's own patterns in its own
/// wall colours, with its first accent folded in so the foil catches it.
pub fn body_spec(theme: DreamTheme, seed: u64) -> TexSpec {
    let spec = theme.spec();
    let mut palette: Vec<[u8; 3]> = spec.wall_colors.to_vec();
    if let Some(&a) = spec.accents.first() {
        palette.push(a);
    }
    TexSpec {
        pattern: spec.patterns[(seed % spec.patterns.len() as u64) as usize],
        palette,
        bands: 4.0,
        seed,
    }
}

/// The theme's accent: medallion rim, emblem colour.
fn accent(theme: DreamTheme) -> [u8; 3] {
    theme
        .spec()
        .accents
        .first()
        .copied()
        .unwrap_or([200, 200, 255])
}

/// How many pips a pack shows: one per card, 1 to 5.
pub fn pip_count(cards: usize) -> usize {
    cards.clamp(1, 5)
}

/// The medallion: centre and radius inside `rect`.
pub fn medallion(rect: Rect) -> (Pos2, f32) {
    let r = rect.width().min(rect.height()) * 0.30;
    (rect.center() - Vec2::new(0.0, rect.height() * 0.02), r)
}

/// How far from the centre an emblem may reach: inside the medallion.
pub fn emblem_reach(rect: Rect) -> f32 {
    medallion(rect).1 * 0.78
}

fn crimp_height(rect: Rect) -> f32 {
    rect.height() * 0.07
}

/// Draw one wrapper into `rect`.
/// - `pips`: card count, drawn on the lower crimp.
/// - `time`: seconds, drives the foil sweep and the moth's blink.
/// - `motion`: 0.0 (reduced motion: no sweep, no parallax) ..= 1.0.
/// - `tilt`: -1..1 on each axis, toward the cursor/stick.
#[allow(clippy::too_many_arguments)]
pub fn draw_pack(
    ctx: &egui::Context,
    p: &egui::Painter,
    rect: Rect,
    theme: DreamTheme,
    emblem: Emblem,
    seed: u64,
    pips: usize,
    time: f32,
    motion: f32,
    tilt: Vec2,
    art: &mut PackArt,
) {
    let motion = motion.clamp(0.0, 1.0);
    let tilt = Vec2::new(tilt.x.clamp(-1.0, 1.0), tilt.y.clamp(-1.0, 1.0)) * motion;
    let rgb = accent(theme);
    let clip = p.with_clip_rect(rect);

    // Drop shadow, then the foil body: the theme's own texture, shifted a
    // touch with tilt so it seems to sit under the wrapper.
    p.rect_filled(
        rect.translate(Vec2::new(5.0, 7.0)),
        6.0,
        Color32::from_black_alpha(110),
    );
    let tex = art.texture(ctx, theme, seed);
    let shift = tilt * 0.04;
    let uv = Rect::from_min_max(Pos2::new(0.05, 0.05) + shift, Pos2::new(0.95, 0.95) + shift);
    clip.image(tex, rect, uv, Color32::from_gray(205));
    // Darker toward the edges so the medallion reads.
    for i in 0..4u8 {
        let k = i as f32 / 4.0;
        clip.rect_stroke(
            rect.shrink(k * rect.width() * 0.04),
            6.0,
            Stroke::new(rect.width() * 0.02, Color32::from_black_alpha(60 - i * 12)),
        );
    }

    // Crimps: a dark strip with a row of teeth, top and bottom.
    let ch = crimp_height(rect);
    let teeth = 14usize;
    let tw = rect.width() / teeth as f32;
    let foil = rgba([rgb[0] / 3, rgb[1] / 3, rgb[2] / 3], 0.95);
    for (top, sign) in [(rect.top(), 1.0_f32), (rect.bottom(), -1.0_f32)] {
        let edge = top + sign * ch * 0.55;
        clip.rect_filled(
            Rect::from_two_pos(Pos2::new(rect.left(), top), Pos2::new(rect.right(), edge)),
            0.0,
            foil,
        );
        for i in 0..teeth {
            let x = rect.left() + i as f32 * tw;
            let tip = top + sign * ch;
            clip.add(Shape::convex_polygon(
                vec![
                    Pos2::new(x, edge),
                    Pos2::new(x + tw, edge),
                    Pos2::new(x + tw * 0.5, tip),
                ],
                foil,
                Stroke::NONE,
            ));
        }
    }

    // Foil sweep: a slanted highlight band. Static when motion is 0.
    let sweep = if motion > 0.0 {
        (time * 0.25 * motion + tilt.x * 0.3).rem_euclid(1.4) - 0.2
    } else {
        0.35
    };
    let bx = rect.left() + rect.width() * sweep;
    let slant = rect.height() * 0.35;
    for (w, a) in [(rect.width() * 0.22, 22u8), (rect.width() * 0.08, 34u8)] {
        clip.add(Shape::convex_polygon(
            vec![
                Pos2::new(bx - w * 0.5 + slant, rect.top()),
                Pos2::new(bx + w * 0.5 + slant, rect.top()),
                Pos2::new(bx + w * 0.5, rect.bottom()),
                Pos2::new(bx - w * 0.5, rect.bottom()),
            ],
            Color32::from_white_alpha(a),
            Stroke::NONE,
        ));
    }

    // Medallion and emblem (the emblem drifts a little with tilt: parallax).
    let (c, r) = medallion(rect);
    let c = c + tilt * Vec2::new(4.0, 3.0);
    clip.circle_filled(c, r, rgba([10, 5, 22], 0.88));
    clip.circle_stroke(c, r, Stroke::new(r * 0.07, rgba(rgb, 0.95)));
    clip.circle_stroke(c, r * 0.88, Stroke::new(1.0_f32, rgba(rgb, 0.45)));
    paint_emblem(&clip, c, emblem_reach(rect), emblem, rgb, time, tilt.x);

    // One pip per card on the lower crimp.
    let n = pip_count(pips);
    let pip = ch * 0.26;
    let gap = pip * 1.8;
    let row_w = (n - 1) as f32 * gap;
    for i in 0..n {
        let x = rect.center().x - row_w * 0.5 + i as f32 * gap;
        let y = rect.bottom() - ch * 0.3;
        clip.rect_filled(
            Rect::from_center_size(Pos2::new(x, y), Vec2::splat(pip)),
            0.0,
            rgba(rgb, 1.0),
        );
    }

    p.rect_stroke(rect, 6.0, Stroke::new(2.0_f32, rgba(rgb, 0.9)));
}

/// One emblem, centred on `c`, no larger than `reach` from the centre.
fn paint_emblem(
    p: &egui::Painter,
    c: Pos2,
    reach: f32,
    emblem: Emblem,
    rgb: [u8; 3],
    time: f32,
    look: f32,
) {
    let bright = [
        rgb[0].saturating_add(60),
        rgb[1].saturating_add(60),
        rgb[2].saturating_add(60),
    ];
    let dark = rgba([6, 0, 14], 1.0);
    match emblem {
        Emblem::Eye => {
            let cells = crate::pixels::eye_pixels(0.9, None, true);
            let (lo, hi) = cells
                .iter()
                .fold((i32::MAX, i32::MIN), |(l, h), q| (l.min(q.0), h.max(q.0)));
            let span = (hi - lo + 1).max(1) as f32;
            let px = (reach * 2.0 / span).floor().max(1.0);
            let mid = (lo + hi) as f32 * 0.5;
            crate::hud::paint_eye_at(p, c - Vec2::new(mid * px, 0.0), &cells, bright, rgb, px);
        }
        Emblem::Moth => {
            let px = (reach * 2.0 / 18.0).floor().max(1.0);
            let o = c - Vec2::new(9.0 * px, 7.0 * px);
            crate::shop_ui::paint_moth(p, o, px, rgb, bright, time, look);
        }
        Emblem::Door => {
            let w = reach * 1.1;
            let h = reach * 1.9;
            let base_y = c.y + h * 0.5;
            let arch_r = w * 0.5;
            let body = Rect::from_min_max(
                Pos2::new(c.x - w * 0.5, base_y - h + arch_r),
                Pos2::new(c.x + w * 0.5, base_y),
            );
            let top = Pos2::new(c.x, body.top());
            p.circle_filled(top, arch_r, rgba(rgb, 1.0));
            p.rect_filled(body, 0.0, rgba(rgb, 1.0));
            let inset = w * 0.16;
            p.circle_filled(top, arch_r - inset, dark);
            p.rect_filled(body.shrink2(Vec2::new(inset, 0.0)), 0.0, dark);
            p.rect_filled(
                Rect::from_center_size(
                    Pos2::new(c.x, base_y - inset * 0.4),
                    Vec2::new(w * 0.5, inset * 0.5),
                ),
                0.0,
                rgba(bright, 0.9),
            );
            p.circle_filled(
                Pos2::new(c.x + w * 0.2, c.y + h * 0.12),
                w * 0.05,
                rgba(bright, 1.0),
            );
        }
        Emblem::Shard => {
            let w = reach * 0.8;
            let h = reach;
            p.add(Shape::convex_polygon(
                vec![
                    Pos2::new(c.x, c.y - h),
                    Pos2::new(c.x + w, c.y),
                    Pos2::new(c.x, c.y + h),
                    Pos2::new(c.x - w, c.y),
                ],
                rgba(rgb, 1.0),
                Stroke::new(2.0_f32, rgba(bright, 1.0)),
            ));
            p.add(Shape::convex_polygon(
                vec![
                    Pos2::new(c.x, c.y - h),
                    Pos2::new(c.x + w, c.y),
                    Pos2::new(c.x, c.y),
                ],
                rgba(bright, 0.55),
                Stroke::NONE,
            ));
        }
        Emblem::Crescent => {
            let r = reach;
            p.circle_filled(c, r, rgba(bright, 1.0));
            p.circle_filled(
                c + Vec2::new(r * 0.45, -r * 0.1),
                r * 0.85,
                rgba([10, 5, 22], 1.0),
            );
            for (dx, dy) in [(0.55, -0.55), (0.75, 0.25)] {
                p.rect_filled(
                    Rect::from_center_size(c + Vec2::new(r * dx, r * dy), Vec2::splat(r * 0.12)),
                    0.0,
                    rgba(bright, 1.0),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::ALL_THEMES;

    #[allow(clippy::too_many_arguments)]
    fn run(
        ctx: &egui::Context,
        art: &mut PackArt,
        size: Vec2,
        theme: DreamTheme,
        emblem: Emblem,
        time: f32,
        motion: f32,
        tilt: Vec2,
    ) -> Vec<egui::epaint::ClippedShape> {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            ..Default::default()
        };
        let rect = Rect::from_min_size(Pos2::new(40.0, 30.0), size);
        ctx.run(input, |ctx| {
            let p = ctx.layer_painter(egui::LayerId::background());
            draw_pack(ctx, &p, rect, theme, emblem, 7, 3, time, motion, tilt, art);
        })
        .shapes
    }

    #[test]
    fn every_theme_and_emblem_draws_at_two_sizes() {
        let ctx = egui::Context::default();
        let mut art = PackArt::default();
        for &theme in ALL_THEMES.iter() {
            for emblem in Emblem::ALL {
                for size in [PACK_SIZE, PACK_SIZE * 0.5] {
                    let shapes = run(&ctx, &mut art, size, theme, emblem, 1.0, 1.0, Vec2::ZERO);
                    assert!(
                        shapes.len() > 20,
                        "{theme:?}/{emblem:?} drew {}",
                        shapes.len()
                    );
                }
            }
        }
    }

    #[test]
    fn the_same_texture_is_uploaded_once() {
        let ctx = egui::Context::default();
        let mut art = PackArt::default();
        let t = ALL_THEMES[0];
        run(
            &ctx,
            &mut art,
            PACK_SIZE,
            t,
            Emblem::Eye,
            0.0,
            1.0,
            Vec2::ZERO,
        );
        run(
            &ctx,
            &mut art,
            PACK_SIZE,
            t,
            Emblem::Moth,
            3.0,
            1.0,
            Vec2::ZERO,
        );
        assert_eq!(art.len(), 1);
        run(
            &ctx,
            &mut art,
            PACK_SIZE,
            ALL_THEMES[1],
            Emblem::Eye,
            0.0,
            1.0,
            Vec2::ZERO,
        );
        assert_eq!(art.len(), 2);
    }

    #[test]
    fn reduced_motion_is_static_and_full_motion_moves() {
        let ctx = egui::Context::default();
        let mut art = PackArt::default();
        let t = ALL_THEMES[0];
        // The eye emblem has no clock of its own, so any difference is the foil.
        let still = |art: &mut PackArt, time, tilt| {
            format!(
                "{:?}",
                run(&ctx, art, PACK_SIZE, t, Emblem::Eye, time, 0.0, tilt)
            )
        };
        let a = still(&mut art, 0.0, Vec2::ZERO);
        let b = still(&mut art, 2.7, Vec2::new(1.0, -1.0));
        assert_eq!(a, b);
        let moving = |art: &mut PackArt, time| {
            format!(
                "{:?}",
                run(&ctx, art, PACK_SIZE, t, Emblem::Eye, time, 1.0, Vec2::ZERO)
            )
        };
        let a = moving(&mut art, 0.0);
        let b = moving(&mut art, 1.5);
        assert_ne!(a, b);
    }

    #[test]
    fn pips_are_one_per_card_clamped() {
        assert_eq!(pip_count(0), 1);
        assert_eq!(pip_count(3), 3);
        assert_eq!(pip_count(40), 5);
    }

    #[test]
    fn the_medallion_and_emblem_sit_inside_the_wrapper() {
        for size in [PACK_SIZE, PACK_SIZE * 0.5, Vec2::new(90.0, 130.0)] {
            let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), size);
            let (c, r) = medallion(rect);
            let ch = crimp_height(rect);
            let inner =
                Rect::from_min_max(rect.min + Vec2::new(0.0, ch), rect.max - Vec2::new(0.0, ch));
            assert!(inner.contains(c + Vec2::splat(r) * 0.999), "{size:?}");
            assert!(inner.contains(c - Vec2::splat(r) * 0.999), "{size:?}");
            assert!(emblem_reach(rect) < r);
        }
    }

    #[test]
    fn every_theme_has_a_body_texture() {
        for &t in ALL_THEMES.iter() {
            let spec = body_spec(t, 3);
            assert!(!spec.palette.is_empty());
            assert_eq!(spec.rgba().len(), (TEX_SIZE * TEX_SIZE * 4) as usize);
        }
    }
}
