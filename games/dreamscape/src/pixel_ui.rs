//! Shared pieces of the game's pixel look for screens: chipped panels,
//! cell sprites, keycaps, chromatic-split text, scanlines, pixel glyphs.
//! Everything is built from whole "cells" (`cell(u)` pixels), so panels and
//! icons share one grid and nothing is rounded or smooth.

use crate::hud::rgba;
use crate::pixels::icons;
use engine::ui::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};

/// The card frame's chipped-corner dark.
pub const CHIP: [u8; 3] = [8, 4, 20];
/// The HUD's chromatic ghosts (cyan / magenta).
pub const GHOST_A: [u8; 3] = [90, 255, 255];
pub const GHOST_B: [u8; 3] = [255, 70, 200];

/// UI scale for a screen: 1.0 at 720p, shrinking and growing a little.
pub fn scale(screen: Rect) -> f32 {
    (screen.height() / 720.0).clamp(0.6, 1.5)
}

/// One cell, in pixels, at UI scale `u`.
pub fn cell(u: f32) -> f32 {
    (3.0 * u).round().max(2.0)
}

fn cells(r: Rect, x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(r.min + Vec2::new(x, y), Vec2::new(w, h))
}

/// A panel with a hard border and chipped corners, like a card's frame.
/// `c` is the cell size; `alpha` fades the border.
pub fn panel(p: &egui::Painter, r: Rect, edge: [u8; 3], fill: Color32, c: f32, alpha: f32) {
    let (w, h) = (r.width(), r.height());
    if w < 4.0 * c || h < 4.0 * c {
        p.rect_filled(r, 0.0, fill);
        p.rect_stroke(r, 0.0, Stroke::new(c.min(2.0), rgba(edge, alpha)));
        return;
    }
    // The body, minus the four corner cells.
    p.rect_filled(cells(r, c, 0.0, w - 2.0 * c, h), 0.0, fill);
    p.rect_filled(cells(r, 0.0, c, c, h - 2.0 * c), 0.0, fill);
    p.rect_filled(cells(r, w - c, c, c, h - 2.0 * c), 0.0, fill);
    let e = rgba(edge, alpha);
    p.rect_filled(cells(r, c, 0.0, w - 2.0 * c, c), 0.0, e);
    p.rect_filled(cells(r, c, h - c, w - 2.0 * c, c), 0.0, e);
    p.rect_filled(cells(r, 0.0, c, c, h - 2.0 * c), 0.0, e);
    p.rect_filled(cells(r, w - c, c, c, h - 2.0 * c), 0.0, e);
    // A faint inner line, as on the cards.
    p.rect_stroke(
        r.shrink(c * 2.0),
        0.0,
        Stroke::new(1.0_f32, rgba(edge, alpha * 0.3)),
    );
}

/// Four corner brackets round `r`: the cursor.
pub fn brackets(p: &egui::Painter, r: Rect, c: f32, color: Color32) {
    let arm = c * 4.0;
    for (corner, dx, dy) in [
        (r.left_top(), 1.0_f32, 1.0_f32),
        (r.right_top(), -1.0, 1.0),
        (r.left_bottom(), 1.0, -1.0),
        (r.right_bottom(), -1.0, -1.0),
    ] {
        let h = Rect::from_two_pos(corner, corner + Vec2::new(dx * arm, dy * c));
        let v = Rect::from_two_pos(corner, corner + Vec2::new(dx * c, dy * arm));
        p.rect_filled(h, 0.0, color);
        p.rect_filled(v, 0.0, color);
    }
}

/// A 9x9 icon from `pixels::icons`, centred on `centre`, `px` pixels a cell.
pub fn icon(
    p: &egui::Painter,
    centre: Pos2,
    rows: &[&str; icons::SIZE],
    px: f32,
    base: [u8; 3],
    accent: [u8; 3],
    alpha: f32,
) {
    for (x, y, acc) in icons::cells(rows) {
        let at = centre + Vec2::new(x as f32, y as f32) * px;
        p.rect_filled(
            Rect::from_center_size(at, Vec2::splat(px)),
            0.0,
            rgba(if acc { accent } else { base }, alpha),
        );
    }
}

/// The accent used for an icon's `+` cells: a lighter tone of its base.
pub fn lighter(rgb: [u8; 3]) -> [u8; 3] {
    rgb.map(|v| v / 2 + 110)
}

/// Text with the HUD's chromatic split: a cyan and a magenta ghost either
/// side of the core. `split` is the ghosts' offset in pixels.
#[allow(clippy::too_many_arguments)]
pub fn split_text(
    p: &egui::Painter,
    at: Pos2,
    align: Align2,
    text: &str,
    size: f32,
    core: Color32,
    split: f32,
    alpha: f32,
) {
    let font = FontId::monospace(size);
    for (dx, rgb) in [(-split, GHOST_A), (split, GHOST_B)] {
        p.text(
            at + Vec2::new(dx, 0.0),
            align,
            text,
            font.clone(),
            rgba(rgb, 0.55 * alpha),
        );
    }
    p.text(at, align, text, font, core);
}

/// Faint CRT scanlines across `r`: a dark line a cell high every two cells.
pub fn scanlines(p: &egui::Painter, r: Rect, c: f32, alpha: u8) {
    let mut y = r.top() + c * 0.65;
    while y < r.bottom() {
        p.rect_filled(
            Rect::from_min_size(Pos2::new(r.left(), y), Vec2::new(r.width(), c * 0.35)),
            0.0,
            Color32::from_black_alpha(alpha),
        );
        y += c * 2.0;
    }
}

/// Diagonal hatching in cells, for locked or unavailable things.
pub fn hatch(p: &egui::Painter, r: Rect, c: f32, color: Color32) {
    let p = p.with_clip_rect(r);
    let step = c * 4.0;
    let mut x = r.left() - r.height();
    while x < r.right() {
        // One stair-stepped diagonal.
        let n = (r.height() / c).ceil() as i32;
        for i in 0..n {
            p.rect_filled(
                Rect::from_min_size(
                    Pos2::new(x + i as f32 * c, r.bottom() - (i + 1) as f32 * c),
                    Vec2::splat(c),
                ),
                0.0,
                color,
            );
        }
        x += step;
    }
}

/// A small keycap with `text` inside.
pub fn keycap(
    p: &egui::Painter,
    centre: Pos2,
    text: &str,
    size: f32,
    c: f32,
    rgb: [u8; 3],
    alpha: f32,
) {
    let galley = p.layout_no_wrap(text.to_string(), FontId::monospace(size), Color32::WHITE);
    let w = galley.size().x + c * 3.0;
    let h = galley.size().y + c * 1.5;
    let r = Rect::from_center_size(centre, Vec2::new(w.max(h), h));
    p.rect_filled(
        r.translate(Vec2::new(0.0, c * 0.7)),
        0.0,
        Color32::from_black_alpha(140),
    );
    p.rect_filled(r, 0.0, rgba([22, 14, 44], 0.95 * alpha));
    p.rect_stroke(r, 0.0, Stroke::new(1.0_f32, rgba(rgb, 0.8 * alpha)));
    p.text(
        r.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::monospace(size),
        rgba(rgb, alpha),
    );
}

/// A plus sign, 5x5 cells, centred.
pub fn plus(p: &egui::Painter, centre: Pos2, c: f32, color: Color32) {
    p.rect_filled(
        Rect::from_center_size(centre, Vec2::new(5.0 * c, c)),
        0.0,
        color,
    );
    p.rect_filled(
        Rect::from_center_size(centre, Vec2::new(c, 5.0 * c)),
        0.0,
        color,
    );
}

/// A right-pointing arrow, 7 wide and 5 tall in cells, centred.
pub fn arrow(p: &egui::Painter, centre: Pos2, c: f32, color: Color32) {
    // Shaft, then a stepped head.
    p.rect_filled(
        Rect::from_min_size(
            centre + Vec2::new(-3.5 * c, -0.5 * c),
            Vec2::new(4.0 * c, c),
        ),
        0.0,
        color,
    );
    for (i, h) in [5.0_f32, 3.0, 1.0].into_iter().enumerate() {
        p.rect_filled(
            Rect::from_min_size(
                centre + Vec2::new(0.5 * c + i as f32 * c, -h * 0.5 * c),
                Vec2::new(c, h * c),
            ),
            0.0,
            color,
        );
    }
}

/// A four-point sparkle, centred.
pub fn sparkle(p: &egui::Painter, centre: Pos2, c: f32, color: Color32) {
    p.rect_filled(Rect::from_center_size(centre, Vec2::splat(c)), 0.0, color);
    for d in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
        p.rect_filled(
            Rect::from_center_size(centre + d * c, Vec2::splat(c)),
            0.0,
            color,
        );
    }
}

/// A checkmark, about 5x4 cells, centred.
pub fn check(p: &egui::Painter, centre: Pos2, c: f32, color: Color32) {
    for (x, y) in [(0, 2), (1, 3), (2, 2), (3, 1), (4, 0)] {
        p.rect_filled(
            Rect::from_center_size(
                centre + Vec2::new((x as f32 - 2.0) * c, (y as f32 - 1.5) * c),
                Vec2::splat(c),
            ),
            0.0,
            color,
        );
    }
}

/// Shrinks `size` until `text` (monospace) fits in `width`, never below `min`.
pub fn fit(text: &str, size: f32, width: f32, min: f32) -> f32 {
    let n = text.chars().count().max(1) as f32;
    size.min(width / (n * 0.6)).max(min)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn painter() -> (egui::Context, egui::LayerId) {
        (egui::Context::default(), egui::LayerId::background())
    }

    fn draw(f: impl Fn(&egui::Painter)) -> usize {
        let (ctx, layer) = painter();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            ..Default::default()
        };
        ctx.run(input, |ctx| f(&ctx.layer_painter(layer)))
            .shapes
            .len()
    }

    #[test]
    fn the_cell_scales_with_the_screen_and_never_vanishes() {
        for h in [300.0, 480.0, 720.0, 1080.0, 2160.0] {
            let s = Rect::from_min_size(Pos2::ZERO, Vec2::new(h * 16.0 / 9.0, h));
            assert!((0.6..=1.5).contains(&scale(s)));
            assert!(cell(scale(s)) >= 2.0);
        }
        assert_eq!(cell(1.0), 3.0);
    }

    #[test]
    fn panels_chip_their_corners_and_survive_tiny_sizes() {
        let big = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(200.0, 80.0));
        let n = draw(|p| panel(p, big, [255, 0, 0], Color32::BLACK, 3.0, 1.0));
        assert!(n >= 8, "body + four edges + line: {n}");
        let tiny = Rect::from_min_size(Pos2::ZERO, Vec2::new(5.0, 5.0));
        assert!(draw(|p| panel(p, tiny, [255, 0, 0], Color32::BLACK, 3.0, 1.0)) > 0);
    }

    #[test]
    fn every_glyph_draws_something() {
        let at = Pos2::new(100.0, 100.0);
        let col = Color32::WHITE;
        assert!(draw(|p| plus(p, at, 3.0, col)) == 2);
        assert!(draw(|p| arrow(p, at, 3.0, col)) == 4);
        assert!(draw(|p| sparkle(p, at, 3.0, col)) == 5);
        assert!(draw(|p| check(p, at, 3.0, col)) == 5);
        assert!(draw(|p| brackets(p, Rect::from_min_size(at, Vec2::splat(40.0)), 3.0, col)) == 8);
        assert!(draw(|p| hatch(p, Rect::from_min_size(at, Vec2::new(40.0, 20.0)), 3.0, col)) > 0);
    }

    #[test]
    fn text_fits_or_hits_the_floor() {
        assert_eq!(fit("DASH", 20.0, 200.0, 8.0), 20.0);
        let long = fit("MELTING CLOCKWORKS", 20.0, 100.0, 8.0);
        assert!((8.0..20.0).contains(&long));
        assert_eq!(fit("MELTING CLOCKWORKS", 20.0, 10.0, 8.0), 8.0);
    }
}
