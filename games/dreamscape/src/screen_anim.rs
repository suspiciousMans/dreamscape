//! M4b: the small amount of state the loadout / deck builder / booklet
//! screens animate with (entrance, hover spring, press squash, meter, tick-up).
//!
//! Pure: no GL, no clock. The painter feeds `now` (the HUD's `time`) once per
//! frame; the state lives in egui's temp memory so no game struct changes.
//! `amount` is `ui_motion::effective(juice, reduced_motion)`: 0 means the
//! final state at once (nothing moves), 1 is Soft, 1.4 is Full.

use crate::ui_motion::{ease_out_cubic, smoothstep, stagger, tick_up, Spring, SOFT_DAMPING, SOFT_STIFFNESS};

/// Entrance length of one item and the delay between neighbours.
pub const ENTER_DUR: f32 = 0.24;
pub const ENTER_STAGGER: f32 = 0.03;
/// Numbers settle in at most this long.
pub const TICK_DUR: f32 = 0.8;
/// A press squashes to `PRESS_MIN` in `PRESS_DOWN`, then eases back in `PRESS_UP`.
pub const PRESS_MIN: f32 = 0.96;
pub const PRESS_DOWN: f32 = 0.08;
pub const PRESS_UP: f32 = 0.14;
/// The focused item grows by this much (1.0 -> 1.04).
pub const FOCUS_GROW: f32 = 0.04;
/// Seconds without a frame after which the screen counts as freshly opened.
const REOPEN_GAP: f32 = 0.5;

/// Progress 0..=1 of item `i` of a staggered entrance `age` seconds in.
pub fn enter(i: usize, age: f32, amount: f32) -> f32 {
    if amount <= 0.0 {
        return 1.0;
    }
    ease_out_cubic(stagger(i, ENTER_STAGGER, ENTER_DUR, age))
}

/// A counter easing up to `to` once the screen has been open `age - delay`
/// seconds; exactly `to` when nothing moves.
pub fn tick(to: i64, age: f32, delay: f32, amount: f32) -> i64 {
    if amount <= 0.0 {
        return to;
    }
    tick_up(0, to, (age - delay) / TICK_DUR)
}

/// The press squash `since` seconds after the press: 1.0 outside it.
pub fn press_scale(since: f32, amount: f32) -> f32 {
    if amount <= 0.0 || !(0.0..PRESS_DOWN + PRESS_UP).contains(&since) {
        return 1.0;
    }
    if since < PRESS_DOWN {
        1.0 - (1.0 - PRESS_MIN) * smoothstep(since / PRESS_DOWN)
    } else {
        PRESS_MIN + (1.0 - PRESS_MIN) * ease_out_cubic((since - PRESS_DOWN) / PRESS_UP)
    }
}

/// What the grid painter needs this frame (all plain numbers).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridMotion {
    /// Seconds since the screen opened.
    pub age: f32,
    pub amount: f32,
    /// Booklet number of the focused card, and its spring (0..~1.05).
    pub focus_key: Option<u32>,
    pub focus: f32,
    /// The card the cursor just left, easing back.
    pub prev_key: Option<u32>,
    pub prev: f32,
    /// Squash factor of the focused card (1 = none).
    pub press: f32,
}

impl GridMotion {
    /// No motion: every item at rest, in its final place.
    pub const fn still() -> Self {
        Self { age: 0.0, amount: 0.0, focus_key: None, focus: 0.0, prev_key: None, prev: 0.0, press: 1.0 }
    }

    /// Entrance progress of the tile at column `col`, row `row` (of the window).
    pub fn enter(&self, col: usize, row: usize) -> f32 {
        enter(col + row, self.age, self.amount)
    }

    fn spring_of(&self, number: u32) -> f32 {
        if self.amount <= 0.0 {
            0.0
        } else if self.focus_key == Some(number) {
            self.focus.max(0.0)
        } else if self.prev_key == Some(number) {
            self.prev.max(0.0)
        } else {
            0.0
        }
    }

    /// The tile's size factor: the hover grow, times the press squash if it is the focused one.
    pub fn scale_of(&self, number: u32) -> f32 {
        let press = if self.focus_key == Some(number) { self.press } else { 1.0 };
        (1.0 + FOCUS_GROW * self.spring_of(number)) * press
    }

    /// How far the tile floats up, in cells `c`.
    pub fn lift_of(&self, number: u32, c: f32) -> f32 {
        self.spring_of(number) * c * 0.6
    }

    /// The detail card's warp hover, 0..=1: the selected card warps more.
    pub fn hover(&self) -> f32 {
        if self.focus_key.is_none() {
            0.0
        } else if self.amount <= 0.0 {
            1.0
        } else {
            self.focus.clamp(0.0, 1.0)
        }
    }
}

/// Per-screen animation state, kept in egui memory between frames.
#[derive(Clone, Debug, PartialEq)]
pub struct ScreenAnim {
    opened: f32,
    last: f32,
    epoch: u32,
    epoch_at: f32,
    key: Option<u32>,
    focus: Spring,
    prev_key: Option<u32>,
    prev: Spring,
    marked: bool,
    press_at: f32,
    meter: Spring,
}

impl Default for ScreenAnim {
    fn default() -> Self {
        Self {
            opened: f32::NEG_INFINITY,
            last: f32::NEG_INFINITY,
            epoch: 0,
            epoch_at: 0.0,
            key: None,
            focus: Spring::at(1.0),
            prev_key: None,
            prev: Spring::at(0.0),
            marked: false,
            press_at: f32::NEG_INFINITY,
            meter: Spring::at(0.0),
        }
    }
}

impl ScreenAnim {
    /// Advance to frame time `now`. `epoch` restarts the entrance when it
    /// changes (a booklet page turn); `key` is the focused card's number;
    /// `marked` whether it is in the deck; `meter` the bar's target, 0..1.
    pub fn update(&mut self, now: f32, epoch: u32, key: Option<u32>, marked: bool, meter: f32, amount: f32) {
        let fresh = !(0.0..=REOPEN_GAP).contains(&(now - self.last));
        if fresh {
            *self = Self { opened: now, last: now, epoch_at: now, epoch, ..Self::default() };
            self.key = key;
            self.marked = marked;
            self.focus = Spring::at(1.0);
        }
        let dt = (now - self.last).clamp(0.0, 0.1);
        self.last = now;
        if epoch != self.epoch {
            self.epoch = epoch;
            self.epoch_at = now;
        }
        if key != self.key {
            self.prev_key = self.key;
            self.prev = self.focus;
            self.key = key;
            self.focus = Spring::at(0.0);
            self.marked = marked;
        } else if marked != self.marked {
            self.marked = marked;
            self.press_at = now;
        }
        if amount <= 0.0 {
            self.focus = Spring::at(1.0);
            self.prev = Spring::at(0.0);
            self.meter = Spring::at(meter);
            return;
        }
        self.focus.step(1.0, dt, SOFT_STIFFNESS, SOFT_DAMPING);
        self.prev.step(0.0, dt, SOFT_STIFFNESS, SOFT_DAMPING);
        self.meter.step(meter, dt, 120.0, 22.0);
    }

    /// Seconds since the screen opened / since the current epoch began.
    pub fn age(&self, now: f32) -> f32 {
        (now - self.opened).max(0.0)
    }
    pub fn epoch_age(&self, now: f32) -> f32 {
        (now - self.epoch_at).max(0.0)
    }

    /// The meter bar's fill now, clamped to 0..=1.
    pub fn meter(&self) -> f32 {
        self.meter.pos.clamp(0.0, 1.0)
    }

    pub fn grid(&self, now: f32, amount: f32) -> GridMotion {
        GridMotion {
            age: self.age(now),
            amount,
            focus_key: self.key,
            focus: self.focus.pos,
            prev_key: self.prev_key,
            prev: self.prev.pos,
            press: press_scale(now - self.press_at, amount),
        }
    }
}

/// Loads this screen's state from egui memory, advances it to `now`, stores
/// it back and returns it.
#[allow(clippy::too_many_arguments)]
pub fn advance(
    ctx: &engine::ui::egui::Context,
    id: &'static str,
    now: f32,
    epoch: u32,
    key: Option<u32>,
    marked: bool,
    meter: f32,
    amount: f32,
) -> ScreenAnim {
    let id = engine::ui::egui::Id::new(id);
    let mut a = ctx.data_mut(|d| d.get_temp::<ScreenAnim>(id)).unwrap_or_default();
    a.update(now, epoch, key, marked, meter, amount);
    ctx.data_mut(|d| d.insert_temp(id, a.clone()));
    a
}

// === m4a === helpers for the list screens (title, settings, memories, codex)
/// Where a chip showing `key` sits beside a panel whose right edge is `right`,
/// level with `y`; `None` when it would not fit inside `screen_right`.
pub fn chip_beside(right: f32, y: f32, key: &str, screen_right: f32) -> Option<engine::ui::egui::Pos2> {
    // `hud::prompt_chip` draws 20 px monospace (0.6 em per glyph) plus 16 px of padding.
    let w = key.chars().count() as f32 * 12.0 + 16.0;
    let x = right + 10.0 + w * 0.5;
    (!key.is_empty() && x + w * 0.5 + 4.0 <= screen_right).then(|| engine::ui::egui::Pos2::new(x, y))
}

/// The screen's motion for a list whose focus is row `sel` (`None`: nothing focused).
pub fn list_motion(
    p: &engine::ui::egui::Painter,
    id: &'static str,
    time: f32,
    sel: Option<usize>,
    s: &crate::settings::Settings,
) -> GridMotion {
    let amount = crate::ui_motion::effective(s.ui_juice, s.reduced_motion);
    advance(p.ctx(), id, time, 0, sel.map(|i| i as u32), false, 0.0, amount).grid(time, amount)
}
// === end m4a ===

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chip_beside_a_panel_only_when_it_fits() {
        let p = chip_beside(500.0, 100.0, "ENTER", 640.0).unwrap();
        assert!(p.x > 500.0 && p.x + 38.0 <= 640.0 && p.y == 100.0);
        assert!(chip_beside(600.0, 100.0, "ENTER", 640.0).is_none());
        assert!(chip_beside(100.0, 100.0, "", 640.0).is_none());
    }

    #[test]
    fn off_is_the_final_state_at_once() {
        assert_eq!(enter(9, 0.0, 0.0), 1.0);
        assert_eq!(tick(12, 0.0, 0.0, 0.0), 12);
        assert_eq!(press_scale(0.01, 0.0), 1.0);
        let m = GridMotion::still();
        assert_eq!(m.enter(5, 4), 1.0);
        assert_eq!(m.scale_of(3), 1.0);
        assert_eq!(m.lift_of(3, 3.0), 0.0);
        let mut a = ScreenAnim::default();
        a.update(10.0, 0, Some(3), false, 0.5, 0.0);
        let g = a.grid(10.0, 0.0);
        assert_eq!(g.scale_of(3), 1.0);
        assert_eq!(g.enter(2, 2), 1.0);
        assert_eq!(a.meter(), 0.5);
        assert_eq!(g.hover(), 1.0);
    }

    #[test]
    fn entrances_are_monotone_bounded_and_staggered() {
        for d in 0..11 {
            let mut last = 0.0;
            for k in 0..=100 {
                let e = enter(d, k as f32 * 0.01, 1.0);
                assert!((0.0..=1.0).contains(&e) && e >= last, "d{d} k{k}");
                last = e;
            }
            assert_eq!(last, 1.0);
        }
        assert_eq!(enter(0, 0.0, 1.0), 0.0);
        assert!(enter(0, 0.1, 1.0) > enter(5, 0.1, 1.0), "later tiles start later");
        // The slowest tile (col 5 + row 4) is done well inside 0.6 s.
        assert_eq!(enter(9, 0.6, 1.0), 1.0);
    }

    #[test]
    fn ticks_are_exact_at_the_ends_and_never_overshoot() {
        assert_eq!(tick(240, 0.0, 0.0, 1.0), 0);
        assert_eq!(tick(240, TICK_DUR, 0.0, 1.0), 240);
        assert_eq!(tick(240, 5.0, 0.0, 1.4), 240);
        let mut last = 0;
        for k in 0..=100 {
            let v = tick(240, k as f32 * 0.01, 0.0, 1.0);
            assert!((0..=240).contains(&v) && v >= last);
            last = v;
        }
        assert_eq!(tick(7, 0.1, 0.5, 1.0), 0, "before its delay it is still 0");
    }

    #[test]
    fn the_press_squashes_once_and_settles() {
        assert_eq!(press_scale(-0.1, 1.0), 1.0);
        assert_eq!(press_scale(0.0, 1.0), 1.0);
        assert!((press_scale(PRESS_DOWN, 1.0) - PRESS_MIN).abs() < 1e-5);
        assert_eq!(press_scale(PRESS_DOWN + PRESS_UP, 1.0), 1.0);
        let mut prev = 1.0;
        for k in 0..=80 {
            let s = press_scale(k as f32 * 0.001, 1.0);
            assert!((PRESS_MIN..=1.0).contains(&s));
            if k <= 80 {
                assert!(s <= prev + 1e-6, "going down");
            }
            prev = s;
        }
        let mut prev = PRESS_MIN;
        for k in 80..=220 {
            let s = press_scale(k as f32 * 0.001, 1.0);
            assert!(s >= prev - 1e-6 && s <= 1.0, "coming back, no bounce");
            prev = s;
        }
    }

    #[test]
    fn a_moved_cursor_springs_up_and_the_old_card_eases_back() {
        let mut a = ScreenAnim::default();
        let mut t = 1.0;
        a.update(t, 0, Some(1), false, 0.0, 1.0);
        for _ in 0..60 {
            t += 1.0 / 60.0;
            a.update(t, 0, Some(1), false, 0.0, 1.0);
        }
        assert!((a.grid(t, 1.0).scale_of(1) - (1.0 + FOCUS_GROW)).abs() < 0.002, "settled grown");
        t += 1.0 / 60.0;
        a.update(t, 0, Some(2), false, 0.0, 1.0);
        let g = a.grid(t, 1.0);
        assert!(g.scale_of(2) < g.scale_of(1), "new card starts small, old one starts grown");
        for _ in 0..90 {
            t += 1.0 / 60.0;
            a.update(t, 0, Some(2), false, 0.0, 1.0);
        }
        let g = a.grid(t, 1.0);
        assert!((g.scale_of(1) - 1.0).abs() < 0.002 && (g.scale_of(2) - 1.04).abs() < 0.002);
        assert!(g.scale_of(2) < 1.06, "overshoot stays tiny");
    }

    #[test]
    fn toggling_the_focused_card_presses_it() {
        let mut a = ScreenAnim::default();
        a.update(1.0, 0, Some(4), false, 0.0, 1.0);
        a.update(1.02, 0, Some(4), true, 0.0, 1.0);
        let g = a.grid(1.02 + PRESS_DOWN, 1.0);
        assert!(g.press < 0.97);
        a.update(1.02 + PRESS_DOWN, 0, Some(4), true, 0.0, 1.0);
        assert!(a.grid(1.02 + PRESS_DOWN, 1.0).scale_of(4) < 1.0 + FOCUS_GROW);
        assert_eq!(a.grid(2.0, 1.0).press, 1.0);
    }

    #[test]
    fn a_gap_in_frames_reopens_and_a_page_turn_restarts_the_entrance() {
        let mut a = ScreenAnim::default();
        a.update(5.0, 0, None, false, 0.0, 1.0);
        a.update(5.05, 0, None, false, 0.0, 1.0);
        assert!((a.age(5.05) - 0.05).abs() < 1e-5);
        a.update(8.0, 0, None, false, 0.0, 1.0);
        assert!(a.age(8.0) < 1e-5, "reopened");
        a.update(8.1, 0, None, false, 0.0, 1.0);
        a.update(9.0, 0, None, false, 0.0, 1.0);
        let _ = a.age(9.0);
        let mut b = ScreenAnim::default();
        b.update(1.0, 0, None, false, 0.0, 1.0);
        b.update(1.3, 0, None, false, 0.0, 1.0);
        b.update(1.35, 1, None, false, 0.0, 1.0);
        assert!(b.epoch_age(1.4) < 0.06 && b.age(1.4) > 0.39, "page turn only restarts the cards");
    }

    #[test]
    fn the_meter_climbs_to_its_target() {
        let mut a = ScreenAnim::default();
        let mut t = 0.0;
        a.update(t, 0, None, false, 0.5, 1.0);
        assert_eq!(a.meter(), 0.0);
        for _ in 0..120 {
            t += 1.0 / 60.0;
            a.update(t, 0, None, false, 0.5, 1.0);
            assert!((0.0..=1.0).contains(&a.meter()));
        }
        assert!((a.meter() - 0.5).abs() < 0.01);
    }
}
