//! The dream HUD. Nothing here is a box or a bar: an eye that opens as you
//! become lucid (and glances toward the nearest shard), a wobbling
//! chromatic-split depth counter, a title card naming each dream, a drifting
//! shard compass, a pause veil, and the dream journal on waking.
//!
//! Layout/animation maths are pure and unit-tested; `draw` paints them with
//! egui's low-level painter and is verified by screenshot.

use engine::ui::egui::{self, Align2, Color32, FontFamily, FontId, Pos2, Rect, Stroke, Vec2};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Playing,
    Paused,
    Journal,
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
    pub journal: Vec<String>,
    pub seed: u64,
}

/// 0.08 = a sleepy slit; 1.0 = wide awake (lucid).
pub fn eye_openness(lucidity: u32, target: u32) -> f32 {
    let t = (lucidity as f32 / target.max(1) as f32).clamp(0.0, 1.0);
    0.08 + 0.92 * t
}

/// Upper and lower eyelid curves, `n` points each, left to right, for an eye
/// centred at (cx, cy) with half-width `w`. Screen y grows downward.
pub fn eyelids(
    cx: f32,
    cy: f32,
    w: f32,
    openness: f32,
    n: usize,
) -> (Vec<[f32; 2]>, Vec<[f32; 2]>) {
    let h = w * 0.55 * openness;
    let mut top = Vec::with_capacity(n);
    let mut bottom = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / (n - 1) as f32;
        let x = cx - w + 2.0 * w * t;
        // Exact zero at the corners (sin(PI) isn't 0.0 in f32) so the lids meet.
        let bulge = if i == 0 || i == n - 1 {
            0.0
        } else {
            (PI * t).sin()
        };
        top.push([x, cy - h * bulge]);
        bottom.push([x, cy + h * 0.8 * bulge]);
    }
    (top, bottom)
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

const INK: [u8; 3] = [245, 240, 255];
const GHOST_RED: [u8; 3] = [255, 40, 110];
const GHOST_CYAN: [u8; 3] = [40, 230, 255];

fn rgba(rgb: [u8; 3], alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        rgb[0],
        rgb[1],
        rgb[2],
        (alpha.clamp(0.0, 1.0) * 255.0) as u8,
    )
}

/// The HUD's signature: text laid out letter by letter so each letter can
/// wobble, drawn as a red ghost, a cyan ghost, then the core. The split
/// widens as the dream gets stranger.
#[allow(clippy::too_many_arguments)]
fn glitch_text(
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

fn diamond(p: &egui::Painter, c: Pos2, r: f32, spin: f32, filled: bool, color: Color32) {
    let pts: Vec<Pos2> = (0..4)
        .map(|k| {
            let a = spin + k as f32 * PI * 0.5;
            c + Vec2::new(a.cos() * r * 0.7, a.sin() * r)
        })
        .collect();
    if filled {
        p.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
    } else {
        p.add(egui::Shape::closed_line(pts, Stroke::new(1.5_f32, color)));
    }
}

pub fn draw(ctx: &egui::Context, v: &HudView) {
    let p = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("dream_hud"),
    ));
    let screen = ctx.screen_rect();
    if v.mode == Mode::Journal {
        journal(&p, screen, v);
        return;
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
    let c = Pos2::new(screen.center().x, screen.bottom() - 78.0);
    let w = 64.0;
    let lucid = v.lucidity >= v.lucid_target;
    // The eye breathes; it is never perfectly still.
    let open =
        (eye_openness(v.lucidity, v.lucid_target) * (1.0 + 0.06 * (v.time * 1.7).sin())).min(1.0);
    let n = 24;
    let (top, bottom) = eyelids(c.x, c.y, w, open, n);
    let mut outline: Vec<Pos2> = top.iter().map(|q| Pos2::new(q[0], q[1])).collect();
    // Skip the shared corner points so the polygon has no duplicate vertices.
    outline.extend(
        bottom
            .iter()
            .rev()
            .skip(1)
            .take(n - 2)
            .map(|q| Pos2::new(q[0], q[1])),
    );
    p.add(egui::Shape::convex_polygon(
        outline.clone(),
        rgba([12, 6, 24], 0.75),
        Stroke::NONE,
    ));
    let h = w * 0.55 * open;
    if h > 6.0 {
        // Iris glances toward the shard when there is one.
        let gaze = v
            .shard_dir
            .map_or(Vec2::ZERO, |d| Vec2::new(d[0], d[1]) * (w * 0.25));
        let r = (h * 0.85).min(w * 0.4);
        let iris = if lucid {
            hue(v.time * 0.15)
        } else {
            hue(0.75 + 0.1 * v.strangeness)
        };
        p.circle_filled(c + gaze, r, rgba(iris, 0.95));
        p.circle_filled(
            c + gaze,
            r * (0.4 + 0.08 * (v.time * 2.0).sin()),
            rgba([5, 0, 10], 1.0),
        );
        p.circle_filled(
            c + gaze + Vec2::new(-r * 0.3, -r * 0.35),
            r * 0.15,
            rgba([255, 255, 255], 0.8),
        );
    }
    p.add(egui::Shape::closed_line(
        outline,
        Stroke::new(3.0_f32, rgba(INK, 0.9)),
    ));
    if lucid {
        for i in 0..12 {
            let a = TAU * i as f32 / 12.0 + v.time * 0.3;
            let len = 14.0 + 8.0 * (v.time * 3.0 + i as f32).sin().abs();
            let dir = Vec2::new(a.cos(), a.sin());
            let from = c + dir * (w + 8.0);
            p.line_segment(
                [from, from + dir * len],
                Stroke::new(2.0_f32, rgba(hue(v.time * 0.2 + i as f32 / 12.0), 0.8)),
            );
        }
    }
    // One diamond pip per shard needed; a shard that could still be dropped blinks.
    for i in 0..v.lucid_target {
        let x = c.x + (i as f32 - (v.lucid_target - 1) as f32 * 0.5) * 26.0;
        let at = Pos2::new(x, c.y + w * 0.55 + 22.0);
        let filled = i < v.lucidity;
        let blinking = filled && v.unbanked && i + 1 == v.lucidity;
        let alpha = if blinking {
            0.35 + 0.65 * (v.time * 6.0).sin().abs()
        } else if filled {
            1.0
        } else {
            0.5
        };
        let rgb = if filled {
            hue(v.time * 0.1 + i as f32 * 0.33)
        } else {
            INK
        };
        diamond(p, at, 7.0, v.time * 0.8, filled, rgba(rgb, alpha));
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

fn shard_compass(p: &egui::Painter, screen: Rect, dir: [f32; 2], v: &HudView) {
    let d = Vec2::new(dir[0], dir[1]);
    let at = screen.center() + d * (screen.height() * 0.28 + 6.0 * (v.time * 2.5).sin());
    let rgb = hue(v.time * 0.5);
    diamond(p, at, 10.0, v.time * 2.0, true, rgba(rgb, 0.9));
    p.line_segment(
        [at - d * 14.0, at - d * 40.0],
        Stroke::new(2.0_f32, rgba(rgb, 0.35)),
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
    let first = v.journal.len().saturating_sub(16);
    for (i, line) in v.journal[first..].iter().enumerate() {
        // Entries surface one by one, like remembering.
        let a = ((v.title_age - 0.12 * i as f32) / 0.4).clamp(0.0, 1.0);
        p.text(
            Pos2::new(cx - 190.0, screen.top() + 148.0 + 26.0 * i as f32),
            Align2::LEFT_TOP,
            line,
            FontId::monospace(24.0),
            rgba(INK, a),
        );
    }
    p.text(
        Pos2::new(cx, screen.bottom() - 54.0),
        Align2::CENTER_TOP,
        "[r] dream again        [esc] wake for real",
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
    fn eyelids_meet_at_the_corners_and_gap_scales_with_openness() {
        let (t1, b1) = eyelids(100.0, 50.0, 60.0, 1.0, 21);
        let (t0, b0) = eyelids(100.0, 50.0, 60.0, 0.1, 21);
        assert_eq!(t1[0], b1[0]);
        assert_eq!(t1[20], b1[20]);
        let gap = |t: &[[f32; 2]], b: &[[f32; 2]]| b[10][1] - t[10][1];
        assert!(gap(&t1, &b1) > 0.0);
        assert!((gap(&t1, &b1) / gap(&t0, &b0) - 10.0).abs() < 1e-3);
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
