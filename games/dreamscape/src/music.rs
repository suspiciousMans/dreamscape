//! Procedural music. Every dream plays its own loop, composed from the
//! dream's seed in its theme's style (key, mode, tempo, instruments), and it
//! bends out of key the deeper you go. Pure: `score` makes notes, `render`
//! makes mono samples at `sounds::RATE`; nothing is loaded from disk.

use crate::dream::DreamTheme;
use crate::sounds::RATE;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::TAU;

/// Bars in one loop.
pub const BARS: u32 = 8;
/// Output peak: music sits under the sound effects.
pub const PEAK: f32 = 0.7;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scale {
    Major,
    Minor,
    Dorian,
    Phrygian,
    Lydian,
    Pentatonic,
    WholeTone,
    HarmonicMinor,
}

impl Scale {
    pub fn steps(self) -> &'static [i32] {
        match self {
            Scale::Major => &[0, 2, 4, 5, 7, 9, 11],
            Scale::Minor => &[0, 2, 3, 5, 7, 8, 10],
            Scale::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            Scale::Phrygian => &[0, 1, 3, 5, 7, 8, 10],
            Scale::Lydian => &[0, 2, 4, 6, 7, 9, 11],
            Scale::Pentatonic => &[0, 2, 4, 7, 9],
            Scale::WholeTone => &[0, 2, 4, 6, 8, 10],
            Scale::HarmonicMinor => &[0, 2, 3, 5, 7, 8, 11],
        }
    }

    /// MIDI note of scale degree `d` above `root` (any integer: wraps into octaves).
    pub fn note(self, root: i32, d: i32) -> i32 {
        let s = self.steps();
        let n = s.len() as i32;
        root + 12 * d.div_euclid(n) + s[d.rem_euclid(n) as usize]
    }

    pub fn contains(self, root: i32, midi: i32) -> bool {
        self.steps().contains(&(midi - root).rem_euclid(12))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Instr {
    Pad,
    Bass,
    Arp,
    Bell,
    Kick,
    Hat,
}

/// How a theme sounds. Amounts are 0..=1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    /// MIDI root of the key, in the bass octave.
    pub root: i32,
    pub scale: Scale,
    pub bpm: f32,
    /// Chance per eighth note of an arpeggio note.
    pub arp: f32,
    /// Chance per beat of a high bell.
    pub bells: f32,
    pub drums: bool,
    /// Low-pass openness: 0 = muffled, 1 = bright.
    pub bright: f32,
    /// Echo feedback.
    pub echo: f32,
}

/// Each dream's key, mode, tempo and mix. Dreams that pulse (beat tiles,
/// breathing to a bpm) always play at their own pulse.
pub fn style(theme: DreamTheme) -> Style {
    use DreamTheme::*;
    use Scale::*;
    #[allow(clippy::too_many_arguments)]
    fn s(
        root: i32,
        scale: Scale,
        bpm: f32,
        arp: f32,
        bells: f32,
        drums: bool,
        bright: f32,
        echo: f32,
    ) -> Style {
        Style {
            root,
            scale,
            bpm,
            arp,
            bells,
            drums,
            bright,
            echo,
        }
    }
    let mut st = match theme {
        Lobby => s(45, Major, 70.0, 0.25, 0.10, false, 0.5, 0.35),
        LiminalOffice => s(43, Dorian, 84.0, 0.15, 0.05, false, 0.35, 0.2),
        VoidPlatforms => s(40, Lydian, 64.0, 0.10, 0.25, false, 0.6, 0.55),
        Garden => s(48, Pentatonic, 78.0, 0.45, 0.20, false, 0.7, 0.3),
        NightmareFactory => s(38, Phrygian, 96.0, 0.30, 0.0, true, 0.4, 0.15),
        Awakening => s(50, Major, 60.0, 0.20, 0.35, false, 0.8, 0.45),
        CursedForest => s(41, HarmonicMinor, 74.0, 0.25, 0.05, false, 0.3, 0.4),
        DrownedLibrary => s(39, Minor, 58.0, 0.10, 0.30, false, 0.25, 0.6),
        SkyStairs => s(47, Lydian, 88.0, 0.50, 0.15, false, 0.85, 0.35),
        MirrorHall => s(44, WholeTone, 66.0, 0.35, 0.20, false, 0.6, 0.5),
        MyceliumGrove => s(42, Dorian, 76.0, 0.40, 0.10, true, 0.45, 0.4),
        TheTunnel => s(36, Minor, 90.0, 0.20, 0.0, true, 0.2, 0.25),
        FractalCathedral => s(46, Minor, 72.0, 0.30, 0.40, false, 0.55, 0.6),
        Elfworks => s(49, Major, 128.0, 0.70, 0.25, true, 0.9, 0.2),
        AfterimageFields => s(45, Lydian, 68.0, 0.30, 0.20, false, 0.5, 0.75),
        SynesthesiaHall => s(43, Pentatonic, 110.0, 0.60, 0.30, true, 0.8, 0.3),
        MeltingClockworks => s(40, WholeTone, 60.0, 0.45, 0.15, true, 0.4, 0.35),
        JellyfishSky => s(48, Lydian, 56.0, 0.20, 0.45, false, 0.65, 0.7),
        WatchingWallpaper => s(41, Phrygian, 62.0, 0.15, 0.10, false, 0.3, 0.5),
        WhiteDissolve => s(47, WholeTone, 50.0, 0.05, 0.50, false, 1.0, 0.8),
    };
    let pulse = theme.spec().mood.bpm;
    if pulse > 0.0 {
        st.bpm = pulse;
    }
    st
}

/// One note. Times are in beats.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Note {
    pub beat: f32,
    pub beats: f32,
    pub midi: i32,
    pub instr: Instr,
    pub vel: f32,
}

/// The notes of one `bars`-bar loop, fixed by `seed`. `strange` (0..=1, the
/// dream's strangeness) swaps some melody notes for out-of-key neighbours.
pub fn score(st: &Style, strange: f32, seed: u64, bars: u32) -> Vec<Note> {
    let mut rng = StdRng::seed_from_u64(seed ^ 0x6D75_7369_6321);
    const CHORD_ROOTS: [i32; 6] = [0, 3, 4, 5, 1, 2];
    let chords: Vec<i32> = std::iter::once(0)
        .chain((0..3).map(|_| CHORD_ROOTS[rng.gen_range(0..CHORD_ROOTS.len())]))
        .collect();
    let bend = |rng: &mut StdRng, midi: i32| {
        if rng.gen::<f32>() < strange * 0.2 {
            midi + if rng.gen_bool(0.5) { 1 } else { -1 }
        } else {
            midi
        }
    };
    let total = bars as f32 * 4.0;
    let span = total / chords.len() as f32;
    let mut out = Vec::new();
    for (k, &c) in chords.iter().enumerate() {
        let start = k as f32 * span;
        // Pad: the chord, held for its whole span.
        for step in [0, 2, 4] {
            out.push(Note {
                beat: start,
                beats: span,
                midi: st.scale.note(st.root + 24, c + step),
                instr: Instr::Pad,
                vel: 0.5,
            });
        }
        // Bass: the chord root every other beat (every beat with drums).
        let every = if st.drums { 1.0 } else { 2.0 };
        let mut b = 0.0;
        while b < span {
            out.push(Note {
                beat: start + b,
                beats: every * 0.9,
                midi: st.scale.note(st.root, c),
                instr: Instr::Bass,
                vel: 0.7,
            });
            b += every;
        }
        // Arpeggio: eighth notes over the chord, an octave up.
        for e in 0..(span * 2.0) as i32 {
            if rng.gen::<f32>() < st.arp {
                let tone = [0, 2, 4, 7][rng.gen_range(0..4)];
                let midi = bend(&mut rng, st.scale.note(st.root + 24, c + tone));
                let vel = 0.35 + 0.2 * rng.gen::<f32>();
                out.push(Note {
                    beat: start + e as f32 * 0.5,
                    beats: 0.45,
                    midi,
                    instr: Instr::Arp,
                    vel,
                });
            }
        }
    }
    // Bells: rare high notes anywhere in the key.
    for beat in 0..total as i32 {
        if rng.gen::<f32>() < st.bells {
            let d = rng.gen_range(0..7);
            let midi = bend(&mut rng, st.scale.note(st.root + 36, d));
            let off = if rng.gen_bool(0.3) { 0.5 } else { 0.0 };
            out.push(Note {
                beat: beat as f32 + off,
                beats: 3.0,
                midi,
                instr: Instr::Bell,
                vel: 0.3,
            });
        }
    }
    if st.drums {
        for beat in 0..total as i32 {
            if beat % 2 == 0 {
                out.push(Note {
                    beat: beat as f32,
                    beats: 0.25,
                    midi: 0,
                    instr: Instr::Kick,
                    vel: 0.9,
                });
            }
            out.push(Note {
                beat: beat as f32 + 0.5,
                beats: 0.05,
                midi: 0,
                instr: Instr::Hat,
                vel: 0.5,
            });
        }
    }
    out
}

/// Seconds in one `bars`-bar loop.
pub fn loop_secs(st: &Style, bars: u32) -> f32 {
    bars as f32 * 4.0 * 60.0 / st.bpm
}

fn hz(midi: i32) -> f32 {
    440.0 * 2f32.powf((midi - 69) as f32 / 12.0)
}

fn tri(phase: f32) -> f32 {
    1.0 - 4.0 * (phase - 0.5).abs()
}

/// (attack s, release s, exponential decay /s, gain) per instrument.
fn shape(i: Instr) -> (f32, f32, f32, f32) {
    match i {
        // Balanced for small speakers (see `audible_on_small_speakers`): the
        // bass and kick are felt, the pad and melody carry the mix.
        Instr::Pad => (0.8, 1.2, 0.0, 0.2),
        Instr::Bass => (0.01, 0.12, 0.8, 0.14),
        Instr::Arp => (0.005, 0.25, 5.0, 0.26),
        Instr::Bell => (0.002, 2.5, 1.2, 0.16),
        Instr::Kick => (0.001, 0.25, 12.0, 0.3),
        Instr::Hat => (0.001, 0.05, 40.0, 0.1),
    }
}

fn envelope(t: f32, hold: f32, attack: f32, release: f32) -> f32 {
    if t < attack {
        t / attack
    } else if t < hold.max(attack) {
        1.0
    } else {
        (1.0 - (t - hold.max(attack)) / release).max(0.0)
    }
}

/// The loop as mono samples. Everything wraps round the end of the buffer
/// (note tails, filter, echo), so playing it on repeat has no seam.
pub fn render(st: &Style, notes: &[Note], strange: f32, bars: u32) -> Vec<f32> {
    let n = (loop_secs(st, bars) * RATE as f32).round() as usize;
    let mut out = vec![0.0_f32; n];
    if n == 0 {
        return out;
    }
    let spb = 60.0 / st.bpm;
    let dt = 1.0 / RATE as f32;
    let mut noise = 0x9E37_79B9_u32;
    for note in notes {
        let (attack, release, decay, gain) = shape(note.instr);
        let hold = note.beats * spb;
        let len = ((hold.max(attack) + release) * RATE as f32) as usize;
        let start = (note.beat * spb * RATE as f32) as usize;
        let f = hz(note.midi);
        // Deep dreams: a slow, seasick pitch wobble.
        let wobble = strange * 0.012;
        let (mut p1, mut p2, mut p3, mut last) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
        for i in 0..len {
            let t = i as f32 * dt;
            let env = envelope(t, hold, attack, release) * (-decay * t).exp();
            let fw = match note.instr {
                Instr::Kick => 45.0 + 90.0 * (-t * 25.0).exp(),
                _ => f * (1.0 + wobble * (TAU * 0.3 * t).sin()),
            };
            p1 = (p1 + fw * dt).fract();
            p2 = (p2 + fw * 1.004 * dt).fract();
            p3 = (p3 + fw * 2.76 * dt).fract();
            let s = match note.instr {
                Instr::Pad => 0.5 * (tri(p1) + tri(p2)),
                Instr::Bass => tri(p1),
                Instr::Arp => 0.7 * (TAU * p1).sin() + 0.3 * if p1 < 0.5 { 1.0 } else { -1.0 },
                Instr::Bell => (TAU * p1).sin() + 0.4 * (TAU * p3).sin(),
                Instr::Kick => (TAU * p1).sin(),
                Instr::Hat => {
                    noise ^= noise << 13;
                    noise ^= noise >> 17;
                    noise ^= noise << 5;
                    let x = noise as f32 / u32::MAX as f32 * 2.0 - 1.0;
                    let hp = x - last;
                    last = x;
                    hp * 0.5
                }
            };
            out[(start + i) % n] += s * env * note.vel * gain;
        }
    }
    lowpass(&mut out, 600.0 * (1.0 + 14.0 * st.bright));
    echo(&mut out, (0.75 * spb * RATE as f32) as usize, st.echo * 0.5);
    let peak = out.iter().fold(0.0_f32, |m, x| m.max(x.abs()));
    if peak > 0.0 {
        for x in &mut out {
            *x *= PEAK / peak;
        }
    }
    out
}

/// One-pole low-pass, run round the loop twice: the first lap only warms
/// the filter up, so the wrap point has no step.
fn lowpass(buf: &mut [f32], cutoff: f32) {
    let a = 1.0 - (-TAU * cutoff / RATE as f32).exp();
    let mut y = 0.0;
    for lap in 0..2 {
        for x in buf.iter_mut() {
            y += a * (*x - y);
            if lap == 1 {
                *x = y;
            }
        }
    }
}

/// Feedback echo, circular: three laps settle the tail across the seam.
fn echo(buf: &mut [f32], delay: usize, feedback: f32) {
    let n = buf.len();
    if feedback <= 0.0 || delay == 0 || delay >= n {
        return;
    }
    let mut wet = vec![0.0_f32; n];
    for _ in 0..3 {
        for i in 0..n {
            let j = (i + n - delay) % n;
            wet[i] = buf[j] + feedback * wet[j];
        }
    }
    for (x, w) in buf.iter_mut().zip(&wet) {
        *x += 0.5 * w;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::{DreamTheme, ALL_THEMES};
    use std::collections::HashSet;

    fn pitched(n: &Note) -> bool {
        !matches!(n.instr, Instr::Kick | Instr::Hat)
    }

    #[test]
    fn every_dream_sounds_different() {
        let set: HashSet<String> = ALL_THEMES
            .iter()
            .map(|&t| {
                let s = style(t);
                format!("{} {:?} {}", s.root, s.scale, s.bpm as i32)
            })
            .collect();
        assert_eq!(set.len(), ALL_THEMES.len());
    }

    #[test]
    fn pulsing_dreams_play_on_their_pulse() {
        for t in ALL_THEMES {
            let bpm = t.spec().mood.bpm;
            if bpm > 0.0 {
                assert_eq!(style(t).bpm, bpm, "{t:?}");
            }
        }
    }

    #[test]
    fn nightmares_are_dark_and_waking_is_bright() {
        let n = style(DreamTheme::NightmareFactory);
        assert!(matches!(
            n.scale,
            Scale::Phrygian | Scale::Minor | Scale::HarmonicMinor
        ));
        assert!(n.drums);
        assert!(matches!(
            style(DreamTheme::Awakening).scale,
            Scale::Major | Scale::Lydian
        ));
    }

    #[test]
    fn a_score_is_fixed_by_its_seed() {
        let st = style(DreamTheme::Garden);
        assert_eq!(score(&st, 0.3, 7, BARS), score(&st, 0.3, 7, BARS));
        assert_ne!(score(&st, 0.3, 7, BARS), score(&st, 0.3, 8, BARS));
    }

    #[test]
    fn calm_dreams_stay_in_key_and_deep_ones_bend() {
        for t in ALL_THEMES {
            let st = style(t);
            for seed in 0..10 {
                for n in score(&st, 0.0, seed, BARS).iter().filter(|n| pitched(n)) {
                    assert!(
                        st.scale.contains(st.root, n.midi),
                        "{t:?} seed {seed}: {n:?}"
                    );
                }
            }
        }
        let st = style(DreamTheme::Garden);
        let bent = |strange: f32| {
            (0..30)
                .flat_map(|s| score(&st, strange, s, BARS))
                .filter(|n| pitched(n) && !st.scale.contains(st.root, n.midi))
                .count()
        };
        assert_eq!(bent(0.0), 0);
        assert!(
            bent(1.0) > 10,
            "deep dreams should sound wrong: {}",
            bent(1.0)
        );
    }

    #[test]
    fn a_loop_is_whole_bars_loud_enough_and_never_clips() {
        for t in ALL_THEMES {
            let st = style(t);
            let s = render(&st, &score(&st, 0.5, 7, 2), 0.5, 2);
            assert_eq!(
                s.len(),
                (loop_secs(&st, 2) * RATE as f32).round() as usize,
                "{t:?}"
            );
            let peak = s.iter().fold(0.0_f32, |m, x| m.max(x.abs()));
            assert!(
                peak <= PEAK + 1e-4 && peak > 0.3 * PEAK,
                "{t:?} peak {peak}"
            );
            let rms = (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt();
            assert!(rms > 0.03, "{t:?} too quiet: {rms}");
        }
    }

    #[test]
    fn audible_on_small_speakers() {
        // Laptop and Steam Deck speakers barely play below ~150 Hz: most of
        // the loudness has to live above that, or a dream sounds silent.
        for t in ALL_THEMES {
            let st = style(t);
            let s = render(&st, &score(&st, 0.3, 1, 2), 0.3, 2);
            // Energy left after a 150 Hz one-pole high-pass, vs the total.
            let a = (-std::f32::consts::TAU * 150.0 / RATE as f32).exp();
            let (mut lp, mut hi, mut all) = (0.0_f32, 0.0_f32, 0.0_f32);
            for &x in &s {
                lp = a * lp + (1.0 - a) * x;
                hi += (x - lp) * (x - lp);
                all += x * x;
            }
            assert!(
                hi / all > 0.5,
                "{t:?}: only {:.0}% above 150 Hz",
                100.0 * hi / all
            );
        }
    }

    #[test]
    fn the_loop_has_no_click_where_it_repeats() {
        for t in [
            DreamTheme::Lobby,
            DreamTheme::Elfworks,
            DreamTheme::WhiteDissolve,
        ] {
            let st = style(t);
            let s = render(&st, &score(&st, 0.2, 3, 2), 0.2, 2);
            let jump = (s[0] - s[s.len() - 1]).abs();
            let mut steps: Vec<f32> = s.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
            steps.sort_by(f32::total_cmp);
            let p99 = steps[steps.len() * 99 / 100];
            assert!(
                jump <= p99 * 1.5 + 1e-3,
                "{t:?}: seam {jump} vs p99 step {p99}"
            );
        }
    }

    /// Writes every theme's loop to WAV so a human can listen:
    /// cargo test -p dreamscape write_music_previews -- --ignored
    #[test]
    #[ignore]
    fn write_music_previews() {
        let dir = std::env::temp_dir().join("dreamscape_music");
        std::fs::create_dir_all(&dir).unwrap();
        for t in ALL_THEMES {
            let st = style(t);
            let s = render(&st, &score(&st, 0.3, 1, BARS), 0.3, BARS);
            let path = dir.join(format!("{t:?}.wav"));
            std::fs::write(&path, wav16(&s)).unwrap();
            println!("{}", path.display());
        }
    }

    /// Mono 16-bit PCM WAV bytes.
    fn wav16(s: &[f32]) -> Vec<u8> {
        let data = (s.len() * 2) as u32;
        let mut b = Vec::with_capacity(44 + data as usize);
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + data).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes()); // PCM
        b.extend_from_slice(&1u16.to_le_bytes()); // mono
        b.extend_from_slice(&RATE.to_le_bytes());
        b.extend_from_slice(&(RATE * 2).to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&data.to_le_bytes());
        for x in s {
            b.extend_from_slice(&((x.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
        }
        b
    }
}
