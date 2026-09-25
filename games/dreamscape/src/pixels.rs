//! Pixel-art sprites for the HUD as integer cells (pure, unit-tested).
//! Cells are (x, y), y grows downward, (0, 0) is the sprite centre.

use std::f32::consts::PI;

pub const EYE_HALF_W: i32 = 12;
pub const EYE_MAX_HALF_H: f32 = 6.0;
pub const IRIS_R: f32 = 4.6;
/// How far (in cells) the pupil may travel toward a shard, before clamping.
const GAZE_REACH: [f32; 2] = [7.0, 3.0];
pub const BLINK_PERIOD: f32 = 4.7;
pub const BLINK_LEN: f32 = 0.16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EyePx {
    Lid,
    Sclera,
    Iris,
    IrisRim,
    Pupil,
    Glint,
    Lash,
}

/// Socket half-heights (above, below) at column `x`, in cells.
pub fn socket(x: i32, openness: f32) -> (f32, f32) {
    let u = x as f32 / (EYE_HALF_W as f32 + 0.5);
    let h = EYE_MAX_HALF_H * openness.clamp(0.0, 1.0) * (1.0 - u * u).max(0.0);
    (h, h * 0.8)
}

/// The mask. Nothing but lashes is ever drawn outside it.
pub fn in_socket(x: i32, y: i32, openness: f32) -> bool {
    if x.abs() > EYE_HALF_W {
        return false;
    }
    let (above, below) = socket(x, openness);
    let y = y as f32;
    y >= -above - 0.5 && y <= below + 0.5
}

/// Inside the socket and not on its outline (the lid line).
fn interior(x: i32, y: i32, openness: f32) -> bool {
    in_socket(x, y, openness)
        && [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .all(|(dx, dy)| in_socket(x + dx, y + dy, openness))
}

/// Awake-but-dreaming: a 3-tall cat slit. Lucid: a dilated round pupil.
pub fn pupil_offsets(lucid: bool) -> Vec<(i32, i32)> {
    if lucid {
        let mut v = Vec::new();
        for dy in -2..=2 {
            for dx in -2..=2 {
                if ((dx * dx + dy * dy) as f32).sqrt() <= 2.3 {
                    v.push((dx, dy));
                }
            }
        }
        v
    } else {
        vec![(0, -1), (0, 0), (0, 1)]
    }
}

fn pupil_fits(c: (i32, i32), openness: f32, lucid: bool) -> bool {
    pupil_offsets(lucid)
        .iter()
        .all(|&(dx, dy)| interior(c.0 + dx, c.1 + dy, openness))
}

/// Where the pupil sits: as far toward the shard as possible while the whole
/// pupil stays inside the socket (shrinks the reach until it fits).
pub fn gaze_centre(dir: Option<[f32; 2]>, openness: f32, lucid: bool) -> (i32, i32) {
    let Some(d) = dir else { return (0, 0) };
    for k in (0..=10).rev() {
        let t = k as f32 / 10.0;
        let c = (
            (d[0] * GAZE_REACH[0] * t).round() as i32,
            (d[1] * GAZE_REACH[1] * t).round() as i32,
        );
        if pupil_fits(c, openness, lucid) {
            return c;
        }
    }
    (0, 0)
}

pub fn eye_pixels(openness: f32, gaze: Option<[f32; 2]>, lucid: bool) -> Vec<(i32, i32, EyePx)> {
    let (cx, cy) = gaze_centre(gaze, openness, lucid);
    let pupil: Vec<(i32, i32)> = pupil_offsets(lucid)
        .into_iter()
        .map(|(dx, dy)| (cx + dx, cy + dy))
        .collect();
    let mut out = Vec::new();
    let max_h = EYE_MAX_HALF_H as i32 + 1;
    for y in -max_h..=max_h {
        for x in -EYE_HALF_W..=EYE_HALF_W {
            if !in_socket(x, y, openness) {
                continue;
            }
            let d = (((x - cx).pow(2) + (y - cy).pow(2)) as f32).sqrt();
            let kind = if !interior(x, y, openness) {
                EyePx::Lid
            } else if (x, y) == (cx - 2, cy - 2) && d <= IRIS_R {
                EyePx::Glint
            } else if pupil.contains(&(x, y)) {
                EyePx::Pupil
            } else if d <= IRIS_R - 1.0 {
                EyePx::Iris
            } else if d <= IRIS_R {
                EyePx::IrisRim
            } else {
                EyePx::Sclera
            };
            out.push((x, y, kind));
        }
    }
    if openness > 0.3 {
        for x in [-6, -2, 2, 6] {
            let (above, _) = socket(x, openness);
            out.push((x, -((above + 0.5).floor() as i32) - 1, EyePx::Lash));
        }
    }
    out
}

/// 1.0 = open; dips to 0 for a quick blink every `BLINK_PERIOD` seconds.
pub fn blink(time: f32) -> f32 {
    let ph = time.rem_euclid(BLINK_PERIOD);
    if ph < BLINK_LEN {
        (2.0 * ph / BLINK_LEN - 1.0).abs()
    } else {
        1.0
    }
}

pub const PIP: [&str; 5] = ["..#..", ".###.", "#####", ".###.", "..#.."];
pub const PIP_EMPTY: [&str; 5] = ["..#..", ".#.#.", "#...#", ".#.#.", "..#.."];

/// '#' cells of an odd-sized ASCII sprite, centred on (0, 0).
pub fn sprite(rows: &[&str]) -> Vec<(i32, i32)> {
    let h = rows.len() as i32;
    let mut out = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        let w = row.len() as i32;
        for (x, ch) in row.chars().enumerate() {
            if ch == '#' {
                out.push((x as i32 - w / 2, y as i32 - h / 2));
            }
        }
    }
    out
}

/// Snaps a direction to the nearest of 8 (pixel art only looks right at 45°
/// steps). Float dust is zeroed so axis-aligned arrows are exactly symmetric.
pub fn snap8(d: [f32; 2]) -> [f32; 2] {
    let step = PI / 4.0;
    let a = (d[1].atan2(d[0]) / step).round() * step;
    let clean = |v: f32| if v.abs() < 1e-6 { 0.0 } else { v };
    [clean(a.cos()), clean(a.sin())]
}

fn in_tri(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let s = |p: [f32; 2], q: [f32; 2], r: [f32; 2]| {
        (p[0] - r[0]) * (q[1] - r[1]) - (q[0] - r[0]) * (p[1] - r[1])
    };
    let (d1, d2, d3) = (s(p, a, b), s(p, b, c), s(p, c, a));
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

/// A chevron arrowhead rasterised onto cells, pointing along `dir` (screen space).
pub fn arrow_cells(dir: [f32; 2]) -> Vec<(i32, i32)> {
    let f = snap8(dir);
    let r = [-f[1], f[0]];
    // Local frame: +ly = forward.
    let (tip, right, notch, left) = ([0.0, 5.5], [4.5, -3.5], [0.0, -0.5], [-4.5, -3.5]);
    let mut out = Vec::new();
    for y in -7..=7 {
        for x in -7..=7 {
            let (xf, yf) = (x as f32, y as f32);
            let l = [xf * r[0] + yf * r[1], xf * f[0] + yf * f[1]];
            if in_tri(l, tip, right, notch) || in_tri(l, tip, notch, left) {
                out.push((x, y));
            }
        }
    }
    out
}

/// 9x9 pixel icons for the shop. '#' = main ink, '+' = accent, '.' = empty.
#[rustfmt::skip]
pub mod icons {
    pub const SIZE: usize = 9;

    pub const SHARD: [&str; SIZE] = [
        "....#....", "...#+#...", "..#+++#..", ".#+++++#.", "#+++#+++#",
        ".#+++++#.", "..#+++#..", "...#+#...", "....#....",
    ];
    pub const HEART: [&str; SIZE] = [
        ".........", ".##...##.", "#++#.#++#", "#+++#+++#", "#+++++++#",
        ".#+++++#.", "..#+++#..", "...#+#...", "....#....",
    ];
    pub const BRAIN: [&str; SIZE] = [
        "..#####..", ".#++#++#.", "#+#+#+#+#", "#++#+#++#", "#+#+++#+#",
        "#++#+#++#", ".#++#++#.", "..##.##..", "...#.#...",
    ];
    pub const LID: [&str; SIZE] = [
        ".........", ".........", ".........", "#.......#", ".#######.",
        "..#.#.#..", ".#..#..#.", ".........", ".........",
    ];
    pub const WIND: [&str; SIZE] = [
        ".........", "####+....", "......#..", "######+..", ".........",
        "..######.", "#.......#", "..###+#..", ".........",
    ];
    pub const MAGNET: [&str; SIZE] = [
        ".#######.", "#+++++++#", "#+#####+#", "#+#...#+#", "#+#...#+#",
        "##.....##", "##.....##", ".........", ".+..+..+.",
    ];
    pub const BOOT: [&str; SIZE] = [
        "...###...", "...#+#...", "...#+#...", "...#+#...", "...#+##..",
        "..#++++#.", ".#+++++##", "#########", ".........",
    ];
    pub const EYE: [&str; SIZE] = [
        ".........", ".........", "..#####..", ".#++#++#.", "#++###++#",
        ".#++#++#.", "..#####..", ".........", ".........",
    ];
    pub const GEM: [&str; SIZE] = [
        ".........", "..#####..", ".#+#+#+#.", "#########", "#+++++++#",
        ".#+++++#.", "..#+++#..", "...#+#...", "....#....",
    ];
    pub const CARD: [&str; SIZE] = [
        ".#######.", ".#+++++#.", ".#+#+#+#.", ".#+++++#.", ".#+###+#.",
        ".#+++++#.", ".#+++++#.", ".#+++++#.", ".#######.",
    ];
    pub const INK: [&str; SIZE] = [
        "....#....", "...#+#...", "...#+#...", "..#+++#..", ".#+++++#.",
        "#+++++++#", "#+++++++#", ".#+++++#.", "..#####..",
    ];

    /// (x, y, accent?) for every filled pixel, centred on (0, 0).
    pub fn cells(icon: &[&str; SIZE]) -> Vec<(i32, i32, bool)> {
        let h = (SIZE / 2) as i32;
        icon.iter()
            .enumerate()
            .flat_map(|(y, row)| {
                row.chars().enumerate().filter_map(move |(x, ch)| match ch {
                    '#' => Some((x as i32 - h, y as i32 - h, false)),
                    '+' => Some((x as i32 - h, y as i32 - h, true)),
                    _ => None,
                })
            })
            .collect()
    }

    #[cfg(test)]
    pub const ALL: [&[&str; SIZE]; 11] = [
        &SHARD, &HEART, &BRAIN, &LID, &WIND, &MAGNET, &BOOT, &EYE, &GEM, &CARD, &INK,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirs() -> Vec<Option<[f32; 2]>> {
        let mut v: Vec<Option<[f32; 2]>> = (0..16)
            .map(|i| {
                let a = i as f32 / 16.0 * std::f32::consts::TAU;
                Some([a.cos(), a.sin()])
            })
            .collect();
        v.push(None);
        v
    }

    #[test]
    fn sprites_are_centred() {
        let s = sprite(&PIP);
        assert_eq!(s.len(), 13);
        assert!(
            s.contains(&(0, -2))
                && s.contains(&(-2, 0))
                && s.contains(&(2, 0))
                && s.contains(&(0, 2))
        );
    }

    #[test]
    fn a_closed_eye_is_a_single_line() {
        let px = eye_pixels(0.08, Some([0.0, -1.0]), false);
        assert!(!px.is_empty());
        assert!(
            px.iter().all(|&(_, y, k)| y == 0 && k == EyePx::Lid),
            "{px:?}"
        );
    }

    #[test]
    fn nothing_leaves_the_socket() {
        for step in 0..=20 {
            let open = step as f32 / 20.0;
            for gaze in dirs() {
                for lucid in [false, true] {
                    for (x, y, k) in eye_pixels(open, gaze, lucid) {
                        if k == EyePx::Lash {
                            assert!(!in_socket(x, y, open), "lash inside socket");
                        } else {
                            assert!(
                                in_socket(x, y, open),
                                "{k:?} at ({x},{y}) open={open} gaze={gaze:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn pupil_is_fully_visible_whenever_the_eye_is_open() {
        for (open, lucid) in [
            (0.5, false),
            (0.7, false),
            (1.0, false),
            (0.9, true),
            (1.0, true),
        ] {
            for gaze in dirs() {
                let n = eye_pixels(open, gaze, lucid)
                    .iter()
                    .filter(|p| p.2 == EyePx::Pupil)
                    .count();
                // The glint may cover one pupil cell.
                assert!(
                    n + 1 >= pupil_offsets(lucid).len(),
                    "open={open} lucid={lucid} gaze={gaze:?}: {n} pupil cells"
                );
            }
        }
    }

    #[test]
    fn gaze_moves_the_pupil() {
        let centroid = |g: [f32; 2]| {
            let px: Vec<_> = eye_pixels(1.0, Some(g), false)
                .into_iter()
                .filter(|p| p.2 == EyePx::Pupil)
                .collect();
            let n = px.len() as f32;
            (
                px.iter().map(|p| p.0 as f32).sum::<f32>() / n,
                px.iter().map(|p| p.1 as f32).sum::<f32>() / n,
            )
        };
        assert!(centroid([1.0, 0.0]).0 >= 4.0);
        assert!(centroid([-1.0, 0.0]).0 <= -4.0);
        assert!(centroid([0.0, -1.0]).1 < 0.0);
    }

    #[test]
    fn dreaming_pupil_is_a_slit_and_lucid_pupil_is_round() {
        let width = |lucid: bool| {
            let xs: Vec<i32> = pupil_offsets(lucid).iter().map(|p| p.0).collect();
            xs.iter().max().unwrap() - xs.iter().min().unwrap() + 1
        };
        assert_eq!(width(false), 1);
        assert_eq!(width(true), 5);
    }

    #[test]
    fn blink_is_brief() {
        assert_eq!(blink(1.0), 1.0);
        assert_eq!(blink(0.0), 1.0);
        assert!(blink(BLINK_LEN * 0.5) < 0.05);
        let closed = (0..4700)
            .filter(|i| blink(*i as f32 / 1000.0) < 0.5)
            .count();
        assert!(closed < 120, "closed for {closed}ms per period");
    }

    #[test]
    fn snap8_snaps() {
        let s = snap8([0.9, -0.2]);
        assert!((s[0] - 1.0).abs() < 1e-5 && s[1].abs() < 1e-5);
        let d = snap8([0.6, -0.8]);
        assert!((d[0] - d[1].abs()).abs() < 1e-5 && d[1] < 0.0, "{d:?}");
    }

    #[test]
    fn arrow_points_where_it_should() {
        let up = arrow_cells([0.0, -1.0]);
        let mirrored: std::collections::HashSet<_> = up.iter().map(|&(x, y)| (-x, y)).collect();
        assert_eq!(
            mirrored,
            up.iter().copied().collect(),
            "up arrow is symmetric"
        );
        let tip = up.iter().map(|&(_, y)| -y).max().unwrap();
        assert!(tip >= 5, "tip reaches {tip}");
        let left = arrow_cells([-1.0, 0.0]);
        let mean_x = left.iter().map(|p| p.0 as f32).sum::<f32>() / left.len() as f32;
        assert!(mean_x < 0.0);
    }

    #[test]
    fn diagonal_arrow_has_similar_mass() {
        let a = arrow_cells([0.0, -1.0]).len() as f32;
        let b = arrow_cells([0.7, -0.7]).len() as f32;
        assert!(
            a > 15.0 && (a - b).abs() / a < 0.3,
            "up {a} vs diagonal {b}"
        );
    }

    #[test]
    fn icons_are_square_and_use_only_known_pixels() {
        for (n, icon) in icons::ALL.iter().enumerate() {
            for row in icon.iter() {
                assert_eq!(row.len(), icons::SIZE, "icon {n}: {row:?}");
                assert!(row.chars().all(|c| "#+.".contains(c)), "icon {n}: {row:?}");
            }
            let cells = icons::cells(icon);
            assert!(cells.len() >= 12, "icon {n} is nearly empty");
            assert!(cells.iter().all(|&(x, y, _)| x.abs() <= 4 && y.abs() <= 4));
        }
    }
}
