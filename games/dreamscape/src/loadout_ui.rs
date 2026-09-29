//! The loadout screen, before a run: pick which booklet cards come along.
//! Each card brings its ability (the first ones fill the ability slots),
//! its passive, and its dream, which turns up once in the run.

use crate::booklet_ui::{card, card_frame, CardArt, CARD_SIZE};
use crate::cards::Card;
use crate::hud::{glitch_text, ink, rgba, HudView};
use crate::powers::power;
use crate::upgrades::{ABILITY_SLOTS, SLOT_KEYS};
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect, Stroke, Vec2};

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

/// The booklet's cards as small tiles, `COLS` wide, scrolling with the
/// cursor; `marked` cards are filled in.
pub fn draw_grid(
    p: &egui::Painter,
    screen: Rect,
    v: &HudView,
    cards: &[Card],
    selected: usize,
    marked: &[u32],
    grid_top: f32,
) {
    let left = screen.left() + screen.width() * 0.05;
    let tile = Vec2::new(92.0, 48.0);
    let first_row = (selected / COLS).saturating_sub(ROWS - 1);
    for (i, c) in cards.iter().enumerate() {
        let row = i / COLS;
        if row < first_row || row >= first_row + ROWS {
            continue;
        }
        let at = Pos2::new(
            left + (i % COLS) as f32 * (tile.x + 8.0),
            grid_top + (row - first_row) as f32 * (tile.y + 8.0),
        );
        let r = Rect::from_min_size(at, tile);
        let col = card_frame(c, v.time);
        let inn = marked.contains(&c.number);
        p.rect_filled(r, 3.0, rgba(col, if inn { 0.35 } else { 0.08 }));
        let w = if i == selected { 3.0 } else { 1.0 };
        p.rect_stroke(r, 3.0, Stroke::new(w, rgba(col, 0.9)));
        p.text(
            r.center() - Vec2::new(0.0, 8.0),
            Align2::CENTER_CENTER,
            format!("#{}", c.number),
            FontId::monospace(12.0),
            rgba(ink(v), 0.8),
        );
        p.text(
            r.center() + Vec2::new(0.0, 10.0),
            Align2::CENTER_CENTER,
            c.attribute.label(),
            FontId::monospace(12.0),
            rgba(col, 1.0),
        );
    }
}

pub fn draw(ctx: &egui::Context, p: &egui::Painter, screen: Rect, v: &HudView, art: &mut CardArt) {
    let m = &v.loadout;
    p.rect_filled(screen, 0.0, rgba([8, 4, 20], 0.95));
    let cx = screen.center().x;
    let top = screen.top() + 24.0;
    glitch_text(p, Pos2::new(cx, top), true, "LOADOUT", 44.0, ink(v), 1.0, v);
    let chosen: Vec<&Card> = m
        .chosen
        .iter()
        .filter_map(|&n| m.cards.iter().find(|c| c.number == n))
        .collect();

    // The slots.
    let slot_w = 170.0;
    let row_y = top + 64.0;
    let total = crate::lore::MAX_SLOTS as f32 * (slot_w + 12.0) - 12.0;
    for i in 0..crate::lore::MAX_SLOTS {
        let r = Rect::from_min_size(
            Pos2::new(cx - total * 0.5 + i as f32 * (slot_w + 12.0), row_y),
            Vec2::new(slot_w, 40.0),
        );
        let open = i < m.slots;
        let (text, col) = match chosen.get(i) {
            Some(c) => (c.name.clone(), card_frame(c, v.time)),
            None if open => ("empty".into(), [120, 110, 150]),
            None => ("locked".into(), [60, 55, 80]),
        };
        p.rect_stroke(
            r,
            3.0,
            Stroke::new(2.0_f32, rgba(col, if open { 0.9 } else { 0.4 })),
        );
        p.text(
            r.center(),
            Align2::CENTER_CENTER,
            text,
            FontId::monospace(13.0),
            rgba(col, if open { 0.95 } else { 0.5 }),
        );
    }

    if let Some(line) = &m.companion {
        p.text(
            Pos2::new(cx, row_y + 50.0),
            Align2::CENTER_TOP,
            v.k(line),
            FontId::monospace(15.0),
            rgba([200, 170, 230], 0.9),
        );
    }

    // The grid (scrolls with the cursor).
    let grid_top = row_y + 76.0;
    draw_grid(p, screen, v, &m.cards, m.selected, &m.chosen, grid_top);

    // The selected card, big, and what it does.
    if let Some(c) = m.cards.get(m.selected) {
        let right = screen.right() - screen.width() * 0.05 - CARD_SIZE.x;
        let r = Rect::from_min_size(Pos2::new(right, grid_top), CARD_SIZE);
        card(ctx, p, r, c, v, art, 1.0);
        let pw = power(c.theme);
        let slot = if m.chosen.contains(&c.number) {
            slot_of(&chosen, c.number).or(Some(usize::MAX))
        } else {
            None
        };
        let (active, passive) = describe(c, slot);
        let mut y = r.bottom() + 14.0;
        for (text, col, size) in [
            (
                format!("{} · {}", c.name, c.attribute.label()),
                card_frame(c, v.time),
                16.0,
            ),
            (active, [255, 200, 255], 18.0),
            (passive, [170, 255, 200], 16.0),
            (pw.flavour.to_string(), [190, 180, 220], 15.0),
        ] {
            p.text(
                Pos2::new(r.left(), y),
                Align2::LEFT_TOP,
                text,
                FontId::monospace(size),
                rgba(col, 0.95),
            );
            y += size + 10.0;
        }
    }

    p.text(
        Pos2::new(cx, screen.bottom() - 40.0),
        Align2::CENTER_TOP,
        v.k("[a/d/w/s] browse   [enter] add/remove   [space] fall asleep   [esc] back"),
        FontId::monospace(20.0),
        rgba(ink(v), 0.5 + 0.3 * (v.time * 2.0).sin()),
    );
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
        let mut b = card_from(&record(SynesthesiaHall, 2, 1, 0.1, false));
        b.number = 2;
        let chosen = [&a, &b];
        assert_eq!(slot_of(&chosen, 1), Some(0));
        assert_eq!(slot_of(&chosen, 2), None, "DASH is already taught");
        assert_eq!(describe(&a, Some(0)).0, "SHIFT: DASH");
        assert_eq!(describe(&b, Some(usize::MAX)).0, "(passive only)");
        assert_eq!(describe(&a, None).1, "+ SWIFT FEET");
    }
}
