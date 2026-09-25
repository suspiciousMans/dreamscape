//! The Lucid Store, drawn as a little pixel shop: a shopkeeper (a sleepy
//! moth-eyed thing behind the counter) on the left, five shelves of pixel
//! icons in the middle, and a preview panel on the right that shows what the
//! selected item actually looks like on you. Same VT323 + glitch language.

use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use crate::pixels::icons;
use crate::store::{
    CardStyle, Crystal, EyeStyle, HudPalette, Item, Perk, Shelf, State, MAX_CHARGES, PERK_RUNS,
};
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect, Stroke, Vec2};

/// Selection is (shelf row, item column).
pub fn cursor_item(shelf: usize, col: usize) -> Item {
    let items = Shelf::ALL[shelf.min(Shelf::ALL.len() - 1)].items();
    items[col.min(items.len() - 1)]
}

/// Moving between shelves keeps the column if it can.
pub fn clamp_col(shelf: usize, col: usize) -> usize {
    col.min(Shelf::ALL[shelf].items().len() - 1)
}

fn icon_for(item: Item) -> &'static [&'static str; icons::SIZE] {
    match item {
        Item::Perk(Perk::FirstLight) => &icons::SHARD,
        Item::Perk(Perk::SlowHeart) => &icons::HEART,
        Item::Perk(Perk::DeepMemory) => &icons::BRAIN,
        Item::Perk(Perk::HeavyEyelids) => &icons::LID,
        Item::Perk(Perk::SecondWind) => &icons::WIND,
        Item::Perk(Perk::DustMagnet) => &icons::MAGNET,
        Item::Perk(Perk::LongStride) => &icons::BOOT,
        Item::Eye(_) => &icons::EYE,
        Item::Crystal(_) => &icons::GEM,
        Item::Card(_) => &icons::CARD,
        Item::Hud(_) => &icons::INK,
    }
}

/// The two colours an item's icon is painted in.
fn icon_colors(item: Item, v: &HudView) -> ([u8; 3], [u8; 3]) {
    let t = v.time;
    match item {
        Item::Perk(_) => (ink(v), [80, 230, 200]),
        Item::Eye(e) => (ink(v), crate::hud::eye_iris(e, t)),
        Item::Crystal(Crystal::Prism) => (ink(v), hue(t * 0.4)),
        Item::Crystal(c) => {
            let [r, g, b, _] = c.rgba();
            (ink(v), [r, g, b])
        }
        Item::Card(CardStyle::Plain) => (ink(v), [150, 150, 170]),
        Item::Card(CardStyle::Gilt) => ([255, 205, 90], [255, 235, 160]),
        Item::Card(CardStyle::Holo) => (hue(t * 0.3), hue(t * 0.3 + 0.5)),
        Item::Hud(h) => {
            let c = crate::hud::palette_ink(h);
            (c, c)
        }
    }
}

fn draw_icon(p: &egui::Painter, c: Pos2, px: f32, item: Item, v: &HudView, alpha: f32) {
    let (main, accent) = icon_colors(item, v);
    let o = Pos2::new(c.x.round(), c.y.round());
    for (x, y, acc) in icons::cells(icon_for(item)) {
        let at = o + Vec2::new(x as f32 * px, y as f32 * px);
        p.rect_filled(
            Rect::from_center_size(at + Vec2::splat(px * 0.25), Vec2::splat(px)),
            0.0,
            rgba([0, 0, 0], 0.4 * alpha),
        );
        p.rect_filled(
            Rect::from_center_size(at, Vec2::splat(px)),
            0.0,
            rgba(if acc { accent } else { main }, alpha),
        );
    }
}

/// The shopkeeper: a hunched pixel moth with one huge sleepy eye that
/// follows your cursor, and a speech line under it.
fn shopkeeper(p: &egui::Painter, at: Pos2, v: &HudView, look_x: f32) {
    const BODY: [&str; 14] = [
        "....##......##....",
        ".....#......#.....",
        "......######......",
        ".##..#++++++#..##.",
        "#++#.#+....+#.#++#",
        "#+++##+....+##+++#",
        "#++++#+....+#++++#",
        ".#+++##++++##+++#.",
        "..###.#++++#.###..",
        "......#+##+#......",
        ".....#++##++#.....",
        "....#+++##+++#....",
        "...############...",
        "..##############..",
    ];
    let px = 6.0;
    let bob = (v.time * 1.3).sin() * 2.0;
    let o = at + Vec2::new(-9.0 * px, bob);
    let wing = hue(0.78 + 0.05 * (v.time * 0.5).sin());
    for (y, row) in BODY.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            let col = match ch {
                '#' => ink(v),
                '+' => wing,
                _ => continue,
            };
            p.rect_filled(
                Rect::from_min_size(o + Vec2::new(x as f32 * px, y as f32 * px), Vec2::splat(px)),
                0.0,
                rgba(col, 0.95),
            );
        }
    }
    // The eye: a 4x4 socket inside the head, with a pupil that follows.
    let eye_c = o + Vec2::new(9.0 * px, 5.5 * px);
    let open = if (v.time % 5.3) < 0.15 { 0.2 } else { 1.0 };
    p.rect_filled(
        Rect::from_center_size(eye_c, Vec2::new(4.0 * px, 3.0 * px * open)),
        0.0,
        rgba([30, 18, 52], 1.0),
    );
    if open > 0.5 {
        let dx = (look_x * 1.0).clamp(-1.0, 1.0).round() * px;
        p.rect_filled(
            Rect::from_center_size(eye_c + Vec2::new(dx, 0.0), Vec2::splat(px * 1.4)),
            0.0,
            rgba([255, 210, 80], 1.0),
        );
    }
    // Counter.
    let counter = Rect::from_min_size(
        o + Vec2::new(-2.0 * px, 14.0 * px),
        Vec2::new(22.0 * px, 3.0 * px),
    );
    p.rect_filled(counter, 0.0, rgba([40, 22, 60], 1.0));
    p.rect_stroke(counter, 0.0, Stroke::new(2.0_f32, rgba(ink(v), 0.6)));
}

/// What the moth says about the selected item.
pub fn patter(item: Item, state: State, dust: u32) -> &'static str {
    match state {
        State::Equipped => "you're already wearing it. it suits you.",
        State::Owned => "yours already. [enter] to put it on.",
        State::Stocked { runs, .. } if runs + PERK_RUNS > MAX_CHARGES => {
            "you can't carry any more of that."
        }
        State::Buy(price) | State::Stocked { price, .. } if price > dust => {
            "come back with more dust. i'll wait. i always wait."
        }
        _ => match item {
            Item::Perk(Perk::FirstLight) => "a little light to start with. lasts three nights.",
            Item::Perk(Perk::SlowHeart) => "slow your heart and they slow too.",
            Item::Perk(Perk::DeepMemory) => "hold on to more of what you dream.",
            Item::Perk(Perk::HeavyEyelids) => "close your eyes. they can't see you either.",
            Item::Perk(Perk::SecondWind) => "the dream forgives you. once a night.",
            Item::Perk(Perk::DustMagnet) => "dust sticks to you. lovely.",
            Item::Perk(Perk::LongStride) => "longer legs. it's only a dream.",
            Item::Eye(_) => "a new way of seeing things.",
            Item::Crystal(_) => "a different shape to be.",
            Item::Card(_) => "frames for the pages of you.",
            Item::Hud(_) => "new ink for old thoughts.",
        },
    }
}

fn price_tag(state: State, dust: u32) -> (String, [u8; 3]) {
    match state {
        State::Buy(price) if price <= dust => (format!("{price}"), [255, 210, 80]),
        State::Buy(price) => (format!("{price}"), [120, 100, 90]),
        State::Stocked { runs, price } if runs + PERK_RUNS <= MAX_CHARGES && price <= dust => {
            (format!("x{runs} +{price}"), [80, 230, 200])
        }
        State::Stocked { runs, .. } => (format!("x{runs}"), [80, 230, 200]),
        State::Owned => ("owned".into(), [170, 160, 200]),
        State::Equipped => ("ON".into(), [120, 255, 140]),
    }
}

/// The preview panel: the selected cosmetic applied, or the perk's effect.
fn preview(p: &egui::Painter, r: Rect, item: Item, v: &HudView) {
    p.rect_filled(r, 0.0, rgba([14, 8, 28], 1.0));
    p.rect_stroke(r, 0.0, Stroke::new(2.0_f32, rgba(ink(v), 0.4)));
    let c = r.center() - Vec2::new(0.0, 20.0);
    match item {
        Item::Eye(e) => {
            // The real HUD eye, wide open, in this style.
            crate::hud::draw_eye_preview(p, c, e, v);
        }
        Item::Crystal(cr) => {
            let col = match cr {
                Crystal::Prism => hue(v.time * 0.4),
                _ => {
                    let [r, g, b, _] = cr.rgba();
                    [r, g, b]
                }
            };
            // A spinning pixel octahedron: a diamond whose width breathes.
            let w = (v.time * 2.0).cos().abs() * 5.0 + 1.0;
            for y in -7i32..=7 {
                let half = ((7 - y.abs()) as f32 * w / 7.0).round() as i32;
                for x in -half..=half {
                    let shade = if x < 0 { 0.75 } else { 1.0 };
                    let cc = [
                        (col[0] as f32 * shade) as u8,
                        (col[1] as f32 * shade) as u8,
                        (col[2] as f32 * shade) as u8,
                    ];
                    p.rect_filled(
                        Rect::from_center_size(
                            c + Vec2::new(x as f32 * 6.0, y as f32 * 6.0),
                            Vec2::splat(6.0),
                        ),
                        0.0,
                        rgba(cc, 1.0),
                    );
                }
            }
        }
        Item::Card(style) => {
            let frame = match style {
                CardStyle::Plain => [255, 80, 200],
                CardStyle::Gilt => [255, 205, 90],
                CardStyle::Holo => hue(v.time * 0.3),
            };
            let card = Rect::from_center_size(c, Vec2::new(96.0, 140.0));
            p.rect_filled(card, 0.0, rgba([16, 9, 30], 1.0));
            p.rect_stroke(
                card.shrink(2.0),
                0.0,
                Stroke::new(4.0_f32, rgba(frame, 1.0)),
            );
            p.rect_stroke(
                card.shrink(8.0),
                0.0,
                Stroke::new(1.0_f32, rgba(frame, 0.45)),
            );
            let art = Rect::from_min_size(card.min + Vec2::new(10.0, 30.0), Vec2::new(76.0, 50.0));
            p.rect_filled(art, 0.0, rgba(hue(v.time * 0.1), 0.6));
            p.text(
                card.min + Vec2::new(10.0, 10.0),
                Align2::LEFT_TOP,
                "No.001",
                FontId::monospace(14.0),
                rgba(ink(v), 0.6),
            );
        }
        Item::Hud(h) => {
            let col = crate::hud::palette_ink(h);
            let (gl, gr) = crate::hud::palette_ghosts(h);
            for (off, cc, a) in [(-2.0, gl, 0.6), (2.0, gr, 0.6), (0.0, col, 1.0)] {
                p.text(
                    c + Vec2::new(off, 0.0),
                    Align2::CENTER_CENTER,
                    "DEPTH 07",
                    FontId::monospace(40.0),
                    rgba(cc, a),
                );
            }
        }
        Item::Perk(_) => {
            draw_icon(p, c, 12.0, item, v, 1.0);
        }
    }
}

pub fn draw(p: &egui::Painter, screen: Rect, v: &HudView) {
    p.rect_filled(screen, 0.0, rgba([6, 3, 16], 0.96));
    // Drifting dust motes in the shop air.
    for k in 0..60u32 {
        let h1 = (k.wrapping_mul(2_654_435_761)) as f32 / u32::MAX as f32;
        let h2 = (k.wrapping_mul(40_503).wrapping_add(7)) as f32 / u32::MAX as f32;
        let x = screen.left() + h1 * screen.width();
        let y = screen.bottom()
            - ((h2 * screen.height() + v.time * (8.0 + 10.0 * h1)) % screen.height());
        p.rect_filled(
            Rect::from_center_size(Pos2::new(x, y), Vec2::splat(2.0 + 2.0 * h2)),
            0.0,
            rgba([255, 210, 80], 0.15 + 0.2 * h2),
        );
    }
    let cx = screen.center().x;
    glitch_text(
        p,
        Pos2::new(cx, screen.top() + 14.0),
        true,
        "THE LUCID STORE",
        42.0,
        hue(0.12 + 0.03 * (v.time * 0.7).sin()),
        1.0,
        v,
    );
    p.text(
        Pos2::new(cx, screen.top() + 62.0),
        Align2::CENTER_TOP,
        format!(
            "{} dream dust   ·   {} earned in all",
            v.dust, v.dust_earned
        ),
        FontId::monospace(22.0),
        rgba([255, 210, 80], 1.0),
    );

    let selected = cursor_item(v.shop_shelf, v.shop_col);
    let sel_state = state_of(v, selected);

    // Shopkeeper, left.
    let keeper_at = Pos2::new(screen.left() + 150.0, screen.top() + 150.0);
    let look = (v.shop_col as f32 - 2.0) / 3.0;
    shopkeeper(p, keeper_at, v, look);
    let say = patter(selected, sel_state, v.dust);
    let bubble = p.layout(
        say.to_string(),
        FontId::monospace(18.0),
        rgba(ink(v), 0.9),
        220.0,
    );
    p.galley(
        Pos2::new(screen.left() + 40.0, screen.top() + 290.0),
        bubble,
        rgba(ink(v), 0.9),
    );

    // Shelves, middle.
    let left = screen.left() + 300.0;
    let slot = 64.0;
    let mut y = screen.top() + 104.0;
    for (si, shelf) in Shelf::ALL.iter().enumerate() {
        p.text(
            Pos2::new(left, y),
            Align2::LEFT_TOP,
            shelf.label(),
            FontId::monospace(16.0),
            rgba(ink(v), if si == v.shop_shelf { 0.9 } else { 0.4 }),
        );
        let row_y = y + 20.0;
        // Wooden shelf plank.
        p.rect_filled(
            Rect::from_min_size(
                Pos2::new(left - 6.0, row_y + slot - 8.0),
                Vec2::new(7.0 * slot + 12.0, 6.0),
            ),
            0.0,
            rgba([70, 40, 60], 1.0),
        );
        for (ci, item) in shelf.items().into_iter().enumerate() {
            let c = Pos2::new(left + ci as f32 * slot + slot * 0.5, row_y + slot * 0.45);
            let is_sel = si == v.shop_shelf && ci == v.shop_col;
            let state = state_of(v, item);
            let dim = match state {
                State::Buy(price) if price > v.dust => 0.35,
                _ => 1.0,
            };
            if is_sel {
                let r = Rect::from_center_size(c, Vec2::splat(slot - 6.0));
                p.rect_filled(r, 0.0, rgba(hue(v.time * 0.2), 0.18));
                p.rect_stroke(r, 0.0, Stroke::new(2.0_f32, rgba(hue(v.time * 0.2), 0.9)));
            }
            let lift = if is_sel {
                (v.time * 4.0).sin() * 2.0 - 2.0
            } else {
                0.0
            };
            draw_icon(p, c + Vec2::new(0.0, lift), 4.0, item, v, dim);
            let (tag, col) = price_tag(state, v.dust);
            p.text(
                c + Vec2::new(0.0, 22.0),
                Align2::CENTER_TOP,
                tag,
                FontId::monospace(13.0),
                rgba(col, dim.max(0.6)),
            );
        }
        y += slot + 26.0;
    }

    // Preview + details, right.
    let panel = Rect::from_min_size(
        Pos2::new(screen.right() - 330.0, screen.top() + 104.0),
        Vec2::new(300.0, 250.0),
    );
    preview(p, panel, selected, v);
    let info = selected.info();
    p.text(
        panel.left_bottom() + Vec2::new(0.0, 14.0),
        Align2::LEFT_TOP,
        info.name,
        FontId::monospace(26.0),
        rgba(ink(v), 1.0),
    );
    p.text(
        panel.left_bottom() + Vec2::new(0.0, 46.0),
        Align2::LEFT_TOP,
        info.blurb,
        FontId::proportional(19.0),
        rgba([200, 190, 230], 0.85),
    );
    let action = match sel_state {
        State::Buy(price) => format!("[enter] buy for {price} dust"),
        State::Stocked { runs, price } if runs + PERK_RUNS <= MAX_CHARGES => {
            format!("{runs} runs left · [enter] +{PERK_RUNS} for {price}")
        }
        State::Stocked { runs, .. } => format!("{runs} runs left · full"),
        State::Owned => "[enter] equip".into(),
        State::Equipped => "equipped".into(),
    };
    p.text(
        panel.left_bottom() + Vec2::new(0.0, 76.0),
        Align2::LEFT_TOP,
        action,
        FontId::monospace(20.0),
        rgba([255, 210, 80], 0.95),
    );
    if matches!(selected, Item::Perk(_)) {
        p.text(
            panel.left_bottom() + Vec2::new(0.0, 104.0),
            Align2::LEFT_TOP,
            format!("each purchase lasts {PERK_RUNS} runs"),
            FontId::monospace(16.0),
            rgba(ink(v), 0.5),
        );
    }

    if let Some(msg) = &v.store_status {
        p.text(
            Pos2::new(cx, screen.bottom() - 80.0),
            Align2::CENTER_TOP,
            msg,
            FontId::monospace(22.0),
            rgba([255, 210, 80], 0.95),
        );
    }
    p.text(
        Pos2::new(cx, screen.bottom() - 44.0),
        Align2::CENTER_TOP,
        "[w/s] shelf   [a/d] item   [enter] buy / equip   [esc] back",
        FontId::monospace(20.0),
        rgba(ink(v), 0.5 + 0.3 * (v.time * 2.0).sin()),
    );
}

fn state_of(v: &HudView, item: Item) -> State {
    let i = crate::store::CATALOG
        .iter()
        .position(|&c| c == item)
        .expect("shelf items come from the catalog");
    v.store_states[i]
}

#[allow(dead_code)]
fn _exhaustive(e: EyeStyle, h: HudPalette) -> (EyeStyle, HudPalette) {
    (e, h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::CATALOG;

    #[test]
    fn every_catalog_item_is_reachable_from_the_shelves() {
        let mut seen = Vec::new();
        for (s, shelf) in Shelf::ALL.iter().enumerate() {
            for c in 0..shelf.items().len() {
                seen.push(cursor_item(s, c));
            }
        }
        assert_eq!(seen, CATALOG.to_vec());
    }

    #[test]
    fn cursor_is_clamped_to_real_slots() {
        for s in 0..Shelf::ALL.len() {
            let last = Shelf::ALL[s].items().len() - 1;
            assert_eq!(clamp_col(s, 99), last);
            assert_eq!(cursor_item(s, 99), Shelf::ALL[s].items()[last]);
        }
    }

    #[test]
    fn the_moth_has_a_line_for_everything() {
        for item in CATALOG {
            for state in [
                State::Buy(10),
                State::Buy(10_000),
                State::Stocked { runs: 3, price: 10 },
                State::Stocked { runs: 9, price: 10 },
                State::Owned,
                State::Equipped,
            ] {
                let line = patter(item, state, 100);
                assert!(!line.is_empty() && line.len() <= 60, "{line}");
                assert_eq!(line, line.to_lowercase());
            }
        }
    }
}
