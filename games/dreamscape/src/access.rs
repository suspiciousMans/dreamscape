//! Colour-blind markers. The shard and the portal normally take the dream's
//! own accent colours, which can be indistinguishable from the walls. With
//! the setting on they use fixed Okabe-Ito colours (safe for the common
//! kinds of colour blindness) and a hard-edged pattern, so they read by
//! shape and brightness as well as hue.

use crate::dream::{Pattern, TexSpec};

/// Okabe-Ito yellow: shards.
pub const SHARD: [u8; 3] = [240, 228, 66];
/// Okabe-Ito blue: portals and the wake door.
pub const PORTAL: [u8; 3] = [0, 114, 178];
const INK: [u8; 3] = [10, 10, 10];
const PAPER: [u8; 3] = [255, 255, 255];

pub fn shard(seed: u64) -> TexSpec {
    TexSpec {
        pattern: Pattern::Checker,
        palette: vec![SHARD, INK],
        bands: 2.0,
        seed,
    }
}

pub fn portal(seed: u64) -> TexSpec {
    TexSpec {
        pattern: Pattern::Rings,
        palette: vec![PORTAL, PAPER],
        bands: 3.0,
        seed,
    }
}

/// WCAG relative luminance of an sRGB colour.
pub fn luminance(c: [u8; 3]) -> f32 {
    let lin = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}

pub fn contrast(a: [u8; 3], b: [u8; 3]) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_use_their_own_colours() {
        assert!(shard(1).palette.contains(&SHARD));
        assert!(portal(1).palette.contains(&PORTAL));
    }

    #[test]
    fn each_marker_has_strong_internal_contrast() {
        for spec in [shard(1), portal(1)] {
            assert!(contrast(spec.palette[0], spec.palette[1]) >= 4.5);
        }
    }

    #[test]
    fn shard_and_portal_differ_in_hue_and_brightness() {
        // Distinct even with no colour vision: the shard is much brighter.
        assert!(luminance(SHARD) > luminance(PORTAL) * 3.0);
    }

    #[test]
    fn markers_render_pixels() {
        assert!(!shard(7).rgba().is_empty());
        assert!(!portal(7).rgba().is_empty());
    }
}
