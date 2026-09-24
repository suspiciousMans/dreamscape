//! Pack opening and the Lucid Store: the two screens between waking up and
//! dreaming again. Same pixel + VT323 language as the rest of the HUD.

use crate::booklet_ui::{card, rarity_color, CardArt, CARD_SIZE};
use crate::cards::Recalled;
use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use crate::store::{Item, State, CATALOG};
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect, Stroke, Vec2};

/// Seconds per card in the reveal.
pub const REVEAL_STEP: f32 = 0.9;

/// Which reveal stage each card is in at time `t`:
/// 0 = face down, 1 = flipping, 2 = revealed.
pub fn reveal_stage(i: usize, t: f32) -> u8 {
    let start = 0.6 + i as f32 * REVEAL_STEP;
    if t < start {
        0
    } else if t < start + 0.35 {
        1
    } else {
        2
    }
}

pub fn reveal_done(n: usize, t: f32) -> bool {
    n == 0 || reveal_stage(n - 1, t) == 2
}

#[derive(Clone, Debug, Default)]
pub struct PackView {
    pub pack: Vec<Recalled>,
    /// Seconds since the pack began opening; `f32::MAX` = skipped.
    pub age: f32,
    pub pressed: bool,
    pub dust_earned: u32,
    pub cards_added: usize,
}

pub fn draw_reveal(
    ctx: &egui::Context,
    p: &egui::Painter,
    screen: Rect,
    v: &HudView,
    pk: &PackView,
    art: &mut CardArt,
) {
    p.rect_filled(screen, 0.0, rgba([6, 3, 16], 0.95));
    let cx = screen.center().x;
    glitch_text(
        p,
        Pos2::new(cx, screen.top() + 22.0),
        true,
        "WHAT DO YOU REMEMBER?",
        40.0,
        hue(v.time * 0.1),
        1.0,
        v,
    );
    // Small cards, laid out in rows of up to 6.
    let scale = 0.62;
    let size = CARD_SIZE * scale;
    let gap = 14.0;
    let per_row = 6usize;
    for (i, r) in pk.pack.iter().enumerate() {
        let row = i / per_row;
        let col = i % per_row;
        let in_row = (pk.pack.len() - row * per_row).min(per_row) as f32;
        let row_w = in_row * size.x + (in_row - 1.0) * gap;
        let min = Pos2::new(
            cx - row_w * 0.5 + col as f32 * (size.x + gap),
            screen.top() + 80.0 + row as f32 * (size.y + gap),
        );
        let rect = Rect::from_min_size(min, size);
        match reveal_stage(i, pk.age) {
            0 => card_back(p, rect, v, false),
            1 => {
                // Flip: squash horizontally around the centre.
                let t = ((pk.age - 0.6 - i as f32 * REVEAL_STEP) / 0.35).clamp(0.0, 1.0);
                let w = (1.0 - 2.0 * t).abs();
                let squashed =
                    Rect::from_center_size(rect.center(), Vec2::new(size.x * w.max(0.04), size.y));
                if t < 0.5 {
                    card_back(p, squashed, v, true);
                } else {
                    p.rect_filled(
                        squashed,
                        0.0,
                        rgba(rarity_color(r.card.rarity, v.time), 0.9),
                    );
                }
            }
            _ => {
                if r.remembered {
                    let clip = p.with_clip_rect(rect);
                    card(
                        ctx,
                        &clip.with_clip_rect(rect),
                        rect,
                        &r.card,
                        v,
                        art,
                        scale,
                    );
                } else {
                    faded(p, rect, r, v);
                }
            }
        }
    }
    let done = reveal_done(pk.pack.len(), pk.age);
    let kept = pk.pack.iter().filter(|r| r.remembered).count();
    let msg = if !done {
        "remembering...    [space] skip".to_string()
    } else if pk.pressed {
        format!(
            "pressed {} cards · +{} dust · you hold {} dust",
            pk.cards_added, pk.dust_earned, v.dust
        )
    } else {
        format!("{kept} of {} dreams stayed with you", pk.pack.len())
    };
    p.text(
        Pos2::new(cx, screen.bottom() - 92.0),
        Align2::CENTER_TOP,
        msg,
        FontId::monospace(24.0),
        rgba([255, 210, 80], 0.95),
    );
    if done {
        let keys = if pk.pressed {
            "[b] booklet   [l] lucid store   [r] dream again   [esc] wake"
        } else {
            "[s] press into booklet   [r] dream again   [esc] wake"
        };
        p.text(
            Pos2::new(cx, screen.bottom() - 54.0),
            Align2::CENTER_TOP,
            keys,
            FontId::monospace(22.0),
            rgba(ink(v), 0.5 + 0.3 * (v.time * 2.0).sin()),
        );
    }
}

/// The face-down pack card: a pixel eye, the same one on every back.
fn card_back(p: &egui::Painter, r: Rect, v: &HudView, glowing: bool) {
    p.rect_filled(r, 0.0, rgba([24, 12, 44], 1.0));
    p.rect_stroke(
        r.shrink(3.0),
        0.0,
        Stroke::new(
            3.0_f32,
            rgba(hue(v.time * 0.2), if glowing { 1.0 } else { 0.55 }),
        ),
    );
    if r.width() < 40.0 {
        return;
    }
    let c = r.center();
    let px = (r.width() / 22.0).round().max(2.0);
    for (x, y, k) in crate::pixels::eye_pixels(0.8, None, false) {
        let col = match k {
            crate::pixels::EyePx::Lid | crate::pixels::EyePx::Lash => rgba(ink(v), 0.6),
            crate::pixels::EyePx::Pupil => rgba([6, 0, 12], 1.0),
            crate::pixels::EyePx::Sclera => continue,
            _ => rgba(hue(0.75), 0.8),
        };
        p.rect_filled(
            Rect::from_center_size(c + Vec2::new(x as f32 * px, y as f32 * px), Vec2::splat(px)),
            0.0,
            col,
        );
    }
}

/// A dream that didn't stay: a greyed-out ghost of its name, and the dust it left.
fn faded(p: &egui::Painter, r: Rect, rec: &Recalled, v: &HudView) {
    p.rect_filled(r, 0.0, rgba([14, 10, 22], 0.9));
    p.rect_stroke(r.shrink(3.0), 0.0, Stroke::new(1.0_f32, rgba(ink(v), 0.15)));
    // Dissolving pixel dust.
    for k in 0..40u32 {
        let h = (k.wrapping_mul(2_654_435_761) ^ rec.card.seed as u32) as f32 / u32::MAX as f32;
        let h2 = (k.wrapping_mul(40_503) ^ (rec.card.seed >> 7) as u32) as f32 / u32::MAX as f32;
        let drift = (v.time * 8.0 + k as f32 * 3.0) % r.height();
        let at = Pos2::new(
            r.left() + h * r.width(),
            r.bottom() - (h2 * r.height() + drift) % r.height(),
        );
        p.rect_filled(
            Rect::from_center_size(at, Vec2::splat(3.0)),
            0.0,
            rgba(ink(v), 0.18),
        );
    }
    let name = p.layout(
        rec.card.name.clone(),
        FontId::monospace(15.0),
        rgba(ink(v), 0.3),
        r.width() - 16.0,
    );
    p.galley(r.left_top() + Vec2::new(8.0, 24.0), name, rgba(ink(v), 0.3));
    p.text(
        r.center(),
        Align2::CENTER_CENTER,
        "FADED",
        FontId::monospace(22.0),
        rgba(ink(v), 0.35),
    );
    p.text(
        r.center() + Vec2::new(0.0, 26.0),
        Align2::CENTER_CENTER,
        format!("+{} dust", crate::cards::fade_dust(rec.card.rarity)),
        FontId::monospace(16.0),
        rgba([255, 210, 80], 0.7),
    );
}

pub fn draw_store(p: &egui::Painter, screen: Rect, v: &HudView) {
    p.rect_filled(screen, 0.0, rgba([6, 3, 16], 0.95));
    let cx = screen.center().x;
    glitch_text(
        p,
        Pos2::new(cx, screen.top() + 22.0),
        true,
        "THE LUCID STORE",
        44.0,
        hue(0.12 + 0.03 * (v.time * 0.7).sin()),
        1.0,
        v,
    );
    p.text(
        Pos2::new(cx, screen.top() + 76.0),
        Align2::CENTER_TOP,
        format!("{} dream dust", v.dust),
        FontId::monospace(26.0),
        rgba([255, 210, 80], 1.0),
    );
    let top = screen.top() + 118.0;
    let row_h = 26.0;
    let left = cx - 330.0;
    let mut section = "";
    let mut y = top;
    for (i, item) in CATALOG.iter().enumerate() {
        let sec = match item {
            Item::Perk(_) => "RUN PERKS · used up next run",
            Item::Eye(_) => "EYES",
            Item::Card(_) => "CARD FRAMES",
            Item::Hud(_) => "HUD PALETTES",
        };
        if sec != section {
            section = sec;
            y += 6.0;
            p.text(
                Pos2::new(left, y),
                Align2::LEFT_TOP,
                sec,
                FontId::monospace(18.0),
                rgba(ink(v), 0.45),
            );
            y += 22.0;
        }
        let info = item.info();
        let selected = i == v.store_cursor;
        if selected {
            p.rect_filled(
                Rect::from_min_size(Pos2::new(left - 10.0, y - 2.0), Vec2::new(680.0, row_h)),
                0.0,
                rgba(hue(v.time * 0.2), 0.18),
            );
            p.text(
                Pos2::new(left - 8.0, y),
                Align2::RIGHT_TOP,
                ">",
                FontId::monospace(22.0),
                rgba(ink(v), 1.0),
            );
        }
        let (tag, tag_col) = match v.store_states[i] {
            State::Buy(price) if price <= v.dust => (format!("{price} dust"), [255, 210, 80]),
            State::Buy(price) => (format!("{price} dust"), [120, 100, 90]),
            State::Armed => ("ARMED".to_string(), [80, 230, 200]),
            State::Owned => ("owned".to_string(), [170, 160, 200]),
            State::Equipped => ("EQUIPPED".to_string(), [120, 255, 140]),
        };
        p.text(
            Pos2::new(left + 6.0, y),
            Align2::LEFT_TOP,
            info.name,
            FontId::monospace(21.0),
            rgba(ink(v), if selected { 1.0 } else { 0.8 }),
        );
        p.text(
            Pos2::new(left + 200.0, y + 2.0),
            Align2::LEFT_TOP,
            info.blurb,
            FontId::proportional(18.0),
            rgba([200, 190, 230], 0.75),
        );
        p.text(
            Pos2::new(left + 670.0, y),
            Align2::RIGHT_TOP,
            tag,
            FontId::monospace(20.0),
            rgba(tag_col, 1.0),
        );
        y += row_h;
    }
    if let Some(msg) = &v.store_status {
        p.text(
            Pos2::new(cx, screen.bottom() - 84.0),
            Align2::CENTER_TOP,
            msg,
            FontId::monospace(22.0),
            rgba([255, 210, 80], 0.95),
        );
    }
    p.text(
        Pos2::new(cx, screen.bottom() - 48.0),
        Align2::CENTER_TOP,
        "[w/s] browse   [enter] buy / equip   [esc] back",
        FontId::monospace(22.0),
        rgba(ink(v), 0.5 + 0.3 * (v.time * 2.0).sin()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards_reveal_in_order_and_the_pack_finishes() {
        assert_eq!(reveal_stage(0, 0.0), 0);
        assert_eq!(reveal_stage(0, 0.7), 1);
        assert_eq!(reveal_stage(0, 1.2), 2);
        assert_eq!(reveal_stage(1, 1.2), 0, "second card waits its turn");
        assert!(!reveal_done(3, 1.0));
        assert!(reveal_done(3, 0.6 + 2.0 * REVEAL_STEP + 0.36));
        assert!(reveal_done(0, 0.0));
        assert!(reveal_done(5, f32::MAX), "skip reveals everything");
    }
}
