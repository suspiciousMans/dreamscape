//! Foil pack wrappers, painted with egui (no image files): the theme's own
//! texture as the body, crimped edges, a medallion with a centre emblem.
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
fn uv_at(rect: Rect, shift: Vec2, at: Pos2) -> Pos2 {
    let fx = ((at.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
    let fy = ((at.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
    Pos2::new(0.05 + 0.9 * fx + shift.x, 0.05 + 0.9 * fy + shift.y)
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
    let motion = motion.clamp(0.0, 1.0);
    let tilt = Vec2::new(tilt.x.clamp(-1.0, 1.0), tilt.y.clamp(-1.0, 1.0)) * motion;
    let rgb = accent(theme);
    let clip = p.with_clip_rect(visible.map_or(rect, |v| rect.intersect(v)));
    let ch = crimp_height(rect);

    // Drop shadow, then the foil body: the theme's own texture, shifted a
    // touch with tilt so it seems to sit under the wrapper.
    if visible.is_none() {
        p.rect_filled(
            rect.translate(Vec2::new(5.0, 7.0)),
            6.0,
            Color32::from_black_alpha(110),
        );
    }
    let tex = art.texture(ctx, theme, seed);
    let shift = tilt * 0.04;
    let uv = Rect::from_min_max(Pos2::new(0.05, 0.05) + shift, Pos2::new(0.95, 0.95) + shift);
    clip.image(tex, rect, uv, Color32::from_gray(205));

    // Pillowed foil: air inside the bag lifts the middle and darkens the
    // edges, with a soft highlight toward the upper left.
    let mut pillow = egui::Mesh::default();
    let (cols, rows) = (6usize, 8usize);
    for j in 0..=rows {
        for i in 0..=cols {
            let fx = i as f32 / cols as f32;
            let fy = j as f32 / rows as f32;
            let edge = ((fx - 0.5).abs() * 2.0)
                .max((fy - 0.5).abs() * 2.0)
                .powf(2.2);
            let dark = (edge * 120.0) as u8;
            let lit = ((1.0 - ((fx - 0.3).powi(2) + (fy - 0.25).powi(2)).sqrt() * 2.2).max(0.0)
                * 46.0) as u8;
            let col = Color32::from_rgba_premultiplied(lit, lit, lit, lit.max(dark));
            let col = if dark > lit {
                Color32::from_black_alpha(dark - lit)
            } else {
                col
            };
            pillow.colored_vertex(
                Pos2::new(
                    rect.left() + fx * rect.width(),
                    rect.top() + fy * rect.height(),
                ),
                col,
            );
        }
    }
    for j in 0..rows {
        for i in 0..cols {
            let a = (j * (cols + 1) + i) as u32;
            let b = a + 1;
            let c = a + (cols + 1) as u32;
            let d = c + 1;
            pillow.add_triangle(a, b, c);
            pillow.add_triangle(b, d, c);
        }
    }
    clip.add(Shape::mesh(pillow));

    // Creases: pairs of a light and a dark hairline, where the foil buckled.
    for k in 0..9u32 {
        let y0 = rect.top() + rect.height() * (0.12 + 0.76 * h01(k, seed ^ 0xC1));
        let rise = rect.height() * (h01(k + 40, seed) - 0.5) * 0.35;
        let x0 = rect.left() + rect.width() * h01(k + 80, seed) * 0.35;
        let x1 = rect.right() - rect.width() * h01(k + 120, seed) * 0.35;
        let a = Pos2::new(x0, y0);
        let b = Pos2::new(x1, y0 + rise);
        let fade = 0.35 + 0.65 * h01(k + 160, seed);
        clip.line_segment(
            [a, b],
            Stroke::new(1.0_f32, Color32::from_black_alpha((70.0 * fade) as u8)),
        );
        clip.line_segment(
            [a + Vec2::new(0.0, 1.6), b + Vec2::new(0.0, 1.6)],
            Stroke::new(1.0_f32, Color32::from_white_alpha((46.0 * fade) as u8)),
        );
    }

    // A printed frame inside the crimps, and the brand above the medallion.
    let frame = Rect::from_min_max(
        rect.min + Vec2::new(rect.width() * 0.06, ch * 1.5),
        rect.max - Vec2::new(rect.width() * 0.06, ch * 1.5),
    );
    clip.rect_stroke(frame, 4.0, Stroke::new(1.5_f32, rgba(rgb, 0.55)));
    let brand = FontId::monospace((rect.height() * 0.05).max(8.0));
    clip.text(
        Pos2::new(rect.center().x, frame.top() + rect.height() * 0.045),
        Align2::CENTER_TOP,
        "DREAMSCAPE",
        brand,
        rgba(rgb, 0.95),
    );

    // Crimps: a dark strip with fine vertical ridges and a row of teeth.
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
        let step = (rect.width() / 70.0).max(2.0);
        let mut x = rect.left() + step * 0.5;
        let mut odd = false;
        while x < rect.right() {
            let col = if odd {
                Color32::from_white_alpha(34)
            } else {
                Color32::from_black_alpha(70)
            };
            clip.line_segment(
                [Pos2::new(x, top), Pos2::new(x, edge)],
                Stroke::new(step * 0.5, col),
            );
            odd = !odd;
            x += step;
        }
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
    // Tear notches in the sides of the top crimp: where a thumb starts the rip.
    let ny = rect.top() + ch * 1.15;
    for sx in [-1.0_f32, 1.0] {
        let edge_x = if sx < 0.0 { rect.left() } else { rect.right() };
        clip.add(Shape::convex_polygon(
            vec![
                Pos2::new(edge_x, ny - ch * 0.3),
                Pos2::new(edge_x - sx * ch * 0.45, ny),
                Pos2::new(edge_x, ny + ch * 0.3),
            ],
            rgba([6, 3, 16], 0.95),
            Stroke::new(1.0_f32, rgba(rgb, 0.8)),
        ));
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

    // The pack's name on a ribbon under the medallion.
    paint_ribbon(&clip, rect, theme, rgb);

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

    clip.rect_stroke(rect, 6.0, Stroke::new(2.0_f32, rgba(rgb, 0.9)));
}

/// The theme's name, split over two lines if long, on a ribbon with notched ends.
fn paint_ribbon(p: &egui::Painter, rect: Rect, theme: DreamTheme, rgb: [u8; 3]) {
    let title = crate::reveal_ui::theme_title(theme);
    let lines: Vec<String> = if title.len() > 10 && title.contains(' ') {
        let mid = title.len() / 2;
        let at = title
            .match_indices(' ')
            .map(|(i, _)| i)
            .min_by_key(|i| i.abs_diff(mid))
            .unwrap_or(mid);
        vec![title[..at].to_string(), title[at + 1..].to_string()]
    } else {
        vec![title]
    };
    let widest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1) as f32;
    let fit = rect.width() * 0.74 / (widest * 0.6);
    let size = (rect.height() * 0.06).min(fit).max(7.0);
    let h = size * (lines.len() as f32 * 1.15 + 0.6);
    let (mc, mr) = medallion(rect);
    let cy = mc.y + mr + (rect.bottom() - (mc.y + mr)) * 0.42;
    let w = rect.width() * 0.84;
    let band = Rect::from_center_size(Pos2::new(rect.center().x, cy), Vec2::new(w, h));
    let nt = h * 0.28;
    let ribbon = vec![
        Pos2::new(band.left(), band.top()),
        Pos2::new(band.right(), band.top()),
        Pos2::new(band.right() - nt, band.center().y),
        Pos2::new(band.right(), band.bottom()),
        Pos2::new(band.left(), band.bottom()),
        Pos2::new(band.left() + nt, band.center().y),
    ];
    p.add(Shape::convex_polygon(
        ribbon,
        rgba([10, 5, 22], 0.92),
        Stroke::new(1.5_f32, rgba(rgb, 0.9)),
    ));
    let line_h = size * 1.15;
    let mut y = band.center().y - line_h * lines.len() as f32 * 0.5;
    for l in &lines {
        p.text(
            Pos2::new(band.center().x, y),
            Align2::CENTER_TOP,
            l,
            FontId::monospace(size),
            rgba(rgb.map(|c| c.saturating_add(70)), 1.0),
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

/// The ragged line the strip tears along: `n` points across `rect`.
fn tear_edge(rect: Rect, n: usize, seed: u64) -> Vec<Pos2> {
    let ch = crimp_height(rect);
    let base = rect.top() + ch * 1.3;
    (0..=n)
        .map(|i| {
            let jag = (h01(i as u32, seed ^ 0x7EA2) - 0.5) * ch * 0.5;
            Pos2::new(rect.left() + rect.width() * i as f32 / n as f32, base + jag)
        })
        .collect()
}

/// A textured band between two edges, `alpha` 0..1 (the foil's own texture).
fn band_mesh(
    tex: egui::TextureId,
    rect: Rect,
    shift: Vec2,
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
            uv: uv_at(rect, shift, *a),
            color: tint,
        });
        m.vertices.push(egui::epaint::Vertex {
            pos: xf(*b),
            uv: uv_at(rect, shift, *b),
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
    let bright = rgb.map(|c| c.saturating_add(80));
    // 0.00-0.20 slide to the middle, 0.20-0.34 pull (shake), 0.34 rip,
    // 0.34-0.62 strip flies, 0.50-1.00 light + body drops.
    let slide = smooth(t / 0.2);
    let mut rect = Rect::from_min_max(
        w.from.min.lerp(tear_rect(screen).min, slide),
        w.from.max.lerp(tear_rect(screen).max, slide),
    );
    let pull = smooth((t - 0.2) / 0.14);
    let shake = pull * (1.0 - smooth((t - 0.34) / 0.05)) * rect.width() * 0.012;
    rect = rect.translate(Vec2::new(
        shake * (t * 97.0).sin(),
        shake * 0.6 * (t * 83.0).cos(),
    ));
    let drop = smooth((t - 0.62) / 0.38);
    let body_rect = rect.translate(Vec2::new(0.0, drop * rect.height() * 0.45));
    let fade = ((t - 0.66) / 0.3).clamp(0.0, 1.0);

    let ch = crimp_height(rect);
    let n = 18;
    let edge = tear_edge(rect, n, w.seed);
    let top_line: Vec<Pos2> = edge.iter().map(|q| Pos2::new(q.x, rect.top())).collect();
    let shift = Vec2::ZERO;
    let tex = art.texture(ctx, w.theme, w.seed);

    // Light spilling out of the opening, behind everything else: two soft
    // beams fading to nothing as they rise.
    let spill = smooth((t - 0.34) / 0.2) * (1.0 - 0.35 * smooth((t - 0.8) / 0.2));
    if spill > 0.0 {
        let ty = rect.top() + ch * 1.3;
        for (half_w, reach, a) in [(0.62, 1.0, 0.5_f32), (0.34, 0.8, 0.8_f32)] {
            let hw = rect.width() * half_w * (0.5 + 0.5 * spill);
            let ht = rect.height() * reach * spill;
            let glow = |alpha: f32| {
                Color32::from_rgba_unmultiplied(
                    bright[0],
                    bright[1],
                    bright[2],
                    (255.0 * alpha) as u8,
                )
            };
            let mut m = egui::Mesh::default();
            let cx = rect.center().x;
            m.colored_vertex(Pos2::new(cx - hw, ty), glow(a * spill));
            m.colored_vertex(Pos2::new(cx + hw, ty), glow(a * spill));
            m.colored_vertex(Pos2::new(cx + hw * 0.55, ty - ht), glow(0.0));
            m.colored_vertex(Pos2::new(cx - hw * 0.55, ty - ht), glow(0.0));
            m.add_triangle(0, 1, 2);
            m.add_triangle(0, 2, 3);
            p.add(Shape::mesh(m));
        }
    }

    // Before the rip the wrapper is whole; after it, a body and a loose strip.
    let ripped = t >= TEAR_RIP_AT;
    if !ripped {
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
        let body_clip_top = rect.top() + ch * 1.3 + ch * 0.3;
        if fade < 1.0 {
            let off = Vec2::new(0.0, body_rect.top() - rect.top());
            let visible = Rect::from_min_max(
                Pos2::new(body_rect.left() - 20.0, body_clip_top + off.y),
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
                .map(|q| Pos2::new(q.x, body_clip_top) + off + Vec2::new(0.0, 1.0))
                .collect();
            p.add(band_mesh(
                tex,
                body_rect,
                shift,
                &lip_top,
                &lip_bot,
                &|q| q,
                1.0,
            ));
            p.add(Shape::line(
                lip_top,
                Stroke::new(1.5_f32, Color32::from_white_alpha(170)),
            ));
            if fade > 0.0 {
                p.rect_filled(body_rect.expand(24.0), 8.0, rgba([6, 3, 16], fade));
            }
        }

        // The strip: the top crimp, hinged at the right end, then thrown off.
        let rip = 0.18 + smooth((t - TEAR_RIP_AT) / 0.28) * 1.2;
        let fly = smooth((t - TEAR_RIP_AT) / 0.3);
        let alpha = 1.0 - smooth((t - 0.5) / 0.14);
        if alpha > 0.0 {
            let pivot = Pos2::new(rect.right(), rect.top() + ch * 0.6);
            let throw = Vec2::new(
                fly * rect.width() * 0.5,
                -fly * rect.height() * 0.5 + fly * fly * rect.height() * 0.12,
            );
            let (sa, ca) = (-rip).sin_cos();
            let xf = move |q: Pos2| {
                let d = q - pivot;
                pivot + Vec2::new(d.x * ca - d.y * sa, d.x * sa + d.y * ca) + throw
            };
            p.add(band_mesh(tex, rect, shift, &top_line, &edge, &xf, alpha));
            let foil = rgba([rgb[0] / 3, rgb[1] / 3, rgb[2] / 3], 0.95 * alpha);
            let band_bot = rect.top() + ch * 0.55;
            p.add(Shape::convex_polygon(
                vec![
                    xf(Pos2::new(rect.left(), rect.top())),
                    xf(Pos2::new(rect.right(), rect.top())),
                    xf(Pos2::new(rect.right(), band_bot)),
                    xf(Pos2::new(rect.left(), band_bot)),
                ],
                foil,
                Stroke::NONE,
            ));
            let teeth = 14usize;
            let tw = rect.width() / teeth as f32;
            for i in 0..teeth {
                let x = rect.left() + i as f32 * tw;
                p.add(Shape::convex_polygon(
                    vec![
                        xf(Pos2::new(x, band_bot)),
                        xf(Pos2::new(x + tw, band_bot)),
                        xf(Pos2::new(x + tw * 0.5, rect.top() + ch)),
                    ],
                    foil,
                    Stroke::NONE,
                ));
            }
            p.add(Shape::line(
                edge.iter().map(|q| xf(*q)).collect(),
                Stroke::new(1.5_f32, Color32::from_white_alpha((180.0 * alpha) as u8)),
            ));
            p.add(Shape::closed_line(
                vec![
                    xf(Pos2::new(rect.left(), rect.top())),
                    xf(Pos2::new(rect.right(), rect.top())),
                    xf(edge[n]),
                    xf(edge[0]),
                ],
                Stroke::new(2.0_f32, rgba(rgb, 0.9 * alpha)),
            ));
        }
    }

    // Sparks fly out of the opening once it lets go.
    let burst = (t - TEAR_RIP_AT) / (1.0 - TEAR_RIP_AT);
    if burst > 0.0 {
        let origin = Pos2::new(rect.center().x, rect.top() + ch * 1.3);
        for k in 0..28u32 {
            let ang = -std::f32::consts::PI * (0.08 + 0.84 * h01(k, w.seed ^ 0x5A));
            let spd = 0.4 + 0.9 * h01(k + 50, w.seed);
            let d = (burst * 1.4).min(1.0);
            let at = origin
                + Vec2::new(ang.cos(), ang.sin()) * rect.height() * 0.7 * spd * d
                + Vec2::new(0.0, d * d * rect.height() * 0.2);
            let a = (1.0 - burst).clamp(0.0, 1.0);
            p.rect_filled(
                Rect::from_center_size(at, Vec2::splat(3.0 + 3.0 * h01(k + 90, w.seed))),
                0.0,
                rgba(bright, a * 0.9),
            );
        }
    }
    if flashes {
        let f =
            ((t - TEAR_RIP_AT) / 0.1).clamp(0.0, 1.0) * (1.0 - ((t - 0.44) / 0.2).clamp(0.0, 1.0));
        if f > 0.0 {
            p.rect_filled(screen, 0.0, rgba([255, 244, 220], f * 0.35));
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
        let e = tear_edge(rect, 18, 5);
        assert_eq!(e.len(), 19);
        assert_eq!(e[0].x, rect.left());
        assert_eq!(e[18].x, rect.right());
        // Jagged, but always below the crimp teeth and above the clip line.
        let ch = crimp_height(rect);
        for q in &e {
            assert!(
                q.y > rect.top() + ch && q.y < rect.top() + ch * 1.6,
                "{q:?}"
            );
        }
        assert_eq!(TEAR_TIME, 1.5);
        assert!(
            tear_rect(Rect::from_min_size(Pos2::ZERO, Vec2::new(640.0, 360.0))).height() < 360.0
        );
    }

    #[test]
    fn ribbons_fit_every_theme_name() {
        for &t in ALL_THEMES.iter() {
            let title = crate::reveal_ui::theme_title(t);
            let rect = Rect::from_min_size(Pos2::ZERO, PACK_SIZE);
            let (mc, mr) = medallion(rect);
            let ribbon_top = mc.y + mr;
            assert!(
                ribbon_top < rect.bottom() - crimp_height(rect) * 2.0,
                "{title}"
            );
        }
    }
}
