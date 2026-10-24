//! Shared pieces of the game's pixel look for screens: chipped panels,
//! cell sprites, chromatic-split text, scanlines, pixel glyphs.
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
// === perf === uipixels: the seven body/edge bars as one mesh (same draw order)
pub fn panel(p: &egui::Painter, r: Rect, edge: [u8; 3], fill: Color32, c: f32, alpha: f32) {
    let (w, h) = (r.width(), r.height());
    if w < 4.0 * c || h < 4.0 * c {
        p.rect_filled(r, 0.0, fill);
        p.rect_stroke(r, 0.0, Stroke::new(c.min(2.0), rgba(edge, alpha)));
        return;
    }
    let e = rgba(edge, alpha);
    let mut q = Quads::new();
    // The body, minus the four corner cells.
    q.rect(cells(r, c, 0.0, w - 2.0 * c, h), fill);
    q.rect(cells(r, 0.0, c, c, h - 2.0 * c), fill);
    q.rect(cells(r, w - c, c, c, h - 2.0 * c), fill);
    q.rect(cells(r, c, 0.0, w - 2.0 * c, c), e);
    q.rect(cells(r, c, h - c, w - 2.0 * c, c), e);
    q.rect(cells(r, 0.0, c, c, h - 2.0 * c), e);
    q.rect(cells(r, w - c, c, c, h - 2.0 * c), e);
    q.paint(p);
    // A faint inner line, as on the cards.
    p.rect_stroke(
        r.shrink(c * 2.0),
        0.0,
        Stroke::new(1.0_f32, rgba(edge, alpha * 0.3)),
    );
}

/// The original panel, a rect per bar: the reference for tests.
#[cfg(test)]
fn panel_rects(p: &egui::Painter, r: Rect, edge: [u8; 3], fill: Color32, c: f32, alpha: f32) {
    let (w, h) = (r.width(), r.height());
    p.rect_filled(cells(r, c, 0.0, w - 2.0 * c, h), 0.0, fill);
    p.rect_filled(cells(r, 0.0, c, c, h - 2.0 * c), 0.0, fill);
    p.rect_filled(cells(r, w - c, c, c, h - 2.0 * c), 0.0, fill);
    let e = rgba(edge, alpha);
    p.rect_filled(cells(r, c, 0.0, w - 2.0 * c, c), 0.0, e);
    p.rect_filled(cells(r, c, h - c, w - 2.0 * c, c), 0.0, e);
    p.rect_filled(cells(r, 0.0, c, c, h - 2.0 * c), 0.0, e);
    p.rect_filled(cells(r, w - c, c, c, h - 2.0 * c), 0.0, e);
}
// === end perf ===

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
// === perf === uipixels: run-merged cells in one mesh
pub fn icon(
    p: &egui::Painter,
    centre: Pos2,
    rows: &[&str; icons::SIZE],
    px: f32,
    base: [u8; 3],
    accent: [u8; 3],
    alpha: f32,
) {
    let mut q = Quads::new();
    for (x, y, w, h, acc) in merge_cells(&icons::cells(rows)) {
        q.rect(
            Rect::from_min_size(
                centre + Vec2::new(x as f32 - 0.5, y as f32 - 0.5) * px,
                Vec2::new(w as f32, h as f32) * px,
            ),
            rgba(if acc { accent } else { base }, alpha),
        );
    }
    q.paint(p);
}

/// The original icon painter, one rect per cell: the reference for tests.
#[cfg(test)]
fn icon_rects(
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
// === end perf ===

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

// === perf === uipixels: many solid quads as ONE mesh shape
/// Solid quads batched into a single egui mesh: one shape, 4 vertices and 2
/// triangles per quad (a `rect_filled` costs a shape, 8+ vertices and ~10
/// triangles for its feathered edge). Edges are hard, so on the cell grid
/// they are as crisp as an aligned rect's.
#[derive(Default)]
pub struct Quads {
    mesh: egui::epaint::Mesh,
}

impl Quads {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn rect(&mut self, r: Rect, color: Color32) {
        if color != Color32::TRANSPARENT && r.is_positive() {
            self.mesh.add_colored_rect(r, color);
        }
    }
    pub fn paint(self, p: &egui::Painter) {
        if !self.mesh.is_empty() {
            p.add(egui::Shape::mesh(self.mesh));
        }
    }
}

/// Greedy merge of same-kind grid cells into rectangles `(x, y, w, h, kind)`:
/// horizontal runs first, then equal runs stacked in consecutive rows.
pub fn merge_cells<K: Copy + PartialEq>(cells: &[(i32, i32, K)]) -> Vec<(i32, i32, i32, i32, K)> {
    use std::collections::{BTreeMap, HashSet};
    let grid: BTreeMap<(i32, i32), K> = cells.iter().map(|&(x, y, k)| ((y, x), k)).collect();
    let mut used: HashSet<(i32, i32)> = HashSet::new();
    let mut out = Vec::new();
    for (&(y, x), &k) in &grid {
        if used.contains(&(y, x)) {
            continue;
        }
        let same = |used: &HashSet<(i32, i32)>, y: i32, x: i32| {
            grid.get(&(y, x)) == Some(&k) && !used.contains(&(y, x))
        };
        let mut w = 1;
        while same(&used, y, x + w) {
            w += 1;
        }
        let mut h = 1;
        while (0..w).all(|dx| same(&used, y + h, x + dx)) {
            h += 1;
        }
        for dy in 0..h {
            for dx in 0..w {
                used.insert((y + dy, x + dx));
            }
        }
        out.push((x, y, w, h, k));
    }
    out
}

/// Faint CRT scanlines across `r`: a dark line a cell high every two cells.
/// One mesh of thin quads (they were a rect each).
pub fn scanlines(p: &egui::Painter, r: Rect, c: f32, alpha: u8) {
    let mut q = Quads::new();
    let mut y = r.top() + c * 0.65;
    while y < r.bottom() {
        q.rect(
            Rect::from_min_size(Pos2::new(r.left(), y), Vec2::new(r.width(), c * 0.35)),
            Color32::from_black_alpha(alpha),
        );
        y += c * 2.0;
    }
    q.paint(p);
}
// === end perf ===

/// Diagonal hatching in cells, for locked or unavailable things.
///
/// One textured quad: the stair-stepped diagonals repeat every 4 cells
/// across and every cell up-and-right, so a 4x4 texel pattern (one texel a
/// cell, NEAREST, repeating) tiles them exactly. Same pixels as
/// `hatch_rects`, 2 triangles instead of one square per cell.
// === perf === uipixels
pub fn hatch(p: &egui::Painter, r: Rect, c: f32, color: Color32) {
    if r.width() <= 0.0 || r.height() <= 0.0 || c <= 0.0 {
        return;
    }
    let tex = hatch_texture(p.ctx());
    // Cell grid origin: x0 = left - height at the rect's bottom edge.
    let k = 4.0 * c;
    let uv = Rect::from_min_max(
        Pos2::new(r.height() / k, -r.height() / k),
        Pos2::new((r.right() - (r.left() - r.height())) / k, 0.0),
    );
    p.image(tex, r, uv, color);
}

/// Texel (x, y) of the 4x4 hatch tile is lit on the anti-diagonal.
pub(crate) fn hatch_texel(x: usize, y: usize) -> bool {
    x == 3 - y
}

fn hatch_texture(ctx: &egui::Context) -> egui::TextureId {
    let id = egui::Id::new("pixel_ui_hatch_tile");
    if let Some(h) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return h.id();
    }
    let pixels: Vec<Color32> = (0..16)
        .map(|i| if hatch_texel(i % 4, i / 4) { Color32::WHITE } else { Color32::TRANSPARENT })
        .collect();
    let image = egui::ColorImage { size: [4, 4], pixels };
    let options = egui::TextureOptions {
        magnification: egui::TextureFilter::Nearest,
        minification: egui::TextureFilter::Nearest,
        wrap_mode: egui::TextureWrapMode::Repeat,
        mipmap_mode: None,
    };
    let h = ctx.load_texture("hatch-tile", image, options);
    let tid = h.id();
    ctx.data_mut(|d| d.insert_temp(id, h));
    tid
}
// === end perf ===

/// The original painter: one rect per hatch cell. Kept as the reference the
/// textured `hatch` is tested against.
#[cfg(test)]
fn hatch_rects(p: &egui::Painter, r: Rect, c: f32, color: Color32) {
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

/// A right-pointing selection marker, 3 cells wide and 5 tall, centred.
pub fn pointer(p: &egui::Painter, centre: Pos2, c: f32, color: Color32) {
    for (i, h) in [5.0_f32, 3.0, 1.0].into_iter().enumerate() {
        p.rect_filled(
            Rect::from_min_size(
                centre + Vec2::new((i as f32 - 1.5) * c, -h * 0.5 * c),
                Vec2::new(c, h * c),
            ),
            0.0,
            color,
        );
    }
}

/// A highlighted row: a tinted bar with a marker at its left end.
pub fn selected_row(p: &egui::Painter, r: Rect, rgb: [u8; 3], c: f32) {
    p.rect_filled(r, 0.0, rgba(rgb, 0.14));
    p.rect_filled(
        Rect::from_min_size(r.min, Vec2::new(c, r.height())),
        0.0,
        rgba(rgb, 0.9),
    );
    pointer(
        p,
        Pos2::new(r.left() + c * 3.0, r.center().y),
        c,
        rgba(rgb, 1.0),
    );
}

/// A segmented pixel bar, `frac` 0..1 filled, one cell high per segment.
pub fn bar(p: &egui::Painter, r: Rect, frac: f32, rgb: [u8; 3], alpha: f32) {
    let seg = (r.height() * 1.2).max(3.0);
    let n = ((r.width() / seg).floor() as i32).max(1);
    let w = r.width() / n as f32;
    let lit = (frac.clamp(0.0, 1.0) * n as f32).round() as i32;
    for i in 0..n {
        let cell = Rect::from_min_size(
            Pos2::new(r.left() + i as f32 * w, r.top()),
            Vec2::new((w - 1.0).max(1.0), r.height()),
        );
        if i < lit {
            p.rect_filled(cell, 0.0, rgba(rgb, 0.9 * alpha));
        } else {
            p.rect_filled(cell, 0.0, rgba(rgb, 0.12 * alpha));
        }
    }
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

/// A panel with a title tab on its top edge.
pub fn titled_panel(
    p: &egui::Painter,
    r: Rect,
    title: &str,
    edge: [u8; 3],
    fill: Color32,
    c: f32,
    size: f32,
) {
    panel(p, r, edge, fill, c, 0.9);
    let galley = p.layout_no_wrap(title.to_string(), FontId::monospace(size), Color32::WHITE);
    let tab = Rect::from_min_size(
        r.left_top() + Vec2::new(c * 3.0, -galley.size().y * 0.5 - c * 0.5),
        Vec2::new(galley.size().x + c * 3.0, galley.size().y + c),
    );
    p.rect_filled(tab, 0.0, rgba([8, 4, 20], 1.0));
    p.rect_stroke(tab, 0.0, Stroke::new(1.0_f32, rgba(edge, 0.9)));
    p.text(
        tab.center(),
        Align2::CENTER_CENTER,
        title,
        FontId::monospace(size),
        rgba(edge, 1.0),
    );
}

/// Shrinks `size` until `text` (monospace) fits in `width`, never below `min`.
pub fn fit(text: &str, size: f32, width: f32, min: f32) -> f32 {
    let n = text.chars().count().max(1) as f32;
    size.min(width / (n * 0.6)).max(min)
}

#[cfg(test)]
pub(crate) mod tests {
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
        assert!(n >= 2, "body+edge mesh and the faint line: {n}");
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

    // === perf === a tiny CPU rasteriser: shapes -> premultiplied RGBA at pixel centres
    pub(crate) fn raster(shapes: &[egui::epaint::ClippedShape], w: usize, h: usize) -> Vec<[u8; 4]> {
        let mut buf = vec![[0u8; 4]; w * h];
        fn over(dst: &mut [u8; 4], src: [u8; 4]) {
            let ia = 255 - src[3] as u32;
            for i in 0..4 {
                dst[i] = (src[i] as u32 + (dst[i] as u32 * ia + 127) / 255).min(255) as u8;
            }
        }
        fn go(buf: &mut [[u8; 4]], w: usize, h: usize, clip: Rect, s: &egui::Shape) {
            match s {
                egui::Shape::Vec(v) => v.iter().for_each(|s| go(buf, w, h, clip, s)),
                egui::Shape::Rect(r) => {
                    assert!(r.stroke.is_empty() && r.rounding == egui::Rounding::ZERO, "fill-only rects");
                    let c = r.fill.to_array();
                    let rr = r.rect.intersect(clip);
                    for y in 0..h {
                        for x in 0..w {
                            if rr.contains(Pos2::new(x as f32 + 0.5, y as f32 + 0.5)) {
                                over(&mut buf[y * w + x], c);
                            }
                        }
                    }
                }
                egui::Shape::Mesh(m) => {
                    // Quads of two triangles; sample the hatch tile for any managed texture.
                    let mut done = vec![false; w * h]; // a quad's shared diagonal counts once
                    for (ti, tri) in m.indices.chunks(3).enumerate() {
                        if ti % 2 == 0 {
                            done.iter_mut().for_each(|d| *d = false);
                        }
                        let v: Vec<_> = tri.iter().map(|&i| m.vertices[i as usize]).collect();
                        let (mut lo, mut hi) = (v[0].pos, v[0].pos);
                        for q in &v {
                            lo = lo.min(q.pos);
                            hi = hi.max(q.pos);
                        }
                        // Axis-aligned quads only: pixel centre inside the triangle's bounds
                        // and on the right side of the diagonal.
                        let area = |a: Pos2, b: Pos2, c: Pos2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
                        let sign = area(v[0].pos, v[1].pos, v[2].pos).signum();
                        for y in 0..h {
                            for x in 0..w {
                                let q = Pos2::new(x as f32 + 0.5, y as f32 + 0.5);
                                if !clip.contains(q) {
                                    continue;
                                }
                                let w0 = area(v[1].pos, v[2].pos, q) * sign;
                                let w1 = area(v[2].pos, v[0].pos, q) * sign;
                                let w2 = area(v[0].pos, v[1].pos, q) * sign;
                                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                                    continue;
                                }
                                let tot = w0 + w1 + w2;
                                let uvp = (v[0].uv.to_vec2() * w0 + v[1].uv.to_vec2() * w1 + v[2].uv.to_vec2() * w2) / tot;
                                let lit = if m.texture_id == egui::TextureId::default() {
                                    true
                                } else {
                                    let tx = (uvp.x * 4.0).floor().rem_euclid(4.0) as usize;
                                    let ty = (uvp.y * 4.0).floor().rem_euclid(4.0) as usize;
                                    hatch_texel(tx, ty)
                                };
                                if lit && !done[y * w + x] {
                                    done[y * w + x] = true;
                                    over(&mut buf[y * w + x], v[0].color.to_array());
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        for cs in shapes {
            go(&mut buf, w, h, cs.clip_rect, &cs.shape);
        }
        buf
    }

    fn shapes_of(f: impl Fn(&egui::Painter)) -> Vec<egui::epaint::ClippedShape> {
        let (ctx, layer) = painter();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(300.0, 200.0))),
            ..Default::default()
        };
        ctx.run(input, |ctx| f(&ctx.layer_painter(layer))).shapes
    }

    #[test]
    fn the_textured_hatch_covers_the_same_pixels_as_the_rects() {
        let color = Color32::from_white_alpha(40);
        for (r, c) in [
            (Rect::from_min_size(Pos2::new(10.0, 12.0), Vec2::new(120.0, 90.0)), 3.0),
            (Rect::from_min_size(Pos2::new(7.0, 5.0), Vec2::new(61.0, 37.0)), 3.0),
            (Rect::from_min_size(Pos2::new(21.0, 9.0), Vec2::new(200.0, 150.0)), 5.0),
            (Rect::from_min_size(Pos2::new(1.0, 1.0), Vec2::new(40.0, 20.0)), 2.0),
        ] {
            let old = shapes_of(|p| hatch_rects(p, r, c, color));
            let new = shapes_of(|p| hatch(p, r, c, color));
            assert!(new.len() < old.len(), "{} vs {}", new.len(), old.len());
            let (a, b) = (raster(&old, 300, 200), raster(&new, 300, 200));
            let bad: Vec<_> = (0..a.len()).filter(|&i| a[i] != b[i]).map(|i| (i % 300, i / 300, a[i], b[i])).collect();
            assert!(bad.is_empty(), "rect {r:?} cell {c}: {} differ, first {:?}", bad.len(), &bad[..bad.len().min(6)]);
        }
    }

    #[test]
    fn merged_icons_cover_the_same_pixels_as_one_rect_per_cell() {
        for (i, rows) in icons::ALL.iter().enumerate() {
            for (px, alpha) in [(2.0, 1.0), (3.0, 1.0), (4.0, 0.5), (12.0, 1.0)] {
                // Cell edges on whole pixels (no sampling ties).
                let f = (px * 0.5_f32).fract();
                let at = Pos2::new(60.0 + (i % 3) as f32 + f, 80.0 + f);
                let old = shapes_of(|p| icon_rects(p, at, rows, px, [200, 100, 50], [250, 240, 230], alpha));
                let new = shapes_of(|p| icon(p, at, rows, px, [200, 100, 50], [250, 240, 230], alpha));
                assert_eq!(new.len(), 1, "one mesh per icon");
                let (a, b) = (raster(&old, 300, 200), raster(&new, 300, 200));
                // Opaque icons must match exactly; translucent ones differ nowhere
                // either, because the cells never overlap.
                assert!(a == b, "icon {i} px {px} alpha {alpha}");
            }
        }
    }

    #[test]
    fn merging_cells_loses_none_and_overlaps_none() {
        let icon_cells = icons::cells(&icons::HEART);
        let merged = merge_cells(&icon_cells);
        let mut seen = std::collections::HashSet::new();
        for &(x, y, w, h, _) in &merged {
            for dy in 0..h {
                for dx in 0..w {
                    assert!(seen.insert((x + dx, y + dy)), "overlap");
                }
            }
        }
        let want: std::collections::HashSet<_> = icon_cells.iter().map(|&(x, y, _)| (x, y)).collect();
        assert_eq!(seen, want);
        assert!(merged.len() < icon_cells.len());
    }

    #[test]
    fn one_mesh_of_scanlines_matches_the_rect_per_line() {
        for (c, alpha) in [(3.0, 8u8), (2.0, 10), (4.0, 10), (5.0, 8)] {
            let r = Rect::from_min_size(Pos2::ZERO, Vec2::new(300.0, 200.0));
            let old = shapes_of(|p| {
                let mut y = r.top() + c * 0.65;
                while y < r.bottom() {
                    p.rect_filled(
                        Rect::from_min_size(Pos2::new(r.left(), y), Vec2::new(r.width(), c * 0.35)),
                        0.0,
                        Color32::from_black_alpha(alpha),
                    );
                    y += c * 2.0;
                }
            });
            let new = shapes_of(|p| scanlines(p, r, c, alpha));
            assert_eq!(new.len(), 1);
            assert!(old.len() > 10);
            assert_eq!(raster(&old, 300, 200), raster(&new, 300, 200), "cell {c}");
        }
    }

    #[test]
    fn the_panel_mesh_matches_the_rect_per_bar() {
        for (r, c, fill, alpha) in [
            (Rect::from_min_size(Pos2::new(10.0, 12.0), Vec2::new(120.0, 90.0)), 3.0, Color32::from_rgba_unmultiplied(14, 8, 28, 180), 0.6),
            (Rect::from_min_size(Pos2::new(20.0, 20.0), Vec2::new(200.0, 60.0)), 4.0, Color32::from_rgb(20, 10, 40), 1.0),
            (Rect::from_min_size(Pos2::new(5.0, 7.0), Vec2::new(41.0, 33.0)), 2.0, Color32::from_rgba_unmultiplied(40, 20, 80, 99), 0.9),
        ] {
            let old = shapes_of(|p| panel_rects(p, r, [200, 120, 255], fill, c, alpha));
            let new = shapes_of(|p| {
                panel(p, r, [200, 120, 255], fill, c, alpha);
            });
            // Compare the bars only (the faint inner stroke is the same shape in both).
            let bars = |s: &[egui::epaint::ClippedShape]| {
                raster(&s.iter().filter(|c| !matches!(c.shape, egui::Shape::Rect(ref r) if !r.stroke.is_empty())).cloned().collect::<Vec<_>>(), 300, 200)
            };
            assert_eq!(bars(&old), bars(&new), "panel {r:?}");
            assert!(new.len() <= 2);
        }
    }
    // === end perf ===

    #[test]
    fn text_fits_or_hits_the_floor() {
        assert_eq!(fit("DASH", 20.0, 200.0, 8.0), 20.0);
        let long = fit("MELTING CLOCKWORKS", 20.0, 100.0, 8.0);
        assert!((8.0..20.0).contains(&long));
        assert_eq!(fit("MELTING CLOCKWORKS", 20.0, 10.0, 8.0), 8.0);
    }
}
