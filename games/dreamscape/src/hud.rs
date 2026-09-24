//! The dream HUD. Nothing here is a box or a bar: an eye that opens as you
//! become lucid (and glances toward the nearest shard), a wobbling
//! chromatic-split depth counter, a title card naming each dream, a drifting
//! shard compass, a pause veil, and the dream journal on waking.
//!
//! Layout/animation maths are pure and unit-tested; `draw` paints them with
//! egui's low-level painter and is verified by screenshot.

use engine::ui::egui::{self, Align2, Color32, FontFamily, FontId, Pos2, Rect, Vec2};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Playing,
    Paused,
    Journal,
    Booklet,
}

pub const TITLE_IN: f32 = 0.6;
pub const TITLE_HOLD: f32 = 3.0;
pub const TITLE_OUT: f32 = 1.2;

/// VT323 (SIL Open Font License 1.1, see assets/fonts/OFL.txt): a worn CRT
/// terminal face. Embedded so the game finds it from any working directory.
const DREAM_FONT: &[u8] = include_bytes!("../assets/fonts/VT323-Regular.ttf");

/// Makes VT323 the first choice for both font families.
pub fn install_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("vt323".to_owned(), egui::FontData::from_static(DREAM_FONT));
    for family in [FontFamily::Monospace, FontFamily::Proportional] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "vt323".to_owned());
    }
    ctx.set_fonts(fonts);
}

/// Everything the HUD needs for one frame (a snapshot, so drawing never
/// borrows the game).
#[derive(Clone, Debug)]
pub struct HudView {
    pub mode: Mode,
    pub time: f32,
    pub depth: u32,
    pub best: u32,
    pub lucidity: u32,
    pub lucid_target: u32,
    /// A shard was picked up in this dream and could still be dropped.
    pub unbanked: bool,
    pub strangeness: f32,
    pub title: String,
    pub whisper: String,
    /// Seconds since this dream (or the journal) began.
    pub title_age: f32,
    /// Screen-space unit direction to the shard / wake door, when it's far.
    pub shard_dir: Option<[f32; 2]>,
    /// World distance to the shard / wake door (shown under the compass).
    pub shard_dist: Option<f32>,
    pub journal: Vec<(String, crate::cards::Rarity)>,
    /// Shown on the journal after pressing [s].
    pub journal_status: Option<String>,
    pub booklet_cards: Vec<crate::cards::Card>,
    pub booklet_page: usize,
    pub booklet_pages: usize,
    pub booklet_total: usize,
    pub seed: u64,
}

/// 0.08 = a sleepy slit; 1.0 = wide awake (lucid).
pub fn eye_openness(lucidity: u32, target: u32) -> f32 {
    let t = (lucidity as f32 / target.max(1) as f32).clamp(0.0, 1.0);
    0.08 + 0.92 * t
}

pub fn wobble_amp(strangeness: f32) -> f32 {
    1.0 + 5.0 * strangeness.clamp(0.0, 1.0)
}

/// Per-letter jitter in pixels; grows with strangeness, bounded by `wobble_amp`.
pub fn wobble(i: usize, time: f32, strangeness: f32) -> [f32; 2] {
    let amp = wobble_amp(strangeness);
    let k = i as f32 * 1.7;
    [
        amp * (time * 3.1 + k).sin(),
        amp * (time * 2.3 + k * 1.3).cos(),
    ]
}

/// Title card alpha: fade in, hold, fade out.
pub fn title_alpha(age: f32) -> f32 {
    if age <= 0.0 {
        0.0
    } else if age < TITLE_IN {
        age / TITLE_IN
    } else if age < TITLE_IN + TITLE_HOLD {
        1.0
    } else {
        (1.0 - (age - TITLE_IN - TITLE_HOLD) / TITLE_OUT).max(0.0)
    }
}

/// Screen direction from player to target. The camera looks down +Z with
/// screen-right = world -X and screen-up = world +Z (screen y grows down).
/// `None` when the target is close enough to just see.
pub fn compass(player: [f32; 3], target: [f32; 3], min_dist: f32) -> Option<[f32; 2]> {
    let dx = target[0] - player[0];
    let dz = target[2] - player[2];
    let d = (dx * dx + dz * dz).sqrt();
    (d >= min_dist).then(|| [-dx / d, -dz / d])
}

/// Fully saturated rainbow colour; `t` wraps every 1.0.
pub fn hue(t: f32) -> [u8; 3] {
    let h = t.rem_euclid(1.0) * 6.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}

pub(crate) const INK: [u8; 3] = [245, 240, 255];
const GHOST_RED: [u8; 3] = [255, 40, 110];
const GHOST_CYAN: [u8; 3] = [40, 230, 255];

pub(crate) fn rgba(rgb: [u8; 3], alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        rgb[0],
        rgb[1],
        rgb[2],
        (alpha.clamp(0.0, 1.0) * 255.0) as u8,
    )
}

/// Paints cells as crisp squares of `px` pixels centred on `origin`.
fn px_cells(
    p: &egui::Painter,
    origin: Pos2,
    px: f32,
    cells: impl IntoIterator<Item = (i32, i32)>,
    color: Color32,
) {
    let o = Pos2::new(origin.x.round(), origin.y.round());
    for (x, y) in cells {
        let min = o + Vec2::new(x as f32 * px - px * 0.5, y as f32 * px - px * 0.5);
        p.rect_filled(Rect::from_min_size(min, Vec2::splat(px)), 0.0, color);
    }
}

const EYE_PX: f32 = 6.0;
const ARROW_PX: f32 = 4.0;

/// The HUD's signature: text laid out letter by letter so each letter can
/// wobble, drawn as a red ghost, a cyan ghost, then the core. The split
/// widens as the dream gets stranger.
#[allow(clippy::too_many_arguments)]
pub(crate) fn glitch_text(
    p: &egui::Painter,
    at: Pos2,
    centered: bool,
    text: &str,
    size: f32,
    core: [u8; 3],
    alpha: f32,
    v: &HudView,
) {
    let font = FontId::monospace(size);
    let advance = p
        .layout_no_wrap("M".to_owned(), font.clone(), Color32::WHITE)
        .size()
        .x;
    let count = text.chars().count() as f32;
    let x0 = if centered {
        at.x - advance * count * 0.5
    } else {
        at.x
    };
    let split = 1.0 + 4.0 * v.strangeness;
    for (i, ch) in text.chars().enumerate() {
        let w = wobble(i, v.time, v.strangeness);
        let pos = Pos2::new(x0 + advance * i as f32 + w[0], at.y + w[1]);
        let s = ch.to_string();
        p.text(
            pos - Vec2::new(split, 0.0),
            Align2::LEFT_TOP,
            &s,
            font.clone(),
            rgba(GHOST_RED, alpha * 0.6),
        );
        p.text(
            pos + Vec2::new(split, 0.0),
            Align2::LEFT_TOP,
            &s,
            font.clone(),
            rgba(GHOST_CYAN, alpha * 0.6),
        );
        p.text(pos, Align2::LEFT_TOP, &s, font.clone(), rgba(core, alpha));
    }
}

pub fn draw(ctx: &egui::Context, v: &HudView, art: &mut crate::booklet_ui::CardArt) {
    let p = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("dream_hud"),
    ));
    let screen = ctx.screen_rect();
    match v.mode {
        Mode::Journal => return journal(&p, screen, v),
        Mode::Booklet => return crate::booklet_ui::draw(ctx, &p, screen, v, art),
        Mode::Playing | Mode::Paused => {}
    }
    depth_counter(&p, screen, v);
    lucidity_eye(&p, screen, v);
    title_card(&p, screen, v);
    if let Some(dir) = v.shard_dir {
        shard_compass(&p, screen, dir, v);
    }
    if v.mode == Mode::Paused {
        pause_veil(&p, screen, v);
    }
}

fn depth_counter(p: &egui::Painter, screen: Rect, v: &HudView) {
    let at = screen.left_top() + Vec2::new(28.0, 18.0);
    glitch_text(
        p,
        at,
        false,
        &format!("DEPTH {:02}", v.depth),
        44.0,
        INK,
        1.0,
        v,
    );
    p.text(
        at + Vec2::new(2.0, 48.0),
        Align2::LEFT_TOP,
        format!("deepest {}", v.best),
        FontId::monospace(20.0),
        rgba(INK, 0.55),
    );
}

fn lucidity_eye(p: &egui::Painter, screen: Rect, v: &HudView) {
    use crate::pixels::{self, EyePx};
    let c = Pos2::new(screen.center().x, screen.bottom() - 96.0);
    let lucid = v.lucidity >= v.lucid_target;
    let open = (eye_openness(v.lucidity, v.lucid_target) * (1.0 + 0.05 * (v.time * 1.7).sin()))
        .min(1.0)
        * pixels::blink(v.time);
    let cells = pixels::eye_pixels(open, v.shard_dir, lucid);
    let iris = if lucid {
        hue(v.time * 0.15)
    } else {
        hue(0.75 + 0.1 * v.strangeness)
    };
    let rim = [iris[0] / 2, iris[1] / 2, iris[2] / 2];
    // Pixel drop shadow, one cell down.
    px_cells(
        p,
        c + Vec2::new(0.0, EYE_PX),
        EYE_PX,
        cells
            .iter()
            .filter(|q| q.2 != EyePx::Lash)
            .map(|q| (q.0, q.1)),
        rgba([0, 0, 0], 0.45),
    );
    for &(x, y, kind) in &cells {
        let color = match kind {
            EyePx::Lid => rgba(INK, 0.95),
            EyePx::Lash => rgba(INK, 0.75),
            EyePx::Sclera => rgba([30, 18, 52], 0.92),
            EyePx::Iris => rgba(iris, 1.0),
            EyePx::IrisRim => rgba(rim, 1.0),
            EyePx::Pupil => rgba([6, 0, 12], 1.0),
            EyePx::Glint => rgba([255, 255, 255], 0.95),
        };
        px_cells(p, c, EYE_PX, [(x, y)], color);
    }
    if lucid {
        // Pixel sunburst: 8 flickering rays.
        for i in 0..8 {
            let a = TAU * i as f32 / 8.0 + PI / 8.0;
            for k in 0..3 {
                if ((v.time * 6.0) as i32 + i + k) % 3 == 0 {
                    continue;
                }
                let rad = 15.0 + k as f32 * 1.5;
                let cell = (
                    (a.cos() * rad).round() as i32,
                    (a.sin() * rad * 0.62).round() as i32,
                );
                px_cells(
                    p,
                    c,
                    EYE_PX,
                    [cell],
                    rgba(hue(v.time * 0.2 + i as f32 / 8.0), 0.85),
                );
            }
        }
    }
    // Shard pips: pixel diamonds; a shard that could still be dropped blinks.
    let full = pixels::sprite(&pixels::PIP);
    let empty = pixels::sprite(&pixels::PIP_EMPTY);
    for i in 0..v.lucid_target {
        let at = c + Vec2::new(
            (i as f32 - (v.lucid_target - 1) as f32 * 0.5) * 30.0,
            9.0 * EYE_PX,
        );
        let filled = i < v.lucidity;
        let blinking = filled && v.unbanked && i + 1 == v.lucidity;
        let alpha = if blinking {
            if (v.time * 5.0) as i32 % 2 == 0 {
                1.0
            } else {
                0.25
            }
        } else if filled {
            1.0
        } else {
            0.55
        };
        let rgb = if filled {
            hue(v.time * 0.1 + i as f32 * 0.33)
        } else {
            INK
        };
        px_cells(
            p,
            at,
            3.0,
            if filled { full.clone() } else { empty.clone() },
            rgba(rgb, alpha),
        );
    }
}

fn shard_compass(p: &egui::Painter, screen: Rect, dir: [f32; 2], v: &HudView) {
    use crate::pixels;
    let f = pixels::snap8(dir);
    let fv = Vec2::new(f[0], f[1]);
    let bob = ((v.time * 5.0).sin() * 1.5).round() * ARROW_PX;
    let raw = screen.center() + Vec2::new(dir[0], dir[1]) * (screen.height() * 0.30) + fv * bob;
    // Snap to the pixel grid so the sprite never shimmers between pixels.
    let at = Pos2::new(
        (raw.x / ARROW_PX).round() * ARROW_PX,
        (raw.y / ARROW_PX).round() * ARROW_PX,
    );
    let lucid = v.lucidity >= v.lucid_target;
    let cells = pixels::arrow_cells(dir);
    px_cells(
        p,
        at + Vec2::splat(ARROW_PX),
        ARROW_PX,
        cells.iter().copied(),
        rgba([0, 0, 0], 0.55),
    );
    for k in 1..=3 {
        if ((v.time * 8.0) as i32 + k) % 2 == 0 {
            let back = at - fv * (ARROW_PX * (6.0 + 3.0 * k as f32));
            px_cells(
                p,
                back,
                ARROW_PX,
                [(0, 0)],
                rgba(hue(v.time * 0.5 + 0.1 * k as f32), 0.8 - 0.2 * k as f32),
            );
        }
    }
    for &(x, y) in &cells {
        // Lucid: it points at the wake door, so it burns white-gold instead of rainbow.
        let rgb = if lucid {
            if (x + y + (v.time * 6.0) as i32) % 3 == 0 {
                [255, 255, 255]
            } else {
                [255, 210, 80]
            }
        } else {
            hue(v.time * 0.6 - (x as f32 * f[0] + y as f32 * f[1]) * 0.06)
        };
        px_cells(p, at, ARROW_PX, [(x, y)], rgba(rgb, 0.95));
    }
    if let Some(dist) = v.shard_dist {
        p.text(
            at + Vec2::new(0.0, 9.0 * ARROW_PX),
            Align2::CENTER_TOP,
            format!("{dist:.0}m"),
            FontId::monospace(20.0),
            rgba(INK, 0.8),
        );
    }
}

fn title_card(p: &egui::Painter, screen: Rect, v: &HudView) {
    let a = title_alpha(v.title_age);
    if a <= 0.0 {
        return;
    }
    let core = if v.strangeness > 0.6 {
        hue(v.time * 0.25)
    } else {
        INK
    };
    let y = screen.top() + screen.height() * 0.2;
    glitch_text(
        p,
        Pos2::new(screen.center().x, y),
        true,
        &v.title,
        40.0,
        core,
        a,
        v,
    );
    p.text(
        Pos2::new(screen.center().x, y + 50.0),
        Align2::CENTER_TOP,
        &v.whisper,
        FontId::proportional(24.0),
        rgba([200, 190, 230], a * 0.85),
    );
}

fn pause_veil(p: &egui::Painter, screen: Rect, v: &HudView) {
    p.rect_filled(screen, 0.0, rgba([10, 5, 25], 0.78));
    let c = screen.center();
    glitch_text(
        p,
        c - Vec2::new(0.0, 40.0),
        true,
        "THE DREAM HOLDS ITS BREATH",
        38.0,
        INK,
        1.0,
        v,
    );
    p.text(
        c + Vec2::new(0.0, 24.0),
        Align2::CENTER_TOP,
        "[esc] keep dreaming",
        FontId::monospace(22.0),
        rgba(INK, 0.8),
    );
    p.text(
        c + Vec2::new(0.0, 52.0),
        Align2::CENTER_TOP,
        "[q] wake up for real",
        FontId::monospace(22.0),
        rgba(INK, 0.5),
    );
    p.text(
        c + Vec2::new(0.0, 80.0),
        Align2::CENTER_TOP,
        "[b] dream booklet",
        FontId::monospace(22.0),
        rgba(INK, 0.5),
    );
}

fn journal(p: &egui::Painter, screen: Rect, v: &HudView) {
    p.rect_filled(screen, 0.0, rgba([8, 4, 20], 0.92));
    let cx = screen.center().x;
    glitch_text(
        p,
        Pos2::new(cx, screen.top() + 50.0),
        true,
        "DREAM JOURNAL",
        50.0,
        hue(v.time * 0.1),
        1.0,
        v,
    );
    p.text(
        Pos2::new(cx, screen.top() + 110.0),
        Align2::CENTER_TOP,
        format!("seed {}  ·  deepest {}", v.seed, v.best),
        FontId::monospace(20.0),
        rgba(INK, 0.55),
    );
    let first = v.journal.len().saturating_sub(14);
    for (i, (line, rarity)) in v.journal[first..].iter().enumerate() {
        // Entries surface one by one, like remembering.
        let a = ((v.title_age - 0.12 * i as f32) / 0.4).clamp(0.0, 1.0);
        let y = screen.top() + 148.0 + 26.0 * i as f32;
        p.text(
            Pos2::new(cx - 300.0, y),
            Align2::LEFT_TOP,
            line,
            FontId::monospace(24.0),
            rgba(INK, a),
        );
        p.text(
            Pos2::new(cx + 330.0, y),
            Align2::RIGHT_TOP,
            rarity.label(),
            FontId::monospace(20.0),
            rgba(crate::booklet_ui::rarity_color(*rarity, v.time), a),
        );
    }
    if let Some(status) = &v.journal_status {
        p.text(
            Pos2::new(cx, screen.bottom() - 90.0),
            Align2::CENTER_TOP,
            status,
            FontId::monospace(22.0),
            rgba([255, 210, 80], 0.9),
        );
    }
    let keys = if v.journal_status.is_some() {
        "[b] booklet   [r] dream again   [esc] wake for real"
    } else {
        "[s] press into booklet   [b] booklet   [r] dream again   [esc] wake"
    };
    p.text(
        Pos2::new(cx, screen.bottom() - 54.0),
        Align2::CENTER_TOP,
        keys,
        FontId::monospace(22.0),
        rgba(INK, 0.5 + 0.3 * (v.time * 2.0).sin()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eye_opens_with_lucidity() {
        assert!((eye_openness(0, 3) - 0.08).abs() < 1e-6);
        assert!(eye_openness(1, 3) < eye_openness(2, 3));
        assert_eq!(eye_openness(3, 3), 1.0);
        assert_eq!(eye_openness(9, 3), 1.0, "clamps past lucid");
    }

    #[test]
    fn wobble_is_bounded_and_grows_with_strangeness() {
        let peak = |s: f32| {
            (0..40)
                .flat_map(|i| (0..100).map(move |k| wobble(i, k as f32 * 0.07, s)))
                .map(|w| w[0].abs().max(w[1].abs()))
                .fold(0.0_f32, f32::max)
        };
        assert!(peak(0.0) <= wobble_amp(0.0) + 1e-4);
        assert!(peak(1.0) <= wobble_amp(1.0) + 1e-4);
        assert!(peak(1.0) > 3.0 * peak(0.0));
    }

    #[test]
    fn title_fades_in_holds_and_fades_out() {
        assert_eq!(title_alpha(0.0), 0.0);
        assert!((title_alpha(TITLE_IN * 0.5) - 0.5).abs() < 1e-4);
        assert_eq!(title_alpha(TITLE_IN + 1.0), 1.0);
        assert!(title_alpha(TITLE_IN + TITLE_HOLD + TITLE_OUT * 0.5) < 1.0);
        assert_eq!(title_alpha(TITLE_IN + TITLE_HOLD + TITLE_OUT + 0.1), 0.0);
    }

    #[test]
    fn compass_matches_the_camera() {
        let up = compass([0.0; 3], [0.0, 0.0, 10.0], 5.0).unwrap();
        assert!(up[1] < -0.99, "+Z is up-screen, got {up:?}");
        let right = compass([0.0; 3], [-10.0, 0.0, 0.0], 5.0).unwrap();
        assert!(right[0] > 0.99, "-X is screen-right, got {right:?}");
        assert!(compass([0.0; 3], [1.0, 0.0, 1.0], 5.0).is_none());
    }

    #[test]
    fn hue_hits_the_primaries() {
        assert_eq!(hue(0.0), [255, 0, 0]);
        assert_eq!(hue(1.0 / 3.0)[1], 255);
        assert_eq!(hue(2.0 / 3.0)[2], 255);
        assert_eq!(hue(1.0), hue(0.0));
    }

    #[test]
    fn dream_font_is_a_real_truetype_file() {
        assert!(DREAM_FONT.len() > 50_000);
        assert_eq!(&DREAM_FONT[0..4], &[0, 1, 0, 0], "TrueType magic");
    }
}
