//! KALEIDOSCOPE: the finished frame is folded into four mirrored quadrants
//! around the middle, so the dream turns back on itself. Purely visual — the
//! layout, the route and every enemy are exactly where they were. Off under
//! `reduce_flashing` and reduced motion, since a mirrored frame is exactly the
//! kind of continuous, disorienting motion those settings exist to stop.
//!
//! The fold lives in the game's composite shader (`post_composite.frag`,
//! driven by `PostParams::mirror`); this module is the pure description the
//! shader and its tests share.

/// How far the fold goes when it is on at all.
pub const FOLDED: f32 = 1.0;

/// How strongly to fold the frame: `FOLDED` only when the twist is in play and
/// neither accessibility setting is asking for stillness.
pub fn strength(twist: bool, reduce_flashing: bool, reduced_motion: bool) -> f32 {
    if twist && !reduce_flashing && !reduced_motion {
        FOLDED
    } else {
        0.0
    }
}

/// Quadrant fold about the centre: a point and its three reflections all land
/// on the same corner of the lower-right quadrant. Mirrors the shader's
/// `0.5 + abs(uv - 0.5)`.
pub fn fold(uv: (f32, f32)) -> (f32, f32) {
    (0.5 + (uv.0 - 0.5).abs(), 0.5 + (uv.1 - 0.5).abs())
}

/// `uv` folded by `strength` (0 = untouched, 1 = fully folded).
pub fn shaped(uv: (f32, f32), strength: f32) -> (f32, f32) {
    let f = fold(uv);
    (
        uv.0 + (f.0 - uv.0) * strength,
        uv.1 + (f.1 - uv.1) * strength,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folding_twice_changes_nothing() {
        for i in 0..=20 {
            for j in 0..=20 {
                let uv = (i as f32 / 20.0, j as f32 / 20.0);
                let once = fold(uv);
                assert_eq!(fold(once), once, "not idempotent at {uv:?}");
            }
        }
    }

    #[test]
    fn folding_is_symmetric_about_the_centre() {
        for i in 0..=10 {
            let a = i as f32 / 10.0;
            for j in 0..=10 {
                let b = j as f32 / 10.0;
                let f = fold((a, b));
                assert_eq!(fold((1.0 - a, b)), f);
                assert_eq!(fold((a, 1.0 - b)), f);
                assert_eq!(fold((1.0 - a, 1.0 - b)), f);
            }
        }
        assert_eq!(fold((0.5, 0.5)), (0.5, 0.5));
    }

    #[test]
    fn a_folded_point_stays_on_screen() {
        for i in 0..=50 {
            for j in 0..=50 {
                let f = fold((i as f32 / 50.0, j as f32 / 50.0));
                assert!((0.5..=1.0).contains(&f.0) && (0.5..=1.0).contains(&f.1), "{f:?}");
            }
        }
    }

    #[test]
    fn zero_strength_is_the_identity_and_full_strength_is_the_fold() {
        let uv = (0.2, 0.7);
        assert_eq!(shaped(uv, 0.0), uv);
        assert_eq!(shaped(uv, 1.0), fold(uv));
        // A half fold sits between the two.
        let half = shaped(uv, 0.5);
        assert!(half.0 > uv.0 && half.0 < fold(uv).0);
    }

    #[test]
    fn the_fold_is_off_without_the_twist_or_under_either_setting() {
        assert_eq!(strength(true, false, false), FOLDED);
        assert_eq!(strength(false, false, false), 0.0);
        assert_eq!(strength(true, true, false), 0.0, "reduce flashing must win");
        assert_eq!(strength(true, false, true), 0.0, "reduced motion must win");
    }
}
