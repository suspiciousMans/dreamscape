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
    fn texture(&mut self, ctx: &egui::Context, card: &Card) -> egui::TextureId {
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
    }
}

pub fn draw(ctx: &egui::Context, p: &egui::Painter, screen: Rect, v: &HudView, art: &mut CardArt) {
    p.rect_filled(screen, 0.0, rgba([8, 4, 20], 0.94));
    let cx = screen.center().x;
    glitch_text(
        p,
        Pos2::new(cx, screen.top() + 24.0),
        true,
        "DREAM BOOKLET",
        44.0,
        hue(v.time * 0.1),
        1.0,
        v,
    );
    p.text(
        Pos2::new(cx, screen.top() + 78.0),
        Align2::CENTER_TOP,
        format!(
            "page {} / {}  ·  {} cards",
            v.booklet_page + 1,
            v.booklet_pages,
            v.booklet_total
        ),
        FontId::monospace(20.0),
        rgba(ink(v), 0.6),
    );
    if v.booklet_cards.is_empty() {
        p.text(
            screen.center(),
            Align2::CENTER_CENTER,
            "your booklet is empty. wake up, then press [s].",
            FontId::monospace(24.0),
            rgba(ink(v), 0.8),
        );
    }
    let n = v.booklet_cards.len() as f32;
    let gap = 28.0;
    let total_w = n * CARD_SIZE.x + (n - 1.0).max(0.0) * gap;
    for (i, c) in v.booklet_cards.iter().enumerate() {
        let min = Pos2::new(
            cx - total_w * 0.5 + i as f32 * (CARD_SIZE.x + gap),
            screen.top() + 112.0,
        );
        card(ctx, p, Rect::from_min_size(min, CARD_SIZE), c, v, art, 1.0);
    }
    p.text(
        Pos2::new(cx, screen.bottom() - 40.0),
        Align2::CENTER_TOP,
        "[a/d] turn page      [esc] back",
        FontId::monospace(22.0),
        rgba(ink(v), 0.5 + 0.3 * (v.time * 2.0).sin()),
    );
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
    let frame = match v.card_style {
        crate::store::CardStyle::Plain => rarity_color(c.rarity, v.time),
        crate::store::CardStyle::Gilt => [255, 205, 90],
        crate::store::CardStyle::Holo => hue(v.time * 0.3 + c.number as f32 * 0.13),
    };
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
        format!("No.{:03}", c.number),
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
