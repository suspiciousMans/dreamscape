//! The dream booklet: trading cards in the same pixel + VT323 language as the HUD.

use crate::cards::{Attribute, Card, Rarity};
use crate::dream::TEX_SIZE;
use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use engine::ui::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};
use std::collections::HashMap;

pub const CARD_SIZE: Vec2 = Vec2::new(236.0, 360.0);
const ART_H: f32 = 110.0;

/// Card art textures, uploaded to egui on first sight and kept.
#[derive(Default)]
pub struct CardArt {
    cache: HashMap<u64, egui::TextureHandle>,
}

impl CardArt {
    pub(crate) fn texture(&mut self, ctx: &egui::Context, card: &Card) -> egui::TextureId {
        self.cache
            .entry(card.seed)
            .or_insert_with(|| {
                let size = TEX_SIZE as usize;
                let image =
                    egui::ColorImage::from_rgba_unmultiplied([size, size], &card.art.rgba());
                ctx.load_texture(
                    format!("card-{}", card.seed),
                    image,
                    egui::TextureOptions::NEAREST,
                )
            })
            .id()
    }
}

pub fn rarity_color(r: Rarity, time: f32) -> [u8; 3] {
    match r {
        Rarity::Faint => [150, 150, 170],
        Rarity::Hazy => [80, 220, 200],
        Rarity::Vivid => [255, 80, 200],
        Rarity::Lucid => [255, 210, 80],
        Rarity::Prophetic => hue(time * 0.4),
        // Per-card colours come from `card_frame`; these are the fallbacks.
        Rarity::Fused => [255, 150, 90],
        Rarity::Resonant => resonant_color(time),
    }
}

/// A resonant frame: the whole wheel, with a white pulse.
pub fn resonant_color(time: f32) -> [u8; 3] {
    let c = hue(time * 0.8);
    let w = (0.5 + 0.5 * (time * 4.0).sin()).powi(4);
    c.map(|v| (v as f32 + (255.0 - v as f32) * w) as u8)
}

/// A fused frame cycles between the two dreams' first accents.
pub fn fused_color(a: [u8; 3], b: [u8; 3], time: f32) -> [u8; 3] {
    let t = 0.5 + 0.5 * (time * 1.5).sin();
    [0, 1, 2].map(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t) as u8)
}

/// The frame colour for this particular card.
pub fn card_frame(card: &Card, time: f32) -> [u8; 3] {
    match (card.rarity, card.fused) {
        (Rarity::Fused, Some(b)) => {
            fused_color(card.theme.spec().accents[0], b.spec().accents[0], time)
        }
        (r, _) => rarity_color(r, time),
    }
}

fn attribute_color(a: Attribute) -> [u8; 3] {
    match a {
        Attribute::Velvet => [190, 150, 255],
        Attribute::Static => [220, 230, 120],
        Attribute::Void => [120, 140, 255],
        Attribute::Bloom => [120, 230, 120],
        Attribute::Rust => [230, 110, 60],
        Attribute::Dawn => [255, 220, 150],
        Attribute::Hex => [110, 255, 140],
        Attribute::Tide => [80, 200, 255],
        Attribute::Aether => [235, 245, 255],
        Attribute::Glass => [255, 170, 250],
        Attribute::Spore => [120, 255, 200],
        Attribute::Abyss => [150, 110, 255],
        Attribute::Halo => [255, 215, 110],
        Attribute::Jest => [255, 110, 230],
        Attribute::Trace => [255, 170, 220],
        Attribute::Chord => [120, 255, 200],
        Attribute::Hour => [255, 190, 90],
        Attribute::Lumen => [120, 200, 255],
        Attribute::Gaze => [220, 200, 150],
        Attribute::Blank => [240, 240, 255],
    }
}

/// Where the cards on a page sit (also used to crop exported card images).
pub fn card_rects(screen: Rect, n: usize) -> Vec<Rect> {
    let cx = screen.center().x;
    let gap = 28.0;
    let total_w = n as f32 * CARD_SIZE.x + (n as f32 - 1.0).max(0.0) * gap;
    (0..n)
        .map(|i| {
            let min = Pos2::new(
                cx - total_w * 0.5 + i as f32 * (CARD_SIZE.x + gap),
                screen.top() + 112.0,
            );
            Rect::from_min_size(min, CARD_SIZE)
        })
        .collect()
}

pub fn draw(ctx: &egui::Context, p: &egui::Painter, screen: Rect, v: &HudView, art: &mut CardArt) {
    p.rect_filled(screen, 0.0, rgba([8, 4, 20], 0.94));
    let u = crate::pixel_ui::scale(screen);
    let cell = crate::pixel_ui::cell(u);
    let cx = screen.center().x;
    glitch_text(
        p,
        Pos2::new(cx, screen.top() + 24.0),
        true,
        "DREAM BOOKLET",
        44.0 * u,
        hue(v.time * 0.1),
        1.0,
        v,
    );
    // Page indicator: a pixel pip per page when there are few, else a count.
    let label = format!(
        "page {} / {}  ·  {} cards",
        v.booklet_page + 1,
        v.booklet_pages,
        v.booklet_total
    );
    p.text(
        Pos2::new(cx, screen.top() + 78.0 * u),
        Align2::CENTER_TOP,
        label,
        FontId::monospace(20.0 * u.max(0.8)),
        rgba(ink(v), 0.6),
    );
    p.text(
        Pos2::new(cx, screen.bottom() - 96.0 * u),
        Align2::CENTER_BOTTOM,
        &v.booklet_attune,
        FontId::monospace(18.0 * u.max(0.8)),
        rgba(hue(v.time * 0.05 + 0.3), 0.85),
    );
    if (2..=24).contains(&v.booklet_pages) {
        let pip = cell;
        let gap = pip * 1.6;
        let total = v.booklet_pages as f32 * gap;
        for i in 0..v.booklet_pages {
            let r = Rect::from_min_size(
                Pos2::new(cx - total * 0.5 + i as f32 * gap, screen.top() + 104.0 * u),
                Vec2::splat(pip),
            );
            if i == v.booklet_page {
                p.rect_filled(r, 0.0, rgba(ink(v), 0.95));
            } else {
                p.rect_filled(r, 0.0, rgba(ink(v), 0.2));
            }
        }
    }
    if v.booklet_cards.is_empty() {
        p.text(
            screen.center(),
            Align2::CENTER_CENTER,
            "your booklet is empty. wake up, then press [s].",
            FontId::monospace(24.0 * u.max(0.8)),
            rgba(ink(v), 0.8),
        );
    }
    let rects = card_rects(screen, v.booklet_cards.len());
    // A shelf behind the cards, so the page reads as a page.
    if let (Some(first), Some(last)) = (rects.first(), rects.last()) {
        let shelf =
            Rect::from_min_max(first.min, last.max).expand2(Vec2::new(cell * 4.0, cell * 3.0));
        crate::pixel_ui::panel(p, shelf, [90, 70, 150], rgba([10, 5, 24], 0.6), cell, 0.6);
    }
    for (c, r) in v.booklet_cards.iter().zip(&rects) {
        card(ctx, p, *r, c, v, art, 1.0);
    }
    // Turn-page arrows, dim when there's no page that way.
    for (dir, ok) in [
        (-1.0_f32, v.booklet_page > 0),
        (1.0, v.booklet_page + 1 < v.booklet_pages),
    ] {
        let at = Pos2::new(
            cx + dir * (screen.width() * 0.5 - 40.0 * u),
            screen.center().y,
        );
        let col = rgba(ink(v), if ok { 0.8 } else { 0.15 });
        if dir > 0.0 {
            crate::pixel_ui::arrow(p, at, cell, col);
        } else {
            // Mirrored: a left-pointing arrow.
            for (i, h) in [5.0_f32, 3.0, 1.0].into_iter().enumerate() {
                p.rect_filled(
                    Rect::from_min_size(
                        at + Vec2::new(-1.5 * cell - i as f32 * cell, -h * 0.5 * cell),
                        Vec2::new(cell, h * cell),
                    ),
                    0.0,
                    col,
                );
            }
            p.rect_filled(
                Rect::from_min_size(
                    at + Vec2::new(-0.5 * cell, -0.5 * cell),
                    Vec2::new(4.0 * cell, cell),
                ),
                0.0,
                col,
            );
        }
    }
    crate::loadout_ui::hint_bar(
        p,
        screen,
        v,
        &format!(
            "[a/d] turn page   [t] sort: {}   [r] flip   [p] save as images   [esc] back",
            v.booklet_sort
        ),
    );
}

/// The frame colour for a bought card frame style; `plain` is what the
/// default frame would be for this card.
pub(crate) fn style_frame(
    style: crate::store::CardStyle,
    plain: [u8; 3],
    time: f32,
    number: u32,
) -> [u8; 3] {
    use crate::store::CardStyle::*;
    match style {
        Plain => plain,
        Gilt => [255, 205, 90],
        Holo => hue(time * 0.3 + number as f32 * 0.13),
        Ember => [255, (110.0 + 50.0 * (time * 3.0 + number as f32).sin()) as u8, 30],
        Frost => [170, 225, 255],
        Neon => hue(time * 0.9 + number as f32 * 0.07),
        Bone => [225, 215, 180],
    }
}

pub fn card(
    ctx: &egui::Context,
    p: &egui::Painter,
    r: Rect,
    c: &Card,
    v: &HudView,
    art: &mut CardArt,
    scale: f32,
) {
    let frame = style_frame(v.card_style, card_frame(c, v.time), v.time, c.number);
    let s = scale;
    p.rect_filled(r, 0.0, rgba([16, 9, 30], 1.0));
    p.rect_stroke(r.shrink(2.0), 0.0, Stroke::new(4.0_f32, rgba(frame, 1.0)));
    p.rect_stroke(r.shrink(8.0), 0.0, Stroke::new(1.0_f32, rgba(frame, 0.45)));
    // Chipped pixel corners.
    for corner in [
        r.left_top(),
        r.right_top() - Vec2::new(6.0, 0.0),
        r.left_bottom() - Vec2::new(0.0, 6.0),
        r.right_bottom() - Vec2::splat(6.0),
    ] {
        p.rect_filled(
            Rect::from_min_size(corner, Vec2::splat(6.0)),
            0.0,
            rgba([8, 4, 20], 1.0),
        );
    }
    let inner = r.shrink(14.0 * s);
    let attr = attribute_color(c.attribute);
    let pill = Rect::from_min_size(inner.left_top(), Vec2::new(78.0, 22.0) * s);
    p.rect_filled(pill, 0.0, rgba(attr, 0.9));
    p.text(
        pill.center(),
        Align2::CENTER_CENTER,
        c.attribute.label(),
        FontId::monospace(18.0 * s),
        rgba([10, 5, 20], 1.0),
    );
    p.text(
        inner.right_top(),
        Align2::RIGHT_TOP,
        if c.number == 0 {
            "NEW".to_string()
        } else {
            format!("No.{:03}", c.number)
        },
        FontId::monospace(18.0 * s),
        rgba(ink(v), 0.6),
    );

    let name = p.layout(
        c.name.clone(),
        FontId::monospace(22.0 * s),
        rgba(ink(v), 1.0),
        inner.width(),
    );
    let name_h = name.size().y;
    p.galley(
        inner.left_top() + Vec2::new(0.0, 28.0 * s),
        name,
        Color32::WHITE,
    );

    let art_rect = Rect::from_min_size(
        inner.left_top() + Vec2::new(0.0, 32.0 * s + name_h),
        Vec2::new(inner.width(), ART_H * s),
    );
    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, ART_H * s / inner.width()));
    p.image(art.texture(ctx, c), art_rect, uv, Color32::WHITE);
    p.rect_stroke(art_rect, 0.0, Stroke::new(2.0_f32, rgba(frame, 0.8)));
    if c.rarity >= Rarity::Lucid {
        foil(p, art_rect, v.time);
    }
    if c.recurring {
        let tag = Rect::from_min_size(
            art_rect.left_bottom() - Vec2::new(0.0, 20.0 * s),
            Vec2::new(96.0, 20.0) * s,
        );
        p.rect_filled(tag, 0.0, rgba([255, 210, 80], 0.95));
        p.text(
            tag.center(),
            Align2::CENTER_CENTER,
            "RECURRING",
            FontId::monospace(16.0 * s),
            rgba([20, 10, 0], 1.0),
        );
    }

    let mut y = art_rect.bottom() + 8.0 * s;
    for (label, val) in [("DREAD", c.dread), ("DRIFT", c.drift)] {
        p.text(
            Pos2::new(inner.left(), y),
            Align2::LEFT_TOP,
            label,
            FontId::monospace(18.0 * s),
            rgba(ink(v), 0.8),
        );
        for k in 0..9u8 {
            let cell = Rect::from_min_size(
                Pos2::new(inner.left() + (64.0 + k as f32 * 14.0) * s, y + 4.0 * s),
                Vec2::splat(10.0 * s),
            );
            if k < val {
                p.rect_filled(cell, 0.0, rgba(attr, 0.95));
            } else {
                p.rect_stroke(cell, 0.0, Stroke::new(1.0_f32, rgba(ink(v), 0.3)));
            }
        }
        y += 22.0 * s;
    }
    let desc = p.layout(
        c.description.clone(),
        FontId::proportional(19.0 * s),
        rgba([200, 190, 230], 0.95),
        inner.width(),
    );
    p.galley(Pos2::new(inner.left(), y + 4.0 * s), desc, Color32::WHITE);

    p.text(
        inner.left_bottom(),
        Align2::LEFT_BOTTOM,
        c.rarity.label(),
        FontId::monospace(20.0 * s),
        rgba(frame, 1.0),
    );
    p.text(
        inner.right_bottom(),
        Align2::RIGHT_BOTTOM,
        format!("depth {}", c.depth),
        FontId::monospace(18.0 * s),
        rgba(ink(v), 0.55),
    );
    worth_strip(p, inner, c, v, s, frame);
    if crate::worth::is_foil(c) {
        // A foil card shines back: rainbow edge and a second, brighter sheen.
        let edge = hue(v.time * 0.5 + c.number as f32 * 0.07);
        p.rect_stroke(r.shrink(2.0), 0.0, Stroke::new(4.0_f32, rgba(edge, 0.85)));
        foil(p, art_rect, v.time + 0.5);
        let tag = Rect::from_min_size(
            art_rect.right_top() - Vec2::new(52.0 * s, 0.0),
            Vec2::new(52.0, 20.0) * s,
        );
        p.rect_filled(tag, 0.0, rgba(edge, 0.95));
        p.text(
            tag.center(),
            Align2::CENTER_CENTER,
            "FOIL",
            FontId::monospace(16.0 * s),
            rgba([10, 5, 20], 1.0),
        );
    }
}

/// The line above the card's foot: its quirk, its worth, its mastery level.
fn worth_strip(p: &egui::Painter, inner: Rect, c: &Card, v: &HudView, s: f32, frame: [u8; 3]) {
    let level = v
        .mastery_levels
        .iter()
        .find(|(n, _)| *n == c.number)
        .map_or(0, |&(_, l)| l);
    let mut bits: Vec<String> = Vec::new();
    if let Some(q) = crate::worth::quirk_of(c) {
        bits.push(crate::worth::label(q).to_string());
    }
    bits.push(format!("WORTH {}", crate::worth::worth(c)));
    if level > 0 {
        bits.push(format!("LV{level}"));
    }
    let y = inner.bottom() - 24.0 * s;
    let strip = Rect::from_min_max(
        Pos2::new(inner.left() - 4.0 * s, y - 20.0 * s),
        Pos2::new(inner.right() + 4.0 * s, y),
    );
    p.rect_filled(strip, 0.0, rgba([16, 9, 30], 0.96));
    p.text(
        strip.center(),
        Align2::CENTER_CENTER,
        bits.join("  ·  "),
        FontId::monospace(16.0 * s),
        rgba(if level >= 5 { hue(v.time * 0.4) } else { frame }, 0.95),
    );
}

/// A diagonal sheen sweeping across the art (Lucid / Prophetic foil).
fn foil(p: &egui::Painter, r: Rect, time: f32) {
    let clip = p.with_clip_rect(r);
    let x = r.left() - 60.0 + (time * 120.0).rem_euclid(r.width() + 120.0);
    let pts = vec![
        Pos2::new(x, r.bottom()),
        Pos2::new(x + 30.0, r.bottom()),
        Pos2::new(x + 70.0, r.top()),
        Pos2::new(x + 40.0, r.top()),
    ];
    clip.add(egui::Shape::convex_polygon(
        pts,
        Color32::from_rgba_unmultiplied(255, 255, 255, 60),
        Stroke::NONE,
    ));
}
