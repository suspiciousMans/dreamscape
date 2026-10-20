//! Foil pack wrappers, painted with egui (no image files) in the game's own
//! low-res look: everything sits on one chunky pixel grid, shading is
//! dithered instead of smooth, corners are chipped, text splits into the
//! same chromatic ghosts as the HUD, and a CRT scanline runs over it all.
//! The theme's own texture is the body, with a medallion and centre emblem.
//! See plans/PLAN-pick-a-pack.md.
//!
//! A wrapper never hints at rarity: rarity comes from how the run went, not
//! from the pack. The crimp carries one pip per card, nothing else.

use crate::dream::{DreamTheme, TexSpec, TEX_SIZE};
use crate::hud::rgba;
use crate::packs::Emblem;
use engine::ui::egui::{self, Align2, Color32, FontId, Pos2, Rect, Shape, Stroke, Vec2};
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

/// Wrapper pixel grid: about 40 chunky cells across, whatever the size.
#[derive(Clone, Copy)]
struct Grid {
    rect: Rect,
    c: f32,
    cols: i32,
    rows: i32,
}

impl Grid {
    fn new(rect: Rect) -> Self {
        let c = (rect.width() / 40.0).floor().max(1.0);
        let cols = (rect.width() / c).floor() as i32;
        let rows = (rect.height() / c).floor() as i32;
        let rect = Rect::from_min_size(rect.min, Vec2::new(cols as f32 * c, rows as f32 * c));
        Self {
            rect,
            c,
            cols,
            rows,
        }
    }

    /// Grid point (in cells, may be fractional) to screen.
    fn at(&self, x: f32, y: f32) -> Pos2 {
        self.rect.min + Vec2::new(x * self.c, y * self.c)
    }

    /// `w` by `h` cells starting at cell (`x`, `y`).
    fn cells(&self, x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect::from_min_size(
            self.at(x as f32, y as f32),
            Vec2::new(w as f32 * self.c, h as f32 * self.c),
        )
    }
}

/// The wrapper rect snapped to whole cells (what is really drawn).
#[cfg(test)]
fn snap_rect(rect: Rect) -> Rect {
    Grid::new(rect).rect
}

/// The crimp strips are three cells deep.
#[cfg(test)]
fn crimp_height(rect: Rect) -> f32 {
    Grid::new(rect).c * 3.0
}

/// The medallion: centre and radius inside `rect`.
pub fn medallion(rect: Rect) -> (Pos2, f32) {
    let g = Grid::new(rect);
    let r = (0.275 * g.cols as f32).round();
    let cy = (g.rows as f32 * 0.46).round();
    (g.at(g.cols as f32 / 2.0, cy), r * g.c)
}

/// How far from the centre an emblem may reach: inside the medallion.
pub fn emblem_reach(rect: Rect) -> f32 {
    medallion(rect).1 * 0.78
}

/// Which pack is being torn open, and where it sat on the pick screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Wrapper {
    pub theme: DreamTheme,
    pub emblem: Emblem,
    pub seed: u64,
    pub pips: usize,
    /// Where the pack was on the pick screen: the tear starts there.
    pub from: Rect,
}

/// The HUD's chromatic ghosts (cyan / magenta), for split text.
const GHOST_A: [u8; 3] = [90, 255, 255];
const GHOST_B: [u8; 3] = [255, 70, 200];
/// The card frame's chipped-corner dark.
const CHIP: [u8; 3] = [8, 4, 20];

/// Deterministic 0..1 noise for creases and jagged edges.
fn h01(k: u32, seed: u64) -> f32 {
    let mut x =
        (seed ^ (k as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 29;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 32;
    (x & 0xFFFF) as f32 / 65535.0
}

/// UV of a point on a wrapper, so a sliced-up wrapper keeps one texture.
fn uv_at(rect: Rect, at: Pos2) -> Pos2 {
    let fx = ((at.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
    let fy = ((at.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
    Pos2::new(0.05 + 0.9 * fx, 0.05 + 0.9 * fy)
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
    paint_wrapper(
        ctx, p, rect, None, theme, emblem, seed, pips, time, motion, tilt, art,
    );
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    Band,
    Tooth,
    Light,
    Dark,
}

/// A crimp strip as cell-aligned rects (so a torn-off strip can reuse it).
fn crimp_parts(g: &Grid, top: bool) -> Vec<(Rect, Part)> {
    let mut out = Vec::new();
    let (band_y, tooth_y) = if top {
        (0, 2)
    } else {
        (g.rows - 2, g.rows - 3)
    };
    out.push((g.cells(0, band_y, g.cols, 2), Part::Band));
    for i in 0..g.cols {
        let part = if i % 2 == 0 { Part::Light } else { Part::Dark };
        out.push((g.cells(i, band_y, 1, 2), part));
        if i % 2 == 0 {
            out.push((g.cells(i, tooth_y, 1, 1), Part::Tooth));
        }
    }
    out
}

fn part_color(part: Part, rgb: [u8; 3]) -> Color32 {
    match part {
        Part::Band | Part::Tooth => rgba([rgb[0] / 3, rgb[1] / 3, rgb[2] / 3], 0.95),
        Part::Light => Color32::from_white_alpha(30),
        Part::Dark => Color32::from_black_alpha(70),
    }
}

/// Text with the HUD's chromatic split: two ghosts either side of the core.
fn split_text(
    p: &egui::Painter,
    at: Pos2,
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
            Align2::CENTER_TOP,
            text,
            font.clone(),
            rgba(rgb, 0.55 * alpha),
        );
    }
    p.text(at, Align2::CENTER_TOP, text, font, core);
}

/// The wrapper, optionally only the part inside `visible` (a half-torn pack).
#[allow(clippy::too_many_arguments)]
fn paint_wrapper(
    ctx: &egui::Context,
    p: &egui::Painter,
    rect: Rect,
    visible: Option<Rect>,
    theme: DreamTheme,
    emblem: Emblem,
    seed: u64,
    pips: usize,
    time: f32,
    motion: f32,
    tilt: Vec2,
    art: &mut PackArt,
) {
    let g = Grid::new(rect);
    let rect = g.rect;
    let c = g.c;
    let motion = motion.clamp(0.0, 1.0);
    let tilt = Vec2::new(tilt.x.clamp(-1.0, 1.0), tilt.y.clamp(-1.0, 1.0)) * motion;
    let rgb = accent(theme);
    let bright = rgb.map(|v| v.saturating_add(70));
    let clip = p.with_clip_rect(visible.map_or(rect, |v| rect.intersect(v)));

    // A hard one-cell shadow, then the foil body: the theme's own texture
    // (nearest-filtered), shifted a touch with tilt so it sits under the wrapper.
    if visible.is_none() {
        p.rect_filled(rect.translate(Vec2::splat(c)), 0.0, rgba([4, 2, 10], 0.8));
    }
    let tex = art.texture(ctx, theme, seed);
    let shift = tilt * 0.04;
    let uv = Rect::from_min_max(Pos2::new(0.05, 0.05) + shift, Pos2::new(0.95, 0.95) + shift);
    clip.image(tex, rect, uv, Color32::from_gray(205));

    // Dithered shading instead of a smooth gradient: the foil puffs up in the
    // middle, so the edges step down through solid, checker and sparse dots.
    for j in 0..g.rows {
        for i in 0..g.cols {
            let d = i.min(g.cols - 1 - i).min(j).min(g.rows - 1 - j);
            let a = match d {
                1 => 120,
                2 if (i + j) % 2 == 0 => 100,
                3 if i % 2 == 0 && j % 2 == 0 => 90,
                _ => 0,
            };
            if a > 0 {
                clip.rect_filled(g.cells(i, j, 1, 1), 0.0, Color32::from_black_alpha(a));
            }
        }
    }
    // The same trick for a highlight up in the top-left.
    for j in 5..16 {
        for i in 3..16 {
            let s = (i - 3) + (j - 5);
            let a = if s < 5 {
                38
            } else if s < 10 && s % 2 == 0 {
                30
            } else {
                0
            };
            if a > 0 {
                clip.rect_filled(g.cells(i, j, 1, 1), 0.0, Color32::from_white_alpha(a));
            }
        }
    }

    // Creases: short stair-stepped scratches, a dark cell with a light one under it.
    for k in 0..5u32 {
        let x0 = 4 + (h01(k, seed ^ 0xC1) * (g.cols - 14) as f32) as i32;
        let y0 = 9 + (h01(k + 40, seed) * (g.rows - 22) as f32) as i32;
        let len = 8 + (h01(k + 80, seed) * 12.0) as i32;
        let slope = (h01(k + 120, seed) - 0.5) * 0.9;
        for s in 0..len {
            let (x, y) = (x0 + s, y0 + (slope * s as f32).round() as i32);
            clip.rect_filled(g.cells(x, y, 1, 1), 0.0, Color32::from_black_alpha(64));
            clip.rect_filled(g.cells(x, y + 1, 1, 1), 0.0, Color32::from_white_alpha(32));
        }
    }

    // A printed frame between the crimps, and the brand above the medallion.
    clip.rect_stroke(
        g.cells(2, 5, g.cols - 4, g.rows - 10),
        0.0,
        Stroke::new((c * 0.3).max(1.0), rgba(rgb, 0.55)),
    );
    let ghost = (c * 0.5).max(1.0);
    split_text(
        &clip,
        g.at(g.cols as f32 / 2.0, 7.0),
        "DREAMSCAPE",
        (c * 4.0).max(7.0),
        rgba(bright, 1.0),
        ghost,
        1.0,
    );

    // Crimps: fine ridges and a castellated row of teeth.
    for top in [true, false] {
        for (r, part) in crimp_parts(&g, top) {
            clip.rect_filled(r, 0.0, part_color(part, rgb));
        }
    }
    // Where a thumb starts the rip: chipped notches in the sides, and a dashed tear line.
    for (x, w) in [(0, 2), (g.cols - 2, 2)] {
        clip.rect_filled(g.cells(x, 3, w, 1), 0.0, rgba(CHIP, 1.0));
        clip.rect_filled(
            g.cells(if x == 0 { 0 } else { g.cols - 1 }, 4, 1, 1),
            0.0,
            rgba(CHIP, 1.0),
        );
    }
    for i in (3..g.cols - 3).step_by(2) {
        clip.rect_filled(g.cells(i, 4, 1, 1), 0.0, Color32::from_white_alpha(46));
    }

    // Foil sweep: a slanted highlight, stepped a row at a time. Static when motion is 0.
    let sweep = if motion > 0.0 {
        (time * 0.25 * motion + tilt.x * 0.3).rem_euclid(1.4) - 0.2
    } else {
        0.35
    };
    for j in 3..g.rows - 3 {
        let xs = (sweep * g.cols as f32) as i32 + ((g.rows - j) as f32 * 0.35) as i32 - 6;
        clip.rect_filled(g.cells(xs, j, 6, 1), 0.0, Color32::from_white_alpha(20));
        clip.rect_filled(g.cells(xs + 2, j, 2, 1), 0.0, Color32::from_white_alpha(32));
    }

    // Medallion: a pixel disc with a thick rim (the emblem drifts a cell with tilt).
    let (mc, mr) = medallion(rect);
    let shift_px = Vec2::new((tilt.x * 0.8).round(), (tilt.y * 0.8).round()) * c;
    let cxg = g.cols as f32 / 2.0;
    let cyg = (mc.y - rect.top()) / c;
    let r_cells = mr / c;
    for j in 0..g.rows {
        let dy = j as f32 + 0.5 - cyg;
        let Some((lo, hi)) = disc_run(cxg, r_cells, dy) else {
            continue;
        };
        let rim = r_cells - 1.5;
        match disc_run(cxg, rim, dy) {
            Some((ilo, ihi)) => {
                clip.rect_filled(g.cells(ilo, j, ihi - ilo, 1), 0.0, rgba([10, 5, 22], 0.9));
                clip.rect_filled(g.cells(lo, j, ilo - lo, 1), 0.0, rgba(rgb, 1.0));
                clip.rect_filled(g.cells(ihi, j, hi - ihi, 1), 0.0, rgba(rgb, 1.0));
            }
            None => {
                clip.rect_filled(g.cells(lo, j, hi - lo, 1), 0.0, rgba(rgb, 1.0));
            }
        }
    }
    paint_emblem(
        &clip,
        mc + shift_px,
        emblem_reach(rect),
        emblem,
        rgb,
        time,
        tilt.x,
    );

    // The pack's name on a ribbon under the medallion.
    paint_ribbon(&clip, &g, theme, rgb, mc, mr);

    // One pip per card on the lower crimp.
    let n = pip_count(pips) as i32;
    for i in 0..n {
        let x = g.cols / 2 - (n * 3 - 1) / 2 + i * 3;
        clip.rect_filled(g.cells(x, g.rows - 2, 2, 1), 0.0, rgba(bright, 1.0));
    }

    // CRT scanlines over the lot.
    for j in (0..g.rows).step_by(2) {
        clip.rect_filled(
            Rect::from_min_size(
                g.at(0.0, j as f32 + 0.65),
                Vec2::new(rect.width(), c * 0.35),
            ),
            0.0,
            Color32::from_black_alpha(26),
        );
    }

    // The border, like a card's: chipped corners and a hard edge.
    let edge = rgba(rgb, 0.95);
    clip.rect_filled(g.cells(1, 0, g.cols - 2, 1), 0.0, edge);
    clip.rect_filled(g.cells(1, g.rows - 1, g.cols - 2, 1), 0.0, edge);
    clip.rect_filled(g.cells(0, 1, 1, g.rows - 2), 0.0, edge);
    clip.rect_filled(g.cells(g.cols - 1, 1, 1, g.rows - 2), 0.0, edge);
    for (x, y) in [
        (0, 0),
        (g.cols - 1, 0),
        (0, g.rows - 1),
        (g.cols - 1, g.rows - 1),
    ] {
        clip.rect_filled(g.cells(x, y, 1, 1), 0.0, rgba(CHIP, 1.0));
    }
}

/// The run of cells (`lo`, `hi` exclusive) a disc of `r` cells covers in a
/// row `dy` cells from its centre, centred on column `cx`.
fn disc_run(cx: f32, r: f32, dy: f32) -> Option<(i32, i32)> {
    let sq = r * r - dy * dy;
    if sq <= 0.0 {
        return None;
    }
    let half = sq.sqrt();
    let lo = (cx - half - 0.5).ceil() as i32;
    let hi = (cx + half - 0.5).floor() as i32 + 1;
    (hi > lo).then_some((lo, hi))
}

/// Where the ribbon goes: its lines, font size, top row and height in rows.
fn ribbon_layout(g: &Grid, title: &str, mc: Pos2, mr: f32) -> (Vec<String>, f32, i32, i32) {
    let c = g.c;
    let lines: Vec<String> = if title.len() > 10 && title.contains(' ') {
        let mid = title.len() / 2;
        let at = title
            .match_indices(' ')
            .map(|(i, _)| i)
            .min_by_key(|i| i.abs_diff(mid))
            .unwrap_or(mid);
        vec![title[..at].to_string(), title[at + 1..].to_string()]
    } else {
        vec![title.to_string()]
    };
    let widest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1) as f32;
    let fit = (g.cols - 10) as f32 * c / (widest * 0.6);
    let size = (c * 5.0).min(fit);
    let want = ((lines.len() as f32 * size * 1.15) / c).ceil() as i32 + 3;
    let top_row = ((mc.y + mr - g.rect.top()) / c).round() as i32 + 2;
    let free = (g.rows - 6) - top_row;
    (lines, size, top_row, want.min(free))
}

/// The theme's name, split over two lines if long, on a chipped ribbon.
fn paint_ribbon(p: &egui::Painter, g: &Grid, theme: DreamTheme, rgb: [u8; 3], mc: Pos2, mr: f32) {
    let c = g.c;
    let (lines, size, top_row, rows_h) =
        ribbon_layout(g, &crate::reveal_ui::theme_title(theme), mc, mr);
    let line_h = size * 1.15;
    let x = 4;
    let w = g.cols - 8;
    let band = g.cells(x, top_row, w, rows_h);
    p.rect_filled(band, 0.0, rgba([10, 5, 22], 0.94));
    // Outline with chipped corners (the corner cells stay dark).
    let edge = rgba(rgb, 0.95);
    p.rect_filled(g.cells(x + 1, top_row, w - 2, 1), 0.0, edge);
    p.rect_filled(g.cells(x + 1, top_row + rows_h - 1, w - 2, 1), 0.0, edge);
    p.rect_filled(g.cells(x, top_row + 1, 1, rows_h - 2), 0.0, edge);
    p.rect_filled(g.cells(x + w - 1, top_row + 1, 1, rows_h - 2), 0.0, edge);
    let mid = g.at(0.0, top_row as f32 + rows_h as f32 * 0.5).y;
    let mut y = mid - line_h * lines.len() as f32 * 0.5;
    let cx = band.center().x;
    for l in &lines {
        split_text(
            p,
            Pos2::new(cx, y),
            l,
            size,
            rgba(rgb.map(|v| v.saturating_add(70)), 1.0),
            (c * 0.4).max(1.0),
            1.0,
        );
        y += line_h;
    }
}

/// Seconds the whole tear takes (pull, rip, light, drop).
pub const TEAR_TIME: f32 = 1.5;
/// When (as a fraction of the tear) the foil lets go: the sound plays here.
pub const TEAR_RIP_AT: f32 = 0.34;

fn smooth(x: f32) -> f32 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Where the pack sits while it is torn open: big, centred.
pub fn tear_rect(screen: Rect) -> Rect {
    let h = (screen.height() * 0.62).min(PACK_SIZE.y * 1.5);
    let w = h * PACK_SIZE.x / PACK_SIZE.y;
    Rect::from_center_size(
        Pos2::new(screen.center().x, screen.center().y - 10.0),
        Vec2::new(w, h),
    )
}

/// The ragged line the strip tears along, one point per cell across `rect`,
/// stepping up and down by half a cell.
fn tear_edge(rect: Rect, seed: u64) -> Vec<Pos2> {
    let g = Grid::new(rect);
    (0..=g.cols)
        .map(|i| {
            let step = (h01(i as u32, seed ^ 0x7EA2) * 3.0).floor() - 1.0;
            Pos2::new(
                g.rect.left() + i as f32 * g.c,
                g.rect.top() + g.c * (4.0 + step * 0.5),
            )
        })
        .collect()
}

/// Where the straight clip under the ragged lip sits.
fn lip_bottom(rect: Rect) -> f32 {
    let g = Grid::new(rect);
    g.rect.top() + 5.0 * g.c
}

/// A textured band between two edges, `alpha` 0..1 (the foil's own texture).
fn band_mesh(
    tex: egui::TextureId,
    rect: Rect,
    top: &[Pos2],
    bottom: &[Pos2],
    xf: &dyn Fn(Pos2) -> Pos2,
    alpha: f32,
) -> Shape {
    let tint = Color32::from_gray((205.0 * alpha) as u8);
    let tint =
        Color32::from_rgba_premultiplied(tint.r(), tint.g(), tint.b(), (255.0 * alpha) as u8);
    let mut m = egui::Mesh::with_texture(tex);
    for (a, b) in top.iter().zip(bottom) {
        m.vertices.push(egui::epaint::Vertex {
            pos: xf(*a),
            uv: uv_at(rect, *a),
            color: tint,
        });
        m.vertices.push(egui::epaint::Vertex {
            pos: xf(*b),
            uv: uv_at(rect, *b),
            color: tint,
        });
    }
    for i in 0..top.len().saturating_sub(1) as u32 {
        let k = i * 2;
        m.add_triangle(k, k + 1, k + 2);
        m.add_triangle(k + 1, k + 3, k + 2);
    }
    Shape::mesh(m)
}

/// The pack being torn open: it slides to the middle, shakes, the top
/// crimp rips off along a ragged line and flies away, light spills out, and
/// the body drops. `t` runs 0..1 over `TEAR_TIME`. `flashes` = false keeps
/// the white flash out for people who asked for less of it.
#[allow(clippy::too_many_arguments)]
pub fn draw_tear(
    ctx: &egui::Context,
    p: &egui::Painter,
    screen: Rect,
    w: &Wrapper,
    t: f32,
    time: f32,
    flashes: bool,
    art: &mut PackArt,
) {
    let t = t.clamp(0.0, 1.0);
    let rgb = accent(w.theme);
    let bright = rgb.map(|v| v.saturating_add(80));
    // 0.00-0.20 slide to the middle, 0.20-0.34 pull (shake), 0.34 rip,
    // 0.34-0.62 strip flies, 0.50-1.00 light + body drops.
    let slide = smooth(t / 0.2);
    let mut rect = Rect::from_min_max(
        w.from.min.lerp(tear_rect(screen).min, slide),
        w.from.max.lerp(tear_rect(screen).max, slide),
    );
    let pull = smooth((t - 0.2) / 0.14);
    let shake = pull * (1.0 - smooth((t - 0.34) / 0.05));
    let g = Grid::new(rect);
    // The shake moves in whole cells, like everything else here.
    let kick = |a: f32| (shake * 1.4 * a.sin()).round() * g.c;
    rect = g
        .rect
        .translate(Vec2::new(kick(t * 97.0), kick(t * 83.0 + 1.0)));
    let drop = smooth((t - 0.62) / 0.38);
    let body_rect = rect.translate(Vec2::new(
        0.0,
        (drop * rect.height() * 0.45 / g.c).round() * g.c,
    ));
    let fade = ((t - 0.66) / 0.3).clamp(0.0, 1.0);

    let n = g.cols as usize;
    let edge = tear_edge(rect, w.seed);
    let top_line: Vec<Pos2> = edge.iter().map(|q| Pos2::new(q.x, rect.top())).collect();
    let tex = art.texture(ctx, w.theme, w.seed);
    let bg = rgba([6, 3, 16], 1.0);

    // Light spilling out of the opening, behind everything else: stacked
    // posterized beams, hard-edged, narrowing as they rise.
    let spill = smooth((t - 0.34) / 0.2) * (1.0 - 0.35 * smooth((t - 0.8) / 0.2));
    if spill > 0.0 {
        let ty = rect.top() + 4.0 * g.c;
        let layers = 9;
        for k in 0..layers {
            let f = k as f32 / (layers - 1) as f32;
            let half = rect.width() * (0.62 - 0.42 * f) * (0.5 + 0.5 * spill);
            let ht = rect.height() * (0.35 + 0.65 * (1.0 - f)) * spill;
            let ht = (ht / g.c).round() * g.c;
            let half = (half / g.c).round() * g.c;
            p.rect_filled(
                Rect::from_min_max(
                    Pos2::new(rect.center().x - half, ty - ht),
                    Pos2::new(rect.center().x + half, ty),
                ),
                0.0,
                rgba(bright, 0.07 * spill),
            );
        }
    }

    // Before the rip the wrapper is whole; after it, a body and a loose strip.
    if t < TEAR_RIP_AT {
        paint_wrapper(
            ctx,
            p,
            rect,
            None,
            w.theme,
            w.emblem,
            w.seed,
            w.pips,
            time,
            1.0,
            Vec2::ZERO,
            art,
        );
    } else {
        // The body: the wrapper below the tear line, plus the ragged lip.
        let off = Vec2::new(0.0, body_rect.top() - rect.top());
        let clip_top = lip_bottom(rect) + off.y;
        if fade < 1.0 {
            let visible = Rect::from_min_max(
                Pos2::new(body_rect.left() - 20.0, clip_top),
                Pos2::new(body_rect.right() + 20.0, body_rect.bottom() + 20.0),
            );
            paint_wrapper(
                ctx,
                p,
                body_rect,
                Some(visible),
                w.theme,
                w.emblem,
                w.seed,
                w.pips,
                time,
                1.0,
                Vec2::ZERO,
                art,
            );
            // Ragged lip: the foil between the tear line and the straight clip.
            let lip_top: Vec<Pos2> = edge.iter().map(|q| *q + off).collect();
            let lip_bot: Vec<Pos2> = edge
                .iter()
                .map(|q| Pos2::new(q.x, clip_top + 1.0))
                .collect();
            p.add(band_mesh(tex, body_rect, &lip_top, &lip_bot, &|q| q, 1.0));
            p.add(Shape::line(
                lip_top,
                Stroke::new(g.c.max(1.0) * 0.4, Color32::from_white_alpha(170)),
            ));
            if fade > 0.0 {
                p.rect_filled(body_rect.expand(g.c * 4.0), 0.0, rgba([6, 3, 16], fade));
            }
        }

        // The strip: the top crimp, hinged at the right end, then thrown off.
        let rip = 0.18 + smooth((t - TEAR_RIP_AT) / 0.28) * 1.2;
        let fly = smooth((t - TEAR_RIP_AT) / 0.3);
        let alpha = 1.0 - smooth((t - 0.5) / 0.14);
        if alpha > 0.0 {
            let pivot = Pos2::new(rect.right(), rect.top() + g.c * 2.0);
            let throw = Vec2::new(
                fly * rect.width() * 0.5,
                -fly * rect.height() * 0.5 + fly * fly * rect.height() * 0.12,
            );
            let (sa, ca) = (-rip).sin_cos();
            let xf = move |q: Pos2| {
                let d = q - pivot;
                pivot + Vec2::new(d.x * ca - d.y * sa, d.x * sa + d.y * ca) + throw
            };
            p.add(band_mesh(tex, rect, &top_line, &edge, &xf, alpha));
            let quad = |r: Rect| {
                vec![
                    xf(r.left_top()),
                    xf(r.right_top()),
                    xf(r.right_bottom()),
                    xf(r.left_bottom()),
                ]
            };
            let g_top = Grid::new(rect);
            for (r, part) in crimp_parts(&g_top, true) {
                let col = part_color(part, rgb);
                let col = Color32::from_rgba_premultiplied(
                    (col.r() as f32 * alpha) as u8,
                    (col.g() as f32 * alpha) as u8,
                    (col.b() as f32 * alpha) as u8,
                    (col.a() as f32 * alpha) as u8,
                );
                p.add(Shape::convex_polygon(quad(r), col, Stroke::NONE));
            }
            p.add(Shape::line(
                edge.iter().map(|q| xf(*q)).collect(),
                Stroke::new(
                    g.c.max(1.0) * 0.4,
                    Color32::from_white_alpha((180.0 * alpha) as u8),
                ),
            ));
            p.add(Shape::closed_line(
                vec![
                    xf(Pos2::new(rect.left(), rect.top())),
                    xf(Pos2::new(rect.right(), rect.top())),
                    xf(edge[n]),
                    xf(edge[0]),
                ],
                Stroke::new(g.c.max(1.0) * 0.5, rgba(rgb, 0.9 * alpha)),
            ));
        }
    }

    // Sparks: square cells flying out of the opening once it lets go.
    let burst = (t - TEAR_RIP_AT) / (1.0 - TEAR_RIP_AT);
    if burst > 0.0 {
        let origin = Pos2::new(rect.center().x, rect.top() + 4.0 * g.c);
        for k in 0..28u32 {
            let ang = -std::f32::consts::PI * (0.08 + 0.84 * h01(k, w.seed ^ 0x5A));
            let spd = 0.4 + 0.9 * h01(k + 50, w.seed);
            let d = (burst * 1.4).min(1.0);
            let at = origin
                + Vec2::new(ang.cos(), ang.sin()) * rect.height() * 0.7 * spd * d
                + Vec2::new(0.0, d * d * rect.height() * 0.2);
            let a = (1.0 - burst).clamp(0.0, 1.0);
            let cell = g.c * (1.0 + (h01(k + 90, w.seed) * 1.5).floor());
            let snapped = Pos2::new((at.x / g.c).round() * g.c, (at.y / g.c).round() * g.c);
            p.rect_filled(
                Rect::from_center_size(snapped, Vec2::splat(cell)),
                0.0,
                rgba(bright, a * 0.9),
            );
        }
    }
    if flashes {
        let f =
            ((t - TEAR_RIP_AT) / 0.1).clamp(0.0, 1.0) * (1.0 - ((t - 0.44) / 0.2).clamp(0.0, 1.0));
        if f > 0.0 {
            p.rect_filled(screen, 0.0, rgba([255, 244, 220], f * 0.25));
        }
    }
    let _ = bg;
}

/// Fill a small cell sprite centred on `c`. `f(x, y)` gives 0 = nothing,
/// 1 = body, 2 = bright, 3 = dark, for cells -half..=half.
fn paint_cells(
    p: &egui::Painter,
    c: Pos2,
    px: f32,
    half: i32,
    tones: [Color32; 3],
    f: &dyn Fn(i32, i32) -> u8,
) {
    for y in -half..=half {
        for x in -half..=half {
            let tone = f(x, y);
            if tone == 0 {
                continue;
            }
            p.rect_filled(
                Rect::from_center_size(c + Vec2::new(x as f32, y as f32) * px, Vec2::splat(px)),
                0.0,
                tones[tone as usize - 1],
            );
        }
    }
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
    let tones = [rgba(rgb, 1.0), rgba(bright, 1.0), dark];
    let px = (reach * 2.0 / 17.0).floor().max(1.0);
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
            // An arched door: dark inside, a bright handle and doorstep.
            let inside = |x: i32, y: i32, r: f32, top: i32| {
                (x.abs() as f32 <= r && y >= top)
                    || ((x * x + (y - top) * (y - top)) as f32 <= (r + 0.5) * (r + 0.5) && y < top)
            };
            paint_cells(p, c, px, 8, tones, &|x, y| {
                if y > 7 {
                    return 0;
                }
                if y == 7 && x.abs() <= 5 {
                    return 2;
                }
                if x == 2 && y == 2 {
                    return 2;
                }
                if inside(x, y, 2.0, -3) {
                    return 3;
                }
                if inside(x, y, 4.0, -3) {
                    return 1;
                }
                0
            });
        }
        Emblem::Shard => {
            // A faceted diamond: the upper-right facet catches the light, the edge glints.
            let inside = |x: i32, y: i32| x.abs() as f32 / 4.5 + y.abs() as f32 / 7.5 <= 1.0;
            paint_cells(p, c, px, 8, tones, &|x, y| {
                if !inside(x, y) {
                    return 0;
                }
                let edge = !inside(x + 1, y)
                    || !inside(x - 1, y)
                    || !inside(x, y + 1)
                    || !inside(x, y - 1);
                if edge || (x > 0 && y <= 0) {
                    2
                } else {
                    1
                }
            });
        }
        Emblem::Crescent => {
            // A bright disc with a bite taken out of it, and two stars.
            let disc = |x: i32, y: i32, cx: i32, cy: i32, r: i32| {
                (x - cx) * (x - cx) + (y - cy) * (y - cy) <= r * r
            };
            paint_cells(p, c, px, 8, tones, &|x, y| {
                if (x, y) == (5, -5) || (x, y) == (7, 2) {
                    return 2;
                }
                if disc(x, y, 0, 0, 7) && !disc(x, y, 3, -1, 6) {
                    2
                } else {
                    0
                }
            });
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

    #[test]
    fn the_tear_draws_at_every_moment_for_every_theme() {
        let ctx = egui::Context::default();
        let mut art = PackArt::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0));
        for &theme in ALL_THEMES.iter() {
            let w = Wrapper {
                theme,
                emblem: Emblem::Eye,
                seed: 11,
                pips: 3,
                from: Rect::from_min_size(Pos2::new(300.0, 200.0), PACK_SIZE),
            };
            for step in 0..=20 {
                let t = step as f32 / 20.0;
                let input = egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                };
                let out = ctx.run(input, |ctx| {
                    let p = ctx.layer_painter(egui::LayerId::background());
                    draw_tear(ctx, &p, screen, &w, t, 1.0, true, &mut art);
                });
                assert!(!out.shapes.is_empty(), "{theme:?} t={t}");
            }
        }
    }

    #[test]
    fn the_torn_halves_share_one_edge_and_the_pack_ends_clear() {
        let rect = tear_rect(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0)));
        let g = Grid::new(rect);
        let e = tear_edge(rect, 5);
        assert_eq!(e.len(), g.cols as usize + 1, "one point per cell");
        assert_eq!(e[0].x, g.rect.left());
        assert_eq!(e[g.cols as usize].x, g.rect.right());
        // Jagged in half-cell steps, always below the crimp teeth and above the clip line.
        for q in &e {
            assert!(q.y >= g.rect.top() + 3.5 * g.c - 0.01, "{q:?}");
            assert!(q.y <= g.rect.top() + 4.5 * g.c + 0.01, "{q:?}");
            assert!(q.y < lip_bottom(rect));
        }
        assert!(e.windows(2).any(|w| w[0].y != w[1].y), "not flat");
        assert_eq!(TEAR_TIME, 1.5);
        assert!(
            tear_rect(Rect::from_min_size(Pos2::ZERO, Vec2::new(640.0, 360.0))).height() < 360.0
        );
    }

    #[test]
    fn the_wrapper_snaps_to_whole_cells_and_stays_snapped() {
        for w in [90.0_f32, 110.0, 130.0, 220.0, 329.0, 480.0] {
            let rect = Rect::from_min_size(Pos2::new(3.0, 4.0), Vec2::new(w, w * 1.45));
            let s = snap_rect(rect);
            let g = Grid::new(rect);
            assert_eq!(s, g.rect);
            assert_eq!(snap_rect(s), s, "idempotent at {w}");
            assert!(s.width() <= rect.width() && s.height() <= rect.height());
            assert!(rect.width() - s.width() < g.c + 0.01);
            assert!(g.cols >= 40, "{w}: {} cols", g.cols);
        }
    }

    #[test]
    fn ribbons_fit_every_theme_name_at_every_size() {
        for &t in ALL_THEMES.iter() {
            let title = crate::reveal_ui::theme_title(t);
            for size in [
                PACK_SIZE,
                PACK_SIZE * 0.5,
                PACK_SIZE * 1.5,
                Vec2::new(90.0, 130.0),
            ] {
                let g = Grid::new(Rect::from_min_size(Pos2::ZERO, size));
                let (mc, mr) = medallion(g.rect);
                let (lines, font, top, rows) = ribbon_layout(&g, &title, mc, mr);
                let widest = lines.iter().map(|l| l.chars().count()).max().unwrap() as f32;
                assert!(
                    widest * font * 0.6 <= (g.cols - 8) as f32 * g.c,
                    "{title} {size:?} text too wide"
                );
                assert!(rows >= 4, "{title} {size:?}: {rows} rows");
                assert!(
                    top + rows <= g.rows - 5,
                    "{title} {size:?}: runs into the crimp"
                );
                assert!(
                    lines.len() as f32 * font * 1.15 <= rows as f32 * g.c,
                    "{title} {size:?}: text taller than ribbon"
                );
            }
        }
    }
}
