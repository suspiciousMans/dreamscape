use std::path::Path;
use std::sync::Mutex;

use sdl2::sys as sdl;

const RATE: u32 = 44_100;

/// One playing sound: mono samples at `RATE`, resampled on load.
struct Voice {
    samples: std::sync::Arc<Vec<f32>>,
    pos: usize,
    volume: f32,
    looped: bool,
}

struct Mixer {
    sfx: Vec<Voice>,
    music: Option<Voice>,
}

static MIXER: Mutex<Mixer> = Mutex::new(Mixer { sfx: Vec::new(), music: None });

fn next(v: &mut Voice) -> Option<f32> {
    if v.pos >= v.samples.len() {
        if !v.looped || v.samples.is_empty() {
            return None;
        }
        v.pos = 0;
    }
    let s = v.samples[v.pos] * v.volume;
    v.pos += 1;
    Some(s)
}

unsafe extern "C" fn callback(_: *mut std::ffi::c_void, stream: *mut u8, len: i32) {
    let out = std::slice::from_raw_parts_mut(stream as *mut f32, len as usize / 4);
    out.fill(0.0);
    let Ok(mut m) = MIXER.try_lock() else { return };
    for o in out.iter_mut() {
        let mut acc = 0.0;
        if let Some(music) = m.music.as_mut() {
            acc += next(music).unwrap_or(0.0);
        }
        for v in m.sfx.iter_mut() {
            acc += next(v).unwrap_or(0.0);
        }
        *o = acc.clamp(-1.0, 1.0);
    }
    m.sfx.retain(|v| v.pos < v.samples.len());
}

/// Downmixes interleaved samples to mono and linearly resamples to `RATE`.
fn to_mono_rate(samples: &[f32], channels: u16, rate: u32) -> Vec<f32> {
    let ch = channels.max(1) as usize;
    let mono: Vec<f32> = samples
        .chunks(ch)
        .map(|f| f.iter().sum::<f32>() / f.len() as f32)
        .collect();
    if rate == RATE || mono.is_empty() {
        return mono;
    }
    let step = rate as f64 / RATE as f64;
    let n = (mono.len() as f64 / step) as usize;
    (0..n)
        .map(|i| {
            let x = i as f64 * step;
            let i0 = x as usize;
            let i1 = (i0 + 1).min(mono.len() - 1);
            let t = (x - i0 as f64) as f32;
            mono[i0] * (1.0 - t) + mono[i1] * t
        })
        .collect()
}

/// Minimal RIFF/WAVE reader: 8/16/24/32-bit PCM and 32-bit float.
fn decode_wav(bytes: &[u8]) -> anyhow::Result<Vec<f32>> {
    anyhow::ensure!(bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE", "not a WAV file");
    let (mut fmt, mut channels, mut rate, mut bits) = (1u16, 1u16, RATE, 16u16);
    let mut i = 12;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let size = u32::from_le_bytes(bytes[i + 4..i + 8].try_into().unwrap()) as usize;
        let body = &bytes[i + 8..(i + 8 + size).min(bytes.len())];
        if id == b"fmt " && body.len() >= 16 {
            fmt = u16::from_le_bytes([body[0], body[1]]);
            channels = u16::from_le_bytes([body[2], body[3]]);
            rate = u32::from_le_bytes(body[4..8].try_into().unwrap());
            bits = u16::from_le_bytes([body[14], body[15]]);
        } else if id == b"data" {
            let samples: Vec<f32> = match (fmt, bits) {
                (3, 32) => body.chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect(),
                (_, 8) => body.iter().map(|&b| (b as f32 - 128.0) / 128.0).collect(),
                (_, 16) => body.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0).collect(),
                (_, 24) => body
                    .chunks_exact(3)
                    .map(|b| (i32::from_le_bytes([0, b[0], b[1], b[2]]) >> 8) as f32 / 8_388_608.0)
                    .collect(),
                (_, 32) => body
                    .chunks_exact(4)
                    .map(|b| i32::from_le_bytes(b.try_into().unwrap()) as f32 / 2_147_483_648.0)
                    .collect(),
                _ => anyhow::bail!("unsupported WAV format {fmt}/{bits}"),
            };
            return Ok(to_mono_rate(&samples, channels, rate));
        }
        i += 8 + size + (size & 1);
    }
    anyhow::bail!("WAV file has no data chunk")
}

/// Browser build of the engine's audio: the same API as the native `rodio`
/// wrapper, mixed in an SDL2 audio callback.
pub struct AudioContext {
    music_volume: f32,
    sfx_volume: f32,
}

impl AudioContext {
    pub fn new() -> anyhow::Result<Self> {
        unsafe {
            if sdl::SDL_InitSubSystem(sdl::SDL_INIT_AUDIO) != 0 {
                anyhow::bail!("SDL audio init failed");
            }
            let mut want: sdl::SDL_AudioSpec = std::mem::zeroed();
            want.freq = RATE as i32;
            want.format = sdl::AUDIO_F32LSB as u16;
            want.channels = 1;
            want.samples = 2048;
            want.callback = Some(callback);
            let mut have: sdl::SDL_AudioSpec = std::mem::zeroed();
            let dev = sdl::SDL_OpenAudioDevice(std::ptr::null(), 0, &want, &mut have, 0);
            anyhow::ensure!(dev != 0, "no audio device");
            sdl::SDL_PauseAudioDevice(dev, 0);
        }
        Ok(Self { music_volume: 1.0, sfx_volume: 1.0 })
    }

    fn push_sfx(&self, samples: Vec<f32>, volume: f32) {
        if samples.is_empty() || volume <= 0.0 {
            return;
        }
        if let Ok(mut m) = MIXER.lock() {
            // A burst of sounds shouldn't pile up without bound.
            if m.sfx.len() >= 24 {
                m.sfx.remove(0);
            }
            m.sfx.push(Voice { samples: std::sync::Arc::new(samples), pos: 0, volume, looped: false });
        }
    }

    pub fn play_sfx_file(&self, path: &Path) -> anyhow::Result<()> {
        let samples = decode_wav(&std::fs::read(path)?)?;
        self.push_sfx(samples, self.sfx_volume);
        Ok(())
    }

    pub fn play_tone(&self, frequency_hz: f32, duration_secs: f32) {
        let secs = if duration_secs.is_finite() { duration_secs.max(0.0) } else { 0.0 };
        let n = (secs * RATE as f32) as usize;
        let w = std::f32::consts::TAU * frequency_hz / RATE as f32;
        let samples = (0..n).map(|i| (i as f32 * w).sin() * 0.3).collect();
        self.push_sfx(samples, self.sfx_volume);
    }

    pub fn play_samples(&self, samples: &[f32], sample_rate: u32) {
        self.push_sfx(to_mono_rate(samples, 1, sample_rate), self.sfx_volume);
    }

    pub fn play_music_file(&mut self, path: &Path, looped: bool) -> anyhow::Result<()> {
        let samples = decode_wav(&std::fs::read(path)?)?;
        if let Ok(mut m) = MIXER.lock() {
            m.music = Some(Voice { samples: std::sync::Arc::new(samples), pos: 0, volume: self.music_volume, looped });
        }
        Ok(())
    }

    pub fn stop_music(&mut self) {
        if let Ok(mut m) = MIXER.lock() {
            m.music = None;
        }
    }

    pub fn set_sfx_volume(&mut self, volume: f32) {
        self.sfx_volume = volume.clamp(0.0, 1.0);
    }

    pub fn sfx_volume(&self) -> f32 {
        self.sfx_volume
    }

    pub fn set_music_volume(&mut self, volume: f32) {
        self.music_volume = volume.clamp(0.0, 1.0);
        if let Ok(mut m) = MIXER.lock() {
            if let Some(music) = m.music.as_mut() {
                music.volume = self.music_volume;
            }
        }
    }
}
