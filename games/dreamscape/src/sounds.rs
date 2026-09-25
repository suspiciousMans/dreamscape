//! Synthesized sound effects: no audio files, every sound is built from a
//! few oscillators, a noise source and an envelope, so each one reads
//! instantly (a rising chirp for a shard, a low thud for being caught, a
//! glassy ping for blink...). Pure: returns mono samples at `RATE`.

use std::f32::consts::TAU;

pub const RATE: u32 = 44_100;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sound {
    Jump,
    AirJump,
    Shard,
    ShardLost,
    Caught,
    MeltStart,
    WakeDoor,
    Deeper,
    Pick,
    Reroll,
    Dash,
    Blink,
    Stillness,
    Phase,
    ShardCall,
    Denied,
    Sigil,
    PortalOpen,
    SentrySaw,
    JesterThrow,
    StalkerShatter,
    Split,
    Crumble,
    Menu,
}

pub const ALL_SOUNDS: [Sound; 24] = [
    Sound::Jump,
    Sound::AirJump,
    Sound::Shard,
    Sound::ShardLost,
    Sound::Caught,
    Sound::MeltStart,
    Sound::WakeDoor,
    Sound::Deeper,
    Sound::Pick,
    Sound::Reroll,
    Sound::Dash,
    Sound::Blink,
    Sound::Stillness,
    Sound::Phase,
    Sound::ShardCall,
    Sound::Denied,
    Sound::Sigil,
    Sound::PortalOpen,
    Sound::SentrySaw,
    Sound::JesterThrow,
    Sound::StalkerShatter,
    Sound::Split,
    Sound::Crumble,
    Sound::Menu,
];

/// A tiny deterministic noise source (xorshift).
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Attack/decay envelope: quick rise, exponential fall, always ends at 0.
fn env(t: f32, len: f32, attack: f32, curve: f32) -> f32 {
    if t < attack {
        t / attack.max(1e-4)
    } else {
        let x = ((t - attack) / (len - attack).max(1e-4)).clamp(0.0, 1.0);
        (1.0 - x).powf(curve)
    }
}

/// A voice: frequency sweeps `f0 → f1`, with `wave` shape and `noise` mix.
struct Voice {
    f0: f32,
    f1: f32,
    len: f32,
    attack: f32,
    curve: f32,
    /// 0 = sine, 1 = square-ish, 2 = triangle.
    wave: u8,
    noise: f32,
    gain: f32,
    /// Start offset in seconds.
    at: f32,
}

impl Voice {
    fn new(f0: f32, f1: f32, len: f32) -> Self {
        Self {
            f0,
            f1,
            len,
            attack: 0.005,
            curve: 2.0,
            wave: 0,
            noise: 0.0,
            gain: 0.5,
            at: 0.0,
        }
    }
    fn wave(mut self, w: u8) -> Self {
        self.wave = w;
        self
    }
    fn noise(mut self, n: f32) -> Self {
        self.noise = n;
        self
    }
    fn gain(mut self, g: f32) -> Self {
        self.gain = g;
        self
    }
    fn at(mut self, t: f32) -> Self {
        self.at = t;
        self
    }
    fn curve(mut self, c: f32) -> Self {
        self.curve = c;
        self
    }
    fn attack(mut self, a: f32) -> Self {
        self.attack = a;
        self
    }
}

fn render(voices: &[Voice], seed: u32) -> Vec<f32> {
    let total = voices.iter().map(|v| v.at + v.len).fold(0.0_f32, f32::max);
    let n = (total * RATE as f32).ceil() as usize;
    let mut out = vec![0.0f32; n];
    let mut noise = Noise(seed | 1);
    for v in voices {
        let start = (v.at * RATE as f32) as usize;
        let len = (v.len * RATE as f32) as usize;
        let mut phase = 0.0f32;
        for i in 0..len {
            let t = i as f32 / RATE as f32;
            let k = t / v.len;
            // Exponential sweep sounds even across octaves.
            let f = v.f0 * (v.f1 / v.f0).powf(k);
            phase = (phase + f / RATE as f32).fract();
            let tone = match v.wave {
                1 => {
                    if phase < 0.5 {
                        0.6
                    } else {
                        -0.6
                    }
                }
                2 => 1.0 - 4.0 * (phase - 0.5).abs(),
                _ => (TAU * phase).sin(),
            };
            let s = tone * (1.0 - v.noise) + noise.next() * v.noise;
            if let Some(o) = out.get_mut(start + i) {
                *o += s * v.gain * env(t, v.len, v.attack, v.curve);
            }
        }
    }
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak > 0.95 {
        for s in &mut out {
            *s *= 0.95 / peak;
        }
    }
    out
}

pub fn synth(sound: Sound) -> Vec<f32> {
    use Sound::*;
    let v = Voice::new;
    let voices: Vec<Voice> = match sound {
        Jump => vec![v(300.0, 520.0, 0.12).wave(2).gain(0.35)],
        AirJump => vec![v(520.0, 900.0, 0.12).wave(2).gain(0.35)],
        Shard => vec![
            v(660.0, 660.0, 0.18).gain(0.35),
            v(880.0, 880.0, 0.18).gain(0.35).at(0.07),
            v(1320.0, 1320.0, 0.35).gain(0.3).at(0.14),
        ],
        ShardLost => vec![v(700.0, 180.0, 0.5).wave(2).gain(0.4)],
        Caught => vec![
            v(140.0, 50.0, 0.4).wave(1).gain(0.45),
            v(90.0, 60.0, 0.3).noise(0.6).gain(0.3),
        ],
        MeltStart => vec![v(220.0, 110.0, 1.0)
            .wave(2)
            .gain(0.3)
            .attack(0.1)
            .curve(1.0)],
        WakeDoor => vec![
            v(523.0, 523.0, 0.8).gain(0.25).attack(0.05),
            v(659.0, 659.0, 0.8).gain(0.25).attack(0.05).at(0.1),
            v(784.0, 784.0, 0.9).gain(0.25).attack(0.05).at(0.2),
        ],
        Deeper => vec![
            v(98.0, 49.0, 1.1).wave(1).gain(0.35).attack(0.05),
            v(147.0, 70.0, 1.0).gain(0.25).at(0.05),
        ],
        Pick => vec![
            v(784.0, 784.0, 0.12).wave(2).gain(0.3),
            v(1175.0, 1175.0, 0.25).wave(2).gain(0.3).at(0.08),
        ],
        Reroll => vec![
            v(400.0, 800.0, 0.08).gain(0.3),
            v(800.0, 400.0, 0.08).gain(0.3).at(0.08),
        ],
        Dash => vec![v(900.0, 200.0, 0.18).noise(0.7).gain(0.4)],
        Blink => vec![
            v(1800.0, 2400.0, 0.1).gain(0.25),
            v(2400.0, 1200.0, 0.22).gain(0.2).at(0.06),
        ],
        Stillness => vec![v(1200.0, 300.0, 0.7).wave(2).gain(0.3).curve(1.2)],
        Phase => vec![
            v(400.0, 420.0, 0.5).gain(0.25),
            v(404.0, 424.0, 0.5).gain(0.25),
        ],
        ShardCall => vec![v(300.0, 1200.0, 0.6).gain(0.3).curve(1.0)],
        Denied => vec![v(160.0, 150.0, 0.12).wave(1).gain(0.25)],
        Sigil => vec![
            v(990.0, 990.0, 0.3).gain(0.3),
            v(1485.0, 1485.0, 0.3).gain(0.2).at(0.05),
        ],
        PortalOpen => vec![
            v(330.0, 990.0, 0.9).wave(2).gain(0.3).attack(0.05),
            v(495.0, 1485.0, 0.9).gain(0.2).at(0.1),
        ],
        SentrySaw => vec![
            v(1500.0, 1500.0, 0.08).wave(1).gain(0.2),
            v(1500.0, 1500.0, 0.08).wave(1).gain(0.2).at(0.14),
        ],
        JesterThrow => vec![
            v(300.0, 1200.0, 0.1).gain(0.3),
            v(1200.0, 400.0, 0.2).wave(2).gain(0.3).at(0.1),
        ],
        StalkerShatter => vec![v(3000.0, 800.0, 0.25).noise(0.8).gain(0.35)],
        Split => vec![
            v(500.0, 250.0, 0.2).wave(1).gain(0.25),
            v(500.0, 1000.0, 0.2).wave(1).gain(0.25),
        ],
        Crumble => vec![v(200.0, 80.0, 0.5).noise(0.85).gain(0.4).curve(1.5)],
        Menu => vec![v(1000.0, 1000.0, 0.04).wave(2).gain(0.2)],
    };
    render(&voices, sound as u32 * 7919 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sound_is_short_audible_and_in_range() {
        for s in ALL_SOUNDS {
            let x = synth(s);
            let secs = x.len() as f32 / RATE as f32;
            assert!(secs > 0.03 && secs < 1.5, "{s:?}: {secs}s");
            assert!(
                x.iter().all(|v| v.is_finite() && v.abs() <= 1.0),
                "{s:?} clips"
            );
            let rms = (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt();
            assert!(rms > 0.01, "{s:?} is silent ({rms})");
        }
    }

    #[test]
    fn sounds_end_without_a_click_and_are_deterministic() {
        for s in ALL_SOUNDS {
            let x = synth(s);
            let tail = &x[x.len() - 20..];
            assert!(
                tail.iter().all(|v| v.abs() < 0.05),
                "{s:?} ends with a click"
            );
            assert_eq!(x, synth(s));
        }
    }

    #[test]
    fn sounds_are_distinct() {
        let mut seen = std::collections::HashSet::new();
        for s in ALL_SOUNDS {
            let x = synth(s);
            let sig: Vec<i32> = x.iter().step_by(441).map(|v| (v * 100.0) as i32).collect();
            assert!(seen.insert((x.len(), sig)), "{s:?} sounds like another");
        }
    }
}
