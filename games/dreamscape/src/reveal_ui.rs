//! Pack opening and the Lucid Store: the two screens between waking up and
//! dreaming again. Same pixel + VT323 language as the rest of the HUD.

use crate::booklet_ui::{card, CardArt, CARD_SIZE};
use crate::cards::Recalled;
use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect, Stroke, Vec2};

/// Seconds per card in the reveal.
pub const REVEAL_STEP: f32 = 0.9;

/// However long the run, the whole pack flips within this many seconds.
pub const REVEAL_BUDGET: f32 = 10.0;

/// Seconds between one card flipping and the next.
pub fn reveal_step(n: usize) -> f32 {
    REVEAL_STEP.min(REVEAL_BUDGET / n.max(1) as f32).max(0.05)
}

/// Seconds before the first flip, and how long a flip takes.
const LEAD_IN: f32 = 0.6;
const FLIP_TIME: f32 = 0.35;
/// Extra suspense before the best card of a pack.
const FINAL_LINGER: f32 = 0.9;
/// How long a near miss shows its face before it dissolves.
pub const NEAR_MISS_TIME: f32 = 1.0;

/// How much a card matters to the player: faded dreams below remembered
/// ones, then by rarity (the derived `Ord`: Fused/Resonant above Prophetic).
fn excitement(r: &Recalled) -> (bool, crate::cards::Rarity) {
    (r.remembered, r.card.rarity)
}

/// Pack indices in the order they are revealed: dullest first, best last.
/// Stable: equal cards keep their pack order.
pub fn reveal_order(pack: &[Recalled]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..pack.len()).collect();
    order.sort_by_key(|&i| excitement(&pack[i]));
    order
}

/// A faded dream of at least Vivid rarity: it almost stayed.
pub fn near_miss(r: &Recalled) -> bool {
    !r.remembered && r.card.rarity >= crate::cards::Rarity::Vivid
}

/// What one flip feels like.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beat {
    /// Extra seconds of held breath before this flip.
    pub hold: f32,
    /// 0..1 white flash strength.
    pub flash: f32,
    /// Screen-shake amplitude in px (decays).
    pub shake: f32,
    /// 0..1 halo behind the card.
    pub glow: f32,
    /// 0 = faded, 1 = Faint ... 7 = Resonant.
    pub tier: u8,
}

pub fn beat(r: &Recalled) -> Beat {
    use crate::cards::Rarity::*;
    if !r.remembered {
        // A soft dim beat; near misses hold their breath a little.
        let hold = if near_miss(r) { 0.35 } else { 0.03 };
        return Beat {
            hold,
            flash: 0.0,
            shake: 0.0,
            glow: 0.0,
            tier: 0,
        };
    }
    let (tier, hold, flash, shake, glow) = match r.card.rarity {
        Faint => (1, 0.0, 0.02, 0.0, 0.05),
        Hazy => (2, 0.05, 0.08, 0.5, 0.2),
        Vivid => (3, 0.12, 0.18, 1.5, 0.4),
        Lucid => (4, 0.25, 0.32, 3.0, 0.6),
        Prophetic => (5, 0.45, 0.55, 6.0, 0.8),
        Fused => (6, 0.6, 0.7, 8.0, 0.9),
        Resonant => (7, 0.75, 0.85, 10.0, 1.0),
    };
    Beat {
        hold,
        flash,
        shake,
        glow,
        tier,
    }
}

/// Start time of each display slot's flip, in seconds. Monotonic, never
/// negative, and the last flip starts within `LEAD_IN + REVEAL_BUDGET`:
/// holds are scaled down to whatever room the budget leaves.
pub fn reveal_times(pack: &[Recalled]) -> Vec<f32> {
    let n = pack.len();
    let order = reveal_order(pack);
    let step = reveal_step(n);
    let mut holds: Vec<f32> = order.iter().map(|&i| beat(&pack[i]).hold.max(0.0)).collect();
    if let Some(last) = holds.last_mut() {
        *last += FINAL_LINGER;
    }
    let total: f32 = holds.iter().sum();
    let room = (REVEAL_BUDGET - n.saturating_sub(1) as f32 * step).max(0.0);
    let k = if total > room { room / total } else { 1.0 };
    let mut t = LEAD_IN;
    holds
        .iter()
        .map(|h| {
            t += h * k;
            let start = t;
            t += step;
            start
        })
        .collect()
}

/// 0 = face down, 1 = flipping, 2 = revealed, for display slot `slot`.
pub fn reveal_stage_at(times: &[f32], slot: usize, t: f32) -> u8 {
    let start = times.get(slot).copied().unwrap_or(f32::MAX);
    if t < start {
        0
    } else if t < start + FLIP_TIME {
        1
    } else {
        2
    }
}

/// Like `reveal_done`, but honouring the holds of this pack.
pub fn reveal_done_pack(pack: &[Recalled], t: f32) -> bool {
    pack.is_empty() || reveal_stage_at(&reveal_times(pack), pack.len() - 1, t) == 2
}

/// Screen-wide effects at one instant.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fx {
    pub flash: f32,
    pub shake: Vec2,
}

/// Flash and shake summed over every flip so far. `motion` is the settings
/// scale; anything below 1 (reduced motion) turns both off.
pub fn effects(pack: &[Recalled], order: &[usize], times: &[f32], age: f32, motion: f32) -> Fx {
    if motion < 1.0 || age == f32::MAX {
        return Fx::default();
    }
    let (mut flash, mut sx, mut sy) = (0.0_f32, 0.0_f32, 0.0_f32);
    for (slot, &i) in order.iter().enumerate() {
        let e = age - (times[slot] + FLIP_TIME * 0.5);
        if !(0.0..2.0).contains(&e) {
            continue;
        }
        let b = beat(&pack[i]);
        flash = flash.max(b.flash * (-e * 7.0).exp());
        let a = b.shake * (-e * 5.0).exp();
        sx += a * (e * 61.0 + slot as f32).sin();
        sy += a * (e * 47.0 + 1.7 * slot as f32).cos();
    }
    Fx {
        flash: flash.clamp(0.0, 1.0),
        shake: Vec2::new(sx, sy),
    }
}

/// Which reveal stage card `i` of `n` is in at time `t`:
/// 0 = face down, 1 = flipping, 2 = revealed.
#[cfg(test)]
pub fn reveal_stage(i: usize, n: usize, t: f32) -> u8 {
    let start = 0.6 + i as f32 * reveal_step(n);
    if t < start {
        0
    } else if t < start + 0.35 {
        1
    } else {
        2
    }
}

#[cfg(test)]
pub fn reveal_done(n: usize, t: f32) -> bool {
    n == 0 || reveal_stage(n - 1, n, t) == 2
}

/// Where the cards may go: under the title, above the two message lines.
pub fn pack_area(screen: Vec2) -> Vec2 {
    Vec2::new(screen.x - 80.0, screen.y - 80.0 - 110.0)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PackLayout {
    pub size: Vec2,
    pub gap: f32,
    pub per_row: usize,
    /// Cards drawn; anything past this is summed up as "+N".
    pub shown: usize,
    /// Tiny rarity chips instead of cards.
    pub compact: bool,
}

const FULL_SCALE: f32 = 0.62;
/// Below this the card art stops being readable: switch to chips.
const MIN_SCALE: f32 = 0.34;
const CHIP: Vec2 = Vec2::new(58.0, 30.0);

/// The biggest cards that fit all `n` dreams; if even the smallest don't,
/// chips; if even chips don't, as many chips as fit (the last says "+N").
pub fn pack_layout(n: usize, area: Vec2) -> PackLayout {
    let n = n.max(1);
    let fits = |size: Vec2, gap: f32| {
        let per_row = (((area.x + gap) / (size.x + gap)).floor() as usize).max(1);
        let rows = ((area.y + gap) / (size.y + gap)).floor() as usize;
        (per_row, rows)
    };
    let mut scale = FULL_SCALE;
    while scale >= MIN_SCALE {
        let size = CARD_SIZE * scale;
        let gap = 14.0 * scale / FULL_SCALE;
        let (per_row, rows) = fits(size, gap);
        let per_row = if scale == FULL_SCALE {
            per_row.min(6)
        } else {
            per_row
        };
        if per_row * rows >= n {
            return PackLayout {
                size,
                gap,
                per_row,
                shown: n,
                compact: false,
            };
        }
        scale -= 0.02;
    }
    let gap = 6.0;
    let (per_row, rows) = fits(CHIP, gap);
    let room = per_row * rows.max(1);
    PackLayout {
        size: CHIP,
        gap,
        per_row,
        shown: n.min(room),
        compact: true,
    }
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
    // As many dreams as the run had: full cards, smaller cards, or chips.
    let n = pk.pack.len();
    let l = pack_layout(n, pack_area(screen.size()));
    let scale = l.size.x / CARD_SIZE.x;
    let overflow = n - l.shown;
    let order = reveal_order(&pk.pack);
    let times = reveal_times(&pk.pack);
    let motion = v.settings_snapshot.motion();
    let fx = effects(&pk.pack, &order, &times, pk.age, motion);
    for (slot, &pi) in order.iter().enumerate().take(l.shown) {
        let r = &pk.pack[pi];
        let b = beat(r);
        let row = slot / l.per_row;
        let col = slot % l.per_row;
        let in_row = (l.shown - row * l.per_row).min(l.per_row) as f32;
        let row_w = in_row * l.size.x + (in_row - 1.0) * l.gap;
        let min = Pos2::new(
            cx - row_w * 0.5 + col as f32 * (l.size.x + l.gap),
            screen.top() + 80.0 + row as f32 * (l.size.y + l.gap),
        ) + fx.shake;
        let rect = Rect::from_min_size(min, l.size);
        // The last chip stands for everything that didn't fit.
        if l.compact && overflow > 0 && slot + 1 == l.shown {
            p.rect_stroke(rect, 0.0, Stroke::new(2.0_f32, rgba(ink(v), 0.7)));
            p.text(
                rect.center(),
                Align2::CENTER_CENTER,
                format!("+{}", overflow + 1),
                FontId::monospace(20.0),
                rgba(ink(v), 0.9),
            );
            continue;
        }
        let start = times[slot];
        let since = pk.age - start - FLIP_TIME; // seconds since fully revealed
        let rgb = crate::booklet_ui::card_frame(&r.card, v.time);
        let stage = reveal_stage_at(&times, slot, pk.age);
        // Halo behind remembered high-rarity cards, strongest at the reveal.
        if stage >= 1 && b.glow > 0.0 && !l.compact {
            let pulse = if stage == 2 && pk.age != f32::MAX {
                0.6 + 0.4 * (-since.max(0.0) * 1.5).exp()
            } else {
                0.6
            };
            for ring in 1..=4 {
                let grow = ring as f32 * 5.0 * (0.5 + b.glow);
                p.rect_filled(
                    rect.expand(grow),
                    4.0 * ring as f32,
                    rgba(rgb, (b.glow * pulse * 0.16 / ring as f32).min(0.3)),
                );
            }
        }
        match stage {
            0 => card_back(p, rect, v, false),
            1 => {
                // Flip: squash horizontally around the centre.
                let t = ((pk.age - start) / FLIP_TIME).clamp(0.0, 1.0);
                let w = (1.0 - 2.0 * t).abs();
                let squashed = Rect::from_center_size(
                    rect.center(),
                    Vec2::new(l.size.x * w.max(0.04), l.size.y),
                );
                if t < 0.5 {
                    card_back(p, squashed, v, true);
                } else {
                    p.rect_filled(squashed, 0.0, rgba(rgb, 0.9));
                }
            }
            _ if l.compact => chip(p, rect, r, v),
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
                } else if near_miss(r) && since < NEAR_MISS_TIME {
                    near_miss_flicker(p, rect, r, v, since.max(0.0));
                } else {
                    faded(p, rect, r, v);
                }
            }
        }
        // Rarity-coloured burst of pixels as the card lands.
        if motion >= 1.0 && b.tier > 0 && !l.compact && (0.0..0.7).contains(&since) {
            let count = 6 + b.tier as u32 * 6;
            let reach = l.size.x * (0.3 + 0.1 * b.tier as f32) * (since / 0.7).sqrt();
            let alpha = 1.0 - since / 0.7;
            for k in 0..count {
                let ang = crate::hud::hash01(k, r.card.seed as u32) * std::f32::consts::TAU;
                let spd = 0.5 + crate::hud::hash01(k, 0x51ED) * 0.5;
                let at = rect.center() + Vec2::new(ang.cos(), ang.sin()) * reach * spd;
                p.rect_filled(
                    Rect::from_center_size(at, Vec2::splat(3.0 + b.tier as f32 * 0.5)),
                    0.0,
                    rgba(rgb, alpha),
                );
            }
        }
    }
    if fx.flash > 0.0 {
        p.rect_filled(screen, 0.0, rgba([255, 255, 255], fx.flash * 0.6));
    }
    let done = reveal_done_pack(&pk.pack, pk.age);
    let kept = pk.pack.iter().filter(|r| r.remembered).count();
    let msg = if !done {
        v.k("remembering...    [space] skip")
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
    if done && pk.pressed {
        if !v.report_line.is_empty() {
            p.text(
                Pos2::new(cx, screen.bottom() - 148.0),
                Align2::CENTER_TOP,
                &v.report_line,
                FontId::monospace(20.0),
                rgba([140, 255, 220], 0.9),
            );
        }
        if !v.goal.is_empty() {
            p.text(
                Pos2::new(cx, screen.bottom() - 120.0),
                Align2::CENTER_TOP,
                &v.goal,
                FontId::monospace(20.0),
                rgba(hue(v.time * 0.15), 0.9),
            );
        }
    }
    if done {
        let keys = if pk.pressed {
            "[b] booklet   [l] lucid store   [r] dream again   [esc] wake"
        } else {
            "[s] press into booklet   [r] dream again   [esc] wake"
        };
        let keys = v.k(keys);
        p.text(
            Pos2::new(cx, screen.bottom() - 54.0),
            Align2::CENTER_TOP,
            keys,
            FontId::monospace(22.0),
            rgba(ink(v), 0.5 + 0.3 * (v.time * 2.0).sin()),
        );
    }
}

/// "SO CLOSE...": the card face stutters on, then dissolves to dust.
fn near_miss_flicker(p: &egui::Painter, r: Rect, rec: &Recalled, v: &HudView, e: f32) {
    let frac = (e / NEAR_MISS_TIME).clamp(0.0, 1.0);
    let frame = crate::booklet_ui::card_frame(&rec.card, v.time);
    // Face on more often early, sputtering out as it dissolves.
    let tick = (e * 18.0) as u32;
    let on = crate::hud::hash01(tick, rec.card.seed as u32) > frac * 0.9;
    if on {
        p.rect_filled(r, 0.0, rgba(frame, 0.85 * (1.0 - frac * 0.5)));
        p.rect_stroke(r.shrink(3.0), 0.0, Stroke::new(2.0_f32, rgba([255, 255, 255], 0.8)));
        let f = (r.width() / (CARD_SIZE.x * 0.62)).min(1.0);
        p.text(
            r.center(),
            Align2::CENTER_CENTER,
            rec.card.name.clone(),
            FontId::monospace(15.0 * f),
            rgba([10, 5, 20], 0.9),
        );
    } else {
        p.rect_filled(r, 0.0, rgba([14, 10, 22], 0.9));
    }
    // Dust lifting off the face.
    for k in 0..(10.0 + 50.0 * frac) as u32 {
        let h = crate::hud::hash01(k, rec.card.seed as u32 ^ 0x77);
        let h2 = crate::hud::hash01(k, 0xBEEF);
        let at = Pos2::new(
            r.left() + h * r.width(),
            r.bottom() - h2 * r.height() - frac * 40.0 * (0.5 + h),
        );
        p.rect_filled(
            Rect::from_center_size(at, Vec2::splat(3.0)),
            0.0,
            rgba(frame, 0.7 * (1.0 - frac)),
        );
    }
    let f = (r.width() / (CARD_SIZE.x * 0.62)).min(1.0);
    p.text(
        Pos2::new(r.center().x, r.bottom() - 18.0 * f),
        Align2::CENTER_CENTER,
        "SO CLOSE...",
        FontId::monospace(20.0 * f),
        rgba([255, 255, 255], if on { 1.0 } else { 0.6 }),
    );
}

/// A dream too many to show as a card: its rarity colour and depth.
/// Forgotten ones are hollow and grey.
fn chip(p: &egui::Painter, r: Rect, rec: &Recalled, v: &HudView) {
    let rgb = crate::booklet_ui::card_frame(&rec.card, v.time);
    if rec.remembered {
        p.rect_filled(r, 0.0, rgba(rgb, 0.85));
    } else {
        p.rect_stroke(r, 0.0, Stroke::new(2.0_f32, rgba([110, 100, 130], 0.6)));
    }
    p.text(
        r.center(),
        Align2::CENTER_CENTER,
        format!("{}", rec.card.depth),
        FontId::monospace(18.0),
        rgba(
            if rec.remembered {
                [10, 5, 20]
            } else {
                [150, 140, 170]
            },
            1.0,
        ),
    );
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
    // Text scales with the card (long runs shrink it).
    let f = (r.width() / (CARD_SIZE.x * 0.62)).min(1.0);
    p.rect_filled(r, 0.0, rgba([14, 10, 22], 0.9));
    p.rect_stroke(r.shrink(3.0), 0.0, Stroke::new(1.0_f32, rgba(ink(v), 0.15)));
    // Dissolving pixel dust.
    for k in 0..40u32 {
        let h = crate::hud::hash01(k, rec.card.seed as u32);
        let h2 = crate::hud::hash01(k, (rec.card.seed >> 32) as u32 ^ 0xA5A5);
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
        FontId::monospace(15.0 * f),
        rgba(ink(v), 0.3),
        r.width() - 16.0 * f,
    );
    p.galley(
        r.left_top() + Vec2::new(8.0 * f, 24.0 * f),
        name,
        rgba(ink(v), 0.3),
    );
    p.text(
        r.center() + Vec2::new(0.0, 0.18 * r.height()),
        Align2::CENTER_CENTER,
        "FADED",
        FontId::monospace(22.0 * f),
        rgba(ink(v), 0.35),
    );
    p.text(
        r.center() + Vec2::new(0.0, 0.18 * r.height() + 26.0 * f),
        Align2::CENTER_CENTER,
        format!("+{} dust", crate::cards::fade_dust(rec.card.rarity)),
        FontId::monospace(16.0 * f),
        rgba([255, 210, 80], 0.7),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards_reveal_in_order_and_the_pack_finishes() {
        assert_eq!(reveal_stage(0, 3, 0.0), 0);
        assert_eq!(reveal_stage(0, 3, 0.7), 1);
        assert_eq!(reveal_stage(0, 3, 1.2), 2);
        assert_eq!(reveal_stage(1, 3, 1.2), 0, "second card waits its turn");
        assert!(!reveal_done(3, 1.0));
        assert!(reveal_done(3, 0.6 + 2.0 * REVEAL_STEP + 0.36));
        assert!(reveal_done(0, 0.0));
        assert!(reveal_done(5, f32::MAX), "skip reveals everything");
    }

    #[test]
    fn any_run_length_fits_on_screen() {
        for screen in [Vec2::new(1280.0, 720.0), Vec2::new(1920.0, 1080.0)] {
            let area = pack_area(screen);
            for n in 1..=300usize {
                let l = pack_layout(n, area);
                let (w, h) = (l.size.x + l.gap, l.size.y + l.gap);
                let rows = n.min(l.shown).div_ceil(l.per_row);
                assert!(
                    l.per_row as f32 * w - l.gap <= area.x + 0.5,
                    "n={n} too wide"
                );
                assert!(rows as f32 * h - l.gap <= area.y + 0.5, "n={n} too tall");
                assert!(l.shown >= n.min(l.per_row), "shows at least a row");
                if n <= 6 {
                    assert!(
                        !l.compact && l.size == CARD_SIZE * 0.62,
                        "short runs look as before"
                    );
                }
            }
            assert!(
                pack_layout(30, area).size.x < CARD_SIZE.x * 0.62,
                "30 dreams shrink the cards"
            );
            assert!(pack_layout(300, area).compact, "300 dreams become chips");
        }
    }

    #[test]
    fn long_packs_reveal_faster_but_in_order() {
        assert_eq!(reveal_step(3), REVEAL_STEP, "short packs keep their pace");
        let total = |n: usize| 0.6 + reveal_step(n) * n as f32;
        for n in [10, 40, 150] {
            assert!(
                total(n) <= REVEAL_BUDGET + 0.7,
                "{n} cards take {}",
                total(n)
            );
        }
        assert!(reveal_stage(0, 40, 0.7) >= 1);
        assert_eq!(
            reveal_stage(39, 40, 0.7),
            0,
            "last card still waits its turn"
        );
        assert!(reveal_done(40, 0.6 + REVEAL_BUDGET + 0.4));
    }

    fn rec(rarity: crate::cards::Rarity, remembered: bool, seed: u64) -> Recalled {
        let mut card = crate::cards::card_from(&crate::cards::tests::record(
            crate::dream::DreamTheme::Lobby,
            seed,
            1,
            0.1,
            false,
        ));
        card.rarity = rarity;
        Recalled { card, remembered }
    }

    use crate::cards::Rarity::*;
    const ALL: [crate::cards::Rarity; 7] = [Faint, Hazy, Vivid, Lucid, Prophetic, Fused, Resonant];

    fn mixed(n: usize) -> Vec<Recalled> {
        (0..n)
            .map(|i| rec(ALL[(i * 5 + i / 3) % 7], i % 4 != 0, i as u64 + 1))
            .collect()
    }

    #[test]
    fn order_puts_best_last_and_is_a_permutation() {
        let pack = vec![
            rec(Resonant, true, 1),
            rec(Faint, true, 2),
            rec(Resonant, false, 3),
            rec(Fused, true, 4),
            rec(Prophetic, true, 5),
            rec(Hazy, true, 6),
        ];
        let o = reveal_order(&pack);
        assert_eq!(o, vec![2, 1, 5, 4, 3, 0]);
        let mut sorted = o.clone();
        sorted.sort();
        assert_eq!(sorted, (0..6).collect::<Vec<_>>());
        assert!(pack[*o.last().unwrap()].remembered);
    }

    #[test]
    fn order_is_stable_and_handles_tiny_packs() {
        let pack = vec![rec(Vivid, true, 1), rec(Vivid, true, 2), rec(Vivid, true, 3)];
        assert_eq!(reveal_order(&pack), vec![0, 1, 2]);
        assert!(reveal_order(&[]).is_empty());
        assert_eq!(reveal_order(&pack[..1]), vec![0]);
        assert!(reveal_times(&[]).is_empty());
        assert_eq!(reveal_times(&pack[..1]).len(), 1);
        assert!(reveal_done_pack(&[], 0.0));
    }

    #[test]
    fn times_are_monotonic_non_negative_and_within_budget() {
        for n in [1, 5, 20, 100] {
            let pack = mixed(n);
            let t = reveal_times(&pack);
            assert_eq!(t.len(), n);
            assert!(t[0] >= 0.0);
            assert!(t.windows(2).all(|w| w[1] > w[0]), "n={n} not increasing");
            assert!(
                *t.last().unwrap() <= LEAD_IN + REVEAL_BUDGET + 1e-3,
                "n={n} ends at {}",
                t.last().unwrap()
            );
            assert!(!reveal_done_pack(&pack, t[n - 1] + 0.1));
            assert!(reveal_done_pack(&pack, t[n - 1] + FLIP_TIME + 0.01));
            assert!(reveal_done_pack(&pack, f32::MAX));
        }
    }

    #[test]
    fn the_best_card_gets_the_longest_hold() {
        for n in [1, 5, 20, 100] {
            let pack = mixed(n);
            let t = reveal_times(&pack);
            let step = reveal_step(n);
            // Gap before each slot beyond the normal step; slot 0 measured from lead-in.
            let hold = |k: usize| {
                let prev = if k == 0 { LEAD_IN - step } else { t[k - 1] };
                t[k] - prev - step
            };
            let last = hold(n - 1);
            assert!(last > 0.0);
            for k in 0..n - 1 {
                assert!(last >= hold(k) - 1e-4, "n={n} slot {k}");
            }
        }
    }

    #[test]
    fn beats_grow_with_rarity() {
        let b: Vec<Beat> = ALL.iter().map(|&r| beat(&rec(r, true, 1))).collect();
        for w in b.windows(2) {
            assert!(w[1].tier > w[0].tier);
            assert!(w[1].hold >= w[0].hold && w[1].flash > w[0].flash);
            assert!(w[1].shake >= w[0].shake && w[1].glow > w[0].glow);
        }
        assert!(b.iter().all(|x| x.flash <= 1.0 && x.glow <= 1.0));
        let dim = beat(&rec(Prophetic, false, 1));
        assert_eq!((dim.tier, dim.flash, dim.shake, dim.glow), (0, 0.0, 0.0, 0.0));
        assert!(dim.flash < b[0].flash);
    }

    #[test]
    fn near_miss_is_a_faded_vivid_or_better() {
        for &r in &ALL {
            assert!(!near_miss(&rec(r, true, 1)), "remembered is never a miss");
            assert_eq!(near_miss(&rec(r, false, 1)), r >= Vivid, "{r:?}");
        }
    }

    #[test]
    fn reduced_motion_kills_shake_and_flash() {
        let pack = vec![rec(Resonant, true, 1)];
        let (o, t) = (reveal_order(&pack), reveal_times(&pack));
        let age = t[0] + FLIP_TIME * 0.5 + 0.05;
        let full = effects(&pack, &o, &t, age, 1.0);
        assert!(full.flash > 0.3 && full.shake != Vec2::ZERO);
        assert_eq!(effects(&pack, &o, &t, age, 0.25), Fx::default());
        assert_eq!(effects(&pack, &o, &t, f32::MAX, 1.0), Fx::default());
    }
}
