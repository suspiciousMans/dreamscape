//! The loadout screen, before a run: pick which booklet cards come along.
//! Each card brings its ability (the first ones fill the ability slots),
//! its passive, and its dream, which turns up once in the run.

use crate::booklet_ui::{card, card_frame, CardArt, CARD_SIZE};
use crate::cards::Card;
use crate::hud::{glitch_text, ink, rgba, HudView};
use crate::pixel_ui;
use crate::pixels::icons;
use crate::powers::power;
use crate::upgrades::{ABILITY_SLOTS, SLOT_KEYS};
use engine::ui::egui::{self, Align2, Color32, FontId, Pos2, Rect, Vec2};

/// Cards per row in the grid.
pub const COLS: usize = 6;
/// Rows of the grid shown at once (it scrolls).
pub const ROWS: usize = 5;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoadoutView {
    /// Every card in the booklet, in order.
    pub cards: Vec<Card>,
    /// Booklet numbers chosen, in order.
    pub chosen: Vec<u32>,
    pub slots: usize,
    pub selected: usize,
    /// The companion slot: `None` until a dreamer is collected, then the
    /// line to show ("[c] Ada: + CALM MIND x2 · ...", or "[c] nobody").
    pub companion: Option<String>,
}

/// Adds the card, or takes it out if it's already in. A full loadout
/// refuses new cards. Returns whether anything changed.
pub fn toggle(loadout: &mut Vec<u32>, number: u32, slots: usize) -> bool {
    if let Some(i) = loadout.iter().position(|&n| n == number) {
        loadout.remove(i);
        true
    } else if loadout.len() < slots {
        loadout.push(number);
        true
    } else {
        false
    }
}

/// Moves the grid cursor; stays inside `0..n`.
pub fn step(selected: usize, n: usize, dx: i32, dy: i32) -> usize {
    if n == 0 {
        return 0;
    }
    let i = selected as i32 + dx + dy * COLS as i32;
    i.clamp(0, n as i32 - 1) as usize
}

/// What a card does, for the detail panel: (active line, passive line).
pub fn describe(c: &Card, slot: Option<usize>) -> (String, String) {
    let p = power(c.theme);
    let active = match slot {
        Some(i) if i < ABILITY_SLOTS => {
            format!("{}: {}", SLOT_KEYS[i].to_uppercase(), p.active.name())
        }
        Some(_) => "(passive only)".to_string(),
        None => p.active.name().to_string(),
    };
    let mut passive = format!("+ {}", p.passive.info().name);
    if let Some(b) = c.fused {
        passive += &format!("  + {}", power(b).passive.info().name);
    }
    // A resonant card doubles its own passive (where the cap allows).
    if c.rarity == crate::cards::Rarity::Resonant && p.passive.info().max >= 2 {
        passive += "  (x2)";
    }
    (active, passive)
}

/// Which ability slot a chosen card's active fills, or `None` if it brings
/// no active (the slots are full, or an earlier card teaches the same one).
pub fn slot_of(chosen: &[&Card], number: u32) -> Option<usize> {
    let mut seen = Vec::new();
    for c in chosen {
        let a = power(c.theme).active;
        let fills = seen.len() < ABILITY_SLOTS && !seen.contains(&a);
        if c.number == number {
            return fills.then_some(seen.len());
        }
        if fills {
            seen.push(a);
        }
    }
    None
}

/// How the grid marks cards: filled in (in the loadout / the first merge
/// card), a letter badge, and cards that can't be picked right now.
#[derive(Default)]
pub struct GridMarks<'a> {
    pub marked: &'a [u32],
    /// A card that wears a letter, like the first card of a merge: ("A").
    pub badge: Option<(u32, &'static str)>,
    /// Cards of this dream are dimmed (they can't merge with the first card).
    pub dim_theme: Option<crate::dream::DreamTheme>,
}

/// The tile size for a grid `width` wide.
fn tile_size(width: f32, u: f32) -> Vec2 {
    let gap = 8.0 * u;
    let w = ((width - gap * (COLS as f32 - 1.0)) / COLS as f32).clamp(40.0, 120.0 * u);
    Vec2::new(w, (w * 0.66).max(28.0))
}

/// Where the grid's tile `i` sits (row `row` of the visible window).
fn tile_rect(left: f32, top: f32, tile: Vec2, gap: f32, i: usize, first_row: usize) -> Rect {
    let row = i / COLS;
    Rect::from_min_size(
        Pos2::new(
            left + (i % COLS) as f32 * (tile.x + gap),
            top + (row - first_row) as f32 * (tile.y + gap),
        ),
        tile,
    )
}

/// The booklet's cards as small pixel tiles, `COLS` wide, scrolling with the
/// cursor. Each shows its number, attribute and what its active ability is.
/// Returns the bottom of the grid.
#[allow(clippy::too_many_arguments)]
pub fn draw_grid(
    p: &egui::Painter,
    screen: Rect,
    v: &HudView,
    cards: &[Card],
    selected: usize,
    marks: &GridMarks,
    grid_top: f32,
    width: f32,
) -> f32 {
    let u = pixel_ui::scale(screen);
    let c = pixel_ui::cell(u);
    let left = screen.left() + screen.width() * 0.05;
    let gap = 8.0 * u;
    let tile = tile_size(width, u);
    let first_row = (selected / COLS).saturating_sub(ROWS - 1);
    let shown = (cards.len().div_ceil(COLS) - first_row.min(cards.len().div_ceil(COLS))).min(ROWS);
    for (i, card) in cards.iter().enumerate() {
        let row = i / COLS;
        if row < first_row || row >= first_row + ROWS {
            continue;
        }
        let r = tile_rect(left, grid_top, tile, gap, i, first_row);
        let col = card_frame(card, v.time);
        let power = power(card.theme);
        let on = marks.marked.contains(&card.number);
        let dim = marks.dim_theme == Some(card.theme);
        let sel = i == selected;
        let a = if dim { 0.35 } else { 1.0 };
        pixel_ui::panel(
            p,
            r,
            col,
            rgba(
                if on { col } else { [14, 8, 28] },
                if on { 0.38 } else { 0.9 },
            ),
            c.min(tile.y / 8.0).max(2.0),
            if sel { 1.0 } else { 0.7 * a },
        );
        // A band of the rarity colour along the top.
        p.rect_filled(
            Rect::from_min_size(
                r.left_top() + Vec2::new(c * 2.0, c * 2.0),
                Vec2::new(tile.x - c * 4.0, c),
            ),
            0.0,
            rgba(col, 0.55 * a),
        );
        // The ability this card teaches.
        let ipx = (c * 0.67).max(2.0);
        pixel_ui::icon(
            p,
            r.left_center() + Vec2::new(c * 2.0 + ipx * 4.5, c * 0.7),
            icons::ability(power.active),
            ipx,
            power.tint,
            pixel_ui::lighter(power.tint),
            a,
        );
        let text_x = r.left() + c * 3.0 + ipx * 9.0;
        p.text(
            Pos2::new(text_x, r.top() + tile.y * 0.34),
            Align2::LEFT_CENTER,
            format!("#{}", card.number),
            FontId::monospace((tile.y * 0.24).max(9.0)),
            rgba(ink(v), 0.9 * a),
        );
        let attr = card.attribute.label();
        let size = pixel_ui::fit(attr, tile.y * 0.24, r.right() - text_x - c * 2.0, 7.0);
        p.text(
            Pos2::new(text_x, r.top() + tile.y * 0.7),
            Align2::LEFT_CENTER,
            attr,
            FontId::monospace(size),
            rgba(col, a),
        );
        if on && marks.badge.is_none() {
            pixel_ui::check(
                p,
                r.right_top() + Vec2::new(-c * 4.0, c * 4.5),
                (c * 0.67).max(2.0),
                rgba([255, 255, 255], 0.95),
            );
        }
        if let Some((n, letter)) = marks.badge {
            if n == card.number {
                let b = Rect::from_center_size(
                    r.right_top() + Vec2::new(-c * 4.0, c * 4.5),
                    Vec2::splat(c * 4.5),
                );
                p.rect_filled(b, 0.0, rgba([255, 240, 150], 1.0));
                p.text(
                    b.center(),
                    Align2::CENTER_CENTER,
                    letter,
                    FontId::monospace(c * 4.0),
                    rgba([20, 10, 0], 1.0),
                );
            }
        }
        if sel {
            // The cursor: brackets that breathe a cell in and out.
            let bob = if v.settings_snapshot.motion() >= 1.0 {
                ((v.time * 4.0).sin() * 0.5 + 0.5).round() * c
            } else {
                0.0
            };
            pixel_ui::brackets(p, r.expand(c * 0.5 + bob), c, rgba(ink(v), 1.0));
        }
    }
    // Where you are in a long booklet: a thin track beside the grid.
    let rows_total = cards.len().div_ceil(COLS);
    let height = shown as f32 * (tile.y + gap) - gap;
    if rows_total > ROWS {
        let x = left + COLS as f32 * (tile.x + gap) - gap + c * 2.0;
        let track = Rect::from_min_size(Pos2::new(x, grid_top), Vec2::new(c, height));
        p.rect_filled(track, 0.0, rgba(ink(v), 0.12));
        let thumb_h = (height * ROWS as f32 / rows_total as f32).max(c * 3.0);
        let thumb_y = grid_top + (height - thumb_h) * first_row as f32 / (rows_total - ROWS) as f32;
        p.rect_filled(
            Rect::from_min_size(Pos2::new(x, thumb_y), Vec2::new(c, thumb_h)),
            0.0,
            rgba(ink(v), 0.7),
        );
    }
    p.text(
        Pos2::new(left, grid_top + height + 6.0 * u),
        Align2::LEFT_TOP,
        format!("{} / {}", (selected + 1).min(cards.len()), cards.len()),
        FontId::monospace(13.0 * u),
        rgba(ink(v), 0.5),
    );
    grid_top + height
}

/// A card's ability and passive as icon lines, for the detail panels.
/// Returns the height used.
pub fn power_lines(
    p: &egui::Painter,
    at: Pos2,
    width: f32,
    c: &Card,
    slot: Option<usize>,
    u: f32,
    time: f32,
) -> f32 {
    let pw = power(c.theme);
    let (active, passive) = describe(c, slot);
    let ipx = (3.0 * u).round().max(2.0);
    let row_h = ipx * 9.0 + 8.0 * u;
    // The ability.
    pixel_ui::icon(
        p,
        at + Vec2::new(ipx * 4.5, ipx * 4.5),
        icons::ability(pw.active),
        ipx,
        pw.tint,
        pixel_ui::lighter(pw.tint),
        1.0,
    );
    let tx = at.x + ipx * 10.5;
    p.text(
        Pos2::new(tx, at.y),
        Align2::LEFT_TOP,
        &active,
        FontId::monospace(pixel_ui::fit(&active, 17.0 * u, width - ipx * 10.5, 9.0)),
        rgba([255, 200, 255], 0.95),
    );
    p.text(
        Pos2::new(tx, at.y + 20.0 * u),
        Align2::LEFT_TOP,
        pw.active.desc(),
        FontId::monospace(pixel_ui::fit(
            pw.active.desc(),
            12.0 * u,
            width - ipx * 10.5,
            7.0,
        )),
        rgba([200, 180, 225], 0.75),
    );
    // The passive.
    let y2 = at.y + row_h + 6.0 * u;
    let info = pw.passive.info();
    pixel_ui::icon(
        p,
        Pos2::new(at.x + ipx * 4.5, y2 + ipx * 4.5),
        crate::upgrade_ui::icon_rows(info.icon),
        ipx,
        info.color,
        pixel_ui::lighter(info.color),
        1.0,
    );
    p.text(
        Pos2::new(tx, y2),
        Align2::LEFT_TOP,
        &passive,
        FontId::monospace(pixel_ui::fit(&passive, 15.0 * u, width - ipx * 10.5, 8.0)),
        rgba([170, 255, 200], 0.95),
    );
    p.text(
        Pos2::new(tx, y2 + 20.0 * u),
        Align2::LEFT_TOP,
        info.desc,
        FontId::monospace(pixel_ui::fit(info.desc, 12.0 * u, width - ipx * 10.5, 7.0)),
        rgba([160, 220, 190], 0.7),
    );
    // The flavour, a little glitchy.
    let y3 = y2 + row_h + 6.0 * u;
    let wob = if (time * 0.7).fract() < 0.04 {
        1.0
    } else {
        0.0
    };
    pixel_ui::split_text(
        p,
        Pos2::new(at.x + wob, y3),
        Align2::LEFT_TOP,
        pw.flavour,
        pixel_ui::fit(pw.flavour, 13.0 * u, width, 7.0),
        rgba([190, 180, 220], 0.9),
        1.0,
        0.5,
    );
    y3 + 18.0 * u - at.y
}

/// The bottom hint line, in a pixel strip.
pub fn hint_bar(p: &egui::Painter, screen: Rect, v: &HudView, text: &str) {
    let u = pixel_ui::scale(screen);
    let c = pixel_ui::cell(u);
    let text = v.k(text);
    let size = pixel_ui::fit(&text, 19.0 * u, screen.width() - 60.0, 9.0);
    let w = (text.chars().count() as f32 * size * 0.6 + c * 8.0).min(screen.width() - 20.0);
    let r = Rect::from_center_size(
        Pos2::new(screen.center().x, screen.bottom() - 34.0 * u),
        Vec2::new(w, size + c * 4.0),
    );
    pixel_ui::panel(p, r, [120, 100, 170], rgba([10, 5, 24], 0.85), c, 0.6);
    p.text(
        r.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::monospace(size),
        rgba(ink(v), 0.6 + 0.3 * (v.time * 2.0).sin()),
    );
}

pub fn draw(ctx: &egui::Context, p: &egui::Painter, screen: Rect, v: &HudView, art: &mut CardArt) {
    let m = &v.loadout;
    let u = pixel_ui::scale(screen);
    let c = pixel_ui::cell(u);
    p.rect_filled(screen, 0.0, rgba([8, 4, 20], 0.95));
    let cx = screen.center().x;
    let top = screen.top() + 22.0 * u;
    glitch_text(
        p,
        Pos2::new(cx, top),
        true,
        "LOADOUT",
        44.0 * u,
        ink(v),
        1.0,
        v,
    );
    let chosen: Vec<&Card> = m
        .chosen
        .iter()
        .filter_map(|&n| m.cards.iter().find(|c| c.number == n))
        .collect();

    // The slots: each shows the card in it and the ability it brings.
    let gap = 10.0 * u;
    let slot_w = ((screen.width() * 0.9 - gap * (crate::lore::MAX_SLOTS as f32 - 1.0))
        / crate::lore::MAX_SLOTS as f32)
        .min(190.0 * u);
    let slot_h = 64.0 * u;
    let row_y = top + 62.0 * u;
    let total = crate::lore::MAX_SLOTS as f32 * (slot_w + gap) - gap;
    for i in 0..crate::lore::MAX_SLOTS {
        let r = Rect::from_min_size(
            Pos2::new(cx - total * 0.5 + i as f32 * (slot_w + gap), row_y),
            Vec2::new(slot_w, slot_h),
        );
        let open = i < m.slots;
        match chosen.get(i) {
            Some(card) => {
                let col = card_frame(card, v.time);
                pixel_ui::panel(p, r, col, rgba(col, 0.16), c, 1.0);
                let pw = power(card.theme);
                pixel_ui::icon(
                    p,
                    r.left_center() + Vec2::new(c * 2.0 + c * 6.0, 0.0),
                    icons::ability(pw.active),
                    c * 0.67,
                    pw.tint,
                    pixel_ui::lighter(pw.tint),
                    1.0,
                );
                let tx = r.left() + c * 3.0 + c * 0.67 * 9.0 + c;
                let text_w = r.right() - tx - c * 2.0;
                let galley = p.layout(
                    card.name.clone(),
                    FontId::monospace((12.0 * u).max(8.0)),
                    rgba(col, 0.95),
                    text_w,
                );
                // Two lines at most; anything longer is cut off with a tilde.
                let line_h = galley.size().y / galley.rows.len().max(1) as f32;
                let rows = galley.rows.len().min(2);
                let mut shown: String = String::new();
                for (k, row) in galley.rows.iter().take(rows).enumerate() {
                    if k > 0 {
                        shown.push('\n');
                    }
                    shown.push_str(row.text().trim_end());
                }
                if galley.rows.len() > 2 {
                    shown.pop();
                    shown.push('~');
                }
                p.text(
                    Pos2::new(tx, r.top() + c * 2.0),
                    Align2::LEFT_TOP,
                    shown,
                    FontId::monospace((12.0 * u).max(8.0)),
                    rgba(col, 0.95),
                );
                let _ = line_h;
                let key = slot_of(&chosen, card.number)
                    .map(|k| SLOT_KEYS[k].to_uppercase())
                    .unwrap_or_else(|| "PASSIVE".to_string());
                p.text(
                    Pos2::new(tx, r.bottom() - c * 2.0),
                    Align2::LEFT_BOTTOM,
                    key,
                    FontId::monospace((10.0 * u).max(7.0)),
                    rgba(ink(v), 0.6),
                );
            }
            None if open => {
                pixel_ui::panel(p, r, [120, 110, 150], rgba([14, 8, 28], 0.6), c, 0.8);
                p.text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    "empty",
                    FontId::monospace(13.0 * u),
                    rgba([150, 140, 180], 0.7),
                );
            }
            None => {
                pixel_ui::panel(p, r, [60, 55, 80], rgba([10, 6, 20], 0.6), c, 0.6);
                pixel_ui::hatch(p, r.shrink(c * 2.0), c, Color32::from_white_alpha(10));
                pixel_ui::icon(
                    p,
                    r.center(),
                    &icons::LOCK,
                    (c * 0.67).max(2.0),
                    [90, 80, 120],
                    [120, 110, 150],
                    0.9,
                );
            }
        }
    }

    if let Some(line) = &m.companion {
        p.text(
            Pos2::new(cx, row_y + slot_h + 10.0 * u),
            Align2::CENTER_TOP,
            v.k(line),
            FontId::monospace(15.0 * u),
            rgba([200, 170, 230], 0.9),
        );
    }

    // The grid (scrolls with the cursor) on the left; the card and its powers on the right.
    let grid_top = row_y + slot_h + 38.0 * u;
    let left = screen.left() + screen.width() * 0.05;
    let detail_w = CARD_SIZE.x * 1.0 + 24.0 * u;
    let grid_w = (screen.right() - screen.width() * 0.05 - detail_w - 24.0 * u - left).max(300.0);
    let marks = GridMarks {
        marked: &m.chosen,
        ..Default::default()
    };
    draw_grid(p, screen, v, &m.cards, m.selected, &marks, grid_top, grid_w);

    if let Some(card_sel) = m.cards.get(m.selected) {
        let avail = screen.bottom() - 64.0 * u - grid_top;
        let sc = ((avail - 150.0 * u) / CARD_SIZE.y).clamp(0.45, 1.0);
        let size = CARD_SIZE * sc;
        let right = screen.right() - screen.width() * 0.05 - size.x;
        let r = Rect::from_min_size(Pos2::new(right, grid_top), size);
        let col = card_frame(card_sel, v.time);
        // A panel behind the card, so it sits on something.
        let back = r.expand2(Vec2::new(10.0 * u, 10.0 * u));
        pixel_ui::panel(p, back, col, rgba([12, 6, 26], 0.8), c, 0.5);
        card(ctx, p, r, card_sel, v, art, sc);
        let slot = if m.chosen.contains(&card_sel.number) {
            slot_of(&chosen, card_sel.number).or(Some(usize::MAX))
        } else {
            None
        };
        let at = Pos2::new(r.left(), back.bottom() + 12.0 * u);
        pixel_ui::split_text(
            p,
            at,
            Align2::LEFT_TOP,
            &card_sel.name,
            pixel_ui::fit(&card_sel.name, 15.0 * u, size.x, 8.0),
            rgba(col, 1.0),
            1.0,
            0.5,
        );
        power_lines(
            p,
            at + Vec2::new(0.0, 22.0 * u),
            size.x,
            card_sel,
            slot,
            u,
            v.time,
        );
    }

    hint_bar(
        p,
        screen,
        v,
        "[a/d/w/s] browse   [enter] add/remove   [space] fall asleep   [esc] back",
    );
    pixel_ui::scanlines(p, screen, c, 10);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{card_from, tests::record};
    use crate::dream::DreamTheme::*;

    #[test]
    fn toggle_adds_removes_and_respects_the_slots() {
        let mut l = vec![];
        assert!(toggle(&mut l, 4, 1));
        assert!(!toggle(&mut l, 5, 1), "full");
        assert_eq!(l, vec![4]);
        assert!(toggle(&mut l, 4, 1));
        assert!(l.is_empty());
    }

    #[test]
    fn the_cursor_stays_on_the_grid() {
        assert_eq!(step(0, 10, -1, 0), 0);
        assert_eq!(step(0, 10, 0, 1), COLS);
        assert_eq!(step(8, 10, 0, 1), 9);
        assert_eq!(step(0, 0, 1, 1), 0);
    }

    #[test]
    fn the_detail_names_the_key_or_says_passive_only() {
        let mut a = card_from(&record(TheTunnel, 1, 1, 0.1, false));
        a.number = 1;
        let mut b = card_from(&record(TheTunnel, 2, 1, 0.1, false));
        b.number = 2;
        let chosen = [&a, &b];
        assert_eq!(slot_of(&chosen, 1), Some(0));
        assert_eq!(slot_of(&chosen, 2), None, "DASH is already taught");
        assert_eq!(describe(&a, Some(0)).0, "SHIFT: DASH");
        assert_eq!(describe(&b, Some(usize::MAX)).0, "(passive only)");
        assert_eq!(describe(&a, None).1, "+ SWIFT FEET");
    }
}
