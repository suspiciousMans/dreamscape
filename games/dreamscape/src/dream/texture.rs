//! Procedural "acid" textures. Pure CPU data (RGBA8), no GL — the game uploads them.
//!
//! Every pattern is built only from sin/cos of `2π·(integer·u + integer·v)`, so
//! each texture tiles seamlessly on both axes. Colours come from a cyclic
//! gradient through the theme palette, so a dream's textures always read as
//! "that dream" no matter how wild the pattern gets.

use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::TAU;

pub const TEX_SIZE: u32 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Pattern {
    /// Soft interfering waves.
    Plasma,
    /// Wobbly wallpaper stripes.
    Stripes,
    /// Concentric rings on a torus.
    Rings,
    /// Posterized blobs, like cells under a microscope.
    Cells,
    /// A checkerboard melting into itself.
    Checker,
    /// Slow spirals.
    Swirl,
    /// A grid of almond eyes, each glancing a slightly different way.
    Eyes,
    /// Mirror-folded plasma, like looking through a kaleidoscope.
    Kaleido,
}

#[cfg(test)]
pub const ALL_PATTERNS: [Pattern; 8] = [
    Pattern::Plasma,
    Pattern::Stripes,
    Pattern::Rings,
    Pattern::Cells,
    Pattern::Checker,
    Pattern::Swirl,
    Pattern::Eyes,
    Pattern::Kaleido,
];

/// One periodic wave: sin(2π(a·u + b·v) + phase).
#[derive(Clone, Copy)]
struct Wave {
    a: f32,
    b: f32,
    phase: f32,
}

impl Wave {
    fn random(rng: &mut StdRng, max_freq: i32) -> Self {
        let mut a = rng.gen_range(-max_freq..=max_freq);
        let b = rng.gen_range(-max_freq..=max_freq);
        if a == 0 && b == 0 {
            a = 1;
        }
        Self {
            a: a as f32,
            b: b as f32,
            phase: rng.gen_range(0.0..TAU),
        }
    }

    fn at(self, u: f32, v: f32) -> f32 {
        (TAU * (self.a * u + self.b * v) + self.phase).sin()
    }
}

/// Palette stops for the gradient. A single colour gets a darker and a lighter
/// sibling so there is still something to cycle through.
pub fn stops(palette: &[[u8; 3]]) -> Vec<[f32; 3]> {
    let f = |c: [u8; 3], k: f32| {
        [
            (c[0] as f32 * k).min(255.0),
            (c[1] as f32 * k).min(255.0),
            (c[2] as f32 * k).min(255.0),
        ]
    };
    match palette {
        [] => vec![[128.0; 3], [255.0; 3]],
        [c] => vec![f(*c, 0.7), f(*c, 1.0), f(*c, 1.15)],
        many => many.iter().map(|&c| f(c, 1.0)).collect(),
    }
}

/// Cyclic gradient: t in any range, wraps every 1.0 back to the first stop.
fn gradient(stops: &[[f32; 3]], t: f32) -> [f32; 3] {
    let n = stops.len();
    let x = t.rem_euclid(1.0) * n as f32;
    let i = (x.floor() as usize) % n;
    let j = (i + 1) % n;
    let f = x - x.floor();
    let s = f * f * (3.0 - 2.0 * f); // smoothstep: softer bands
    [
        stops[i][0] + (stops[j][0] - stops[i][0]) * s,
        stops[i][1] + (stops[j][1] - stops[i][1]) * s,
        stops[i][2] + (stops[j][2] - stops[i][2]) * s,
    ]
}

/// Triangle wave in 0..=1 with period 1: continuous and periodic, so it keeps
/// textures seamless while mirroring them.
fn tri_wave(x: f32) -> f32 {
    1.0 - (2.0 * x.rem_euclid(1.0) - 1.0).abs()
}

/// Generates a `TEX_SIZE`² RGBA8 texture.
/// `bands` = how many times the gradient repeats across the value range
/// (more bands = more psychedelic contour lines).
pub fn generate(pattern: Pattern, palette: &[[u8; 3]], bands: f32, seed: u64) -> Vec<u8> {
    let mut rng = StdRng::seed_from_u64(seed);
    let waves: Vec<Wave> = (0..4).map(|_| Wave::random(&mut rng, 3)).collect();
    let warp: Vec<Wave> = (0..2).map(|_| Wave::random(&mut rng, 2)).collect();
    let ring_k = rng.gen_range(2.0..4.0_f32);
    let checker_n = rng.gen_range(2..=4) as f32;
    let stops = stops(palette);
    let size = TEX_SIZE as usize;
    let mut out = Vec::with_capacity(size * size * 4);

    for y in 0..size {
        for x in 0..size {
            let u = x as f32 / size as f32;
            let v = y as f32 / size as f32;
            // Domain warp keeps periodicity: the offsets are periodic in u and v,
            // and every wave below uses integer frequencies.
            let wu = 0.08 * warp[0].at(u, v);
            let wv = 0.08 * warp[1].at(u, v);
            let value = match pattern {
                Pattern::Plasma => {
                    waves.iter().map(|w| w.at(u + wu, v + wv)).sum::<f32>() / 8.0 + 0.5
                }
                Pattern::Stripes => {
                    let stripe =
                        (TAU * (checker_n * u + 3.0 * wu + 2.0 * waves[0].at(u, v) * 0.1)).sin();
                    stripe * 0.5 + 0.5
                }
                Pattern::Rings => {
                    let r = (TAU * u).cos() + (TAU * v).cos() + warp[0].at(u, v) * 0.3;
                    (ring_k * r).sin() * 0.5 + 0.5
                }
                Pattern::Cells => {
                    let blob = waves.iter().map(|w| w.at(u, v)).sum::<f32>() / 4.0 + 0.5;
                    (blob * 5.0).floor() / 5.0
                }
                Pattern::Checker => {
                    let c = (TAU * checker_n * (u + wu)).sin() * (TAU * checker_n * (v + wv)).sin();
                    let soft = c.signum() * c.abs().sqrt();
                    soft * 0.5 + 0.5
                }
                Pattern::Eyes => {
                    let n = checker_n; // eyes per tile side, an integer, so it tiles
                    let (cu, cv) = ((u * n).floor(), (v * n).floor());
                    let lu = (u * n).fract() * 2.0 - 1.0;
                    let lv = (v * n).fract() * 2.0 - 1.0;
                    // Gaze is constant per eye and periodic across tiles.
                    let look_u = 0.3 * warp[0].at((cu + 0.5) / n, (cv + 0.5) / n);
                    let look_v = 0.2 * warp[1].at((cu + 0.5) / n, (cv + 0.5) / n);
                    let lid = (1.0 - lu * lu).max(0.0) * 0.45;
                    if lv * lv >= lid {
                        0.0 // skin
                    } else {
                        let d = ((lu - look_u).powi(2) + ((lv - look_v) * 1.2).powi(2)).sqrt();
                        if d < 0.16 {
                            0.75 // pupil
                        } else if d < 0.38 {
                            0.5 // iris
                        } else {
                            0.25 // white
                        }
                    }
                }
                Pattern::Kaleido => {
                    let a = tri_wave(2.0 * u);
                    let b = tri_wave(2.0 * v);
                    let (a, b) = if a > b { (a, b) } else { (b, a) }; // mirror on the diagonal
                    waves.iter().map(|w| w.at(a * 0.5, b * 0.5)).sum::<f32>() / 8.0 + 0.5
                }
                Pattern::Swirl => {
                    let a = (TAU * u).sin();
                    let b = (TAU * v).sin();
                    let angle = b.atan2(a) / TAU;
                    let rad = (a * a + b * b).sqrt();
                    (angle * 2.0 + rad + wu).rem_euclid(1.0)
                }
            };
            let c = gradient(&stops, value * bands);
            out.extend_from_slice(&[
                c[0].round() as u8,
                c[1].round() as u8,
                c[2].round() as u8,
                255,
            ]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PALETTE: &[[u8; 3]] = &[[200, 40, 120], [40, 200, 220], [240, 220, 60]];

    fn px(tex: &[u8], x: usize, y: usize) -> [i32; 3] {
        let i = (y * TEX_SIZE as usize + x) * 4;
        [tex[i] as i32, tex[i + 1] as i32, tex[i + 2] as i32]
    }

    fn diff(a: [i32; 3], b: [i32; 3]) -> i32 {
        (0..3).map(|i| (a[i] - b[i]).abs()).max().unwrap()
    }

    #[test]
    fn size_and_alpha() {
        for p in ALL_PATTERNS {
            let t = generate(p, PALETTE, 2.0, 1);
            assert_eq!(t.len(), (TEX_SIZE * TEX_SIZE * 4) as usize);
            assert!(t.chunks(4).all(|c| c[3] == 255), "{p:?}");
        }
    }

    #[test]
    fn tiles_seamlessly() {
        let n = TEX_SIZE as usize;
        for p in ALL_PATTERNS {
            for seed in 0..10 {
                let t = generate(p, PALETTE, 2.0, seed);
                let mut interior = 0;
                for y in 0..n {
                    for x in 0..n - 1 {
                        interior = interior.max(diff(px(&t, x, y), px(&t, x + 1, y)));
                        interior = interior.max(diff(px(&t, y, x), px(&t, y, x + 1)));
                    }
                }
                let mut seam = 0;
                for i in 0..n {
                    seam = seam.max(diff(px(&t, n - 1, i), px(&t, 0, i)));
                    seam = seam.max(diff(px(&t, i, n - 1), px(&t, i, 0)));
                }
                assert!(
                    seam <= interior + 2,
                    "{p:?}/{seed}: seam jump {seam} > worst interior step {interior}"
                );
            }
        }
    }

    #[test]
    fn colours_stay_inside_the_palette_range() {
        let s = stops(PALETTE);
        for p in ALL_PATTERNS {
            let t = generate(p, PALETTE, 3.0, 5);
            for c in t.chunks(4) {
                for ch in 0..3 {
                    let lo = s.iter().map(|s| s[ch]).fold(f32::MAX, f32::min).floor() as u8;
                    let hi = s.iter().map(|s| s[ch]).fold(f32::MIN, f32::max).ceil() as u8;
                    assert!(
                        (lo..=hi).contains(&c[ch]),
                        "{p:?}: channel {ch} = {}",
                        c[ch]
                    );
                }
            }
        }
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        for p in ALL_PATTERNS {
            assert_eq!(generate(p, PALETTE, 2.0, 9), generate(p, PALETTE, 2.0, 9));
            let distinct = (0..5)
                .map(|s| generate(p, PALETTE, 2.0, s))
                .collect::<std::collections::HashSet<_>>()
                .len();
            assert!(
                distinct >= 4,
                "{p:?}: only {distinct} distinct textures in 5 seeds"
            );
        }
    }

    #[test]
    fn patterns_are_not_flat() {
        for p in ALL_PATTERNS {
            let t = generate(p, PALETTE, 2.0, 3);
            let reds: Vec<f32> = t.chunks(4).map(|c| c[0] as f32).collect();
            let mean = reds.iter().sum::<f32>() / reds.len() as f32;
            let var = reds.iter().map(|r| (r - mean).powi(2)).sum::<f32>() / reds.len() as f32;
            assert!(var.sqrt() > 20.0, "{p:?}: std dev {}", var.sqrt());
        }
    }

    #[test]
    fn single_colour_palettes_still_have_texture() {
        let t = generate(Pattern::Plasma, &[[240, 220, 180]], 2.0, 1);
        let first = &t[0..3];
        assert!(t.chunks(4).any(|c| &c[0..3] != first));
    }
}
