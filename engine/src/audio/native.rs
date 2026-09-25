use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

use rodio::source::{SineWave, Source};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

/// Thin wrapper around `rodio` for one-shot SFX, a procedural placeholder
/// tone (no asset file needed), and a single background-music track.
pub struct AudioContext {
    // Must stay alive for the duration of playback — dropping it silences
    // everything, even sounds already `detach()`-ed.
    _stream: OutputStream,
    handle: OutputStreamHandle,
    music: Option<Sink>,
    music_volume: f32,
    sfx_volume: f32,
}

impl AudioContext {
    pub fn new() -> anyhow::Result<Self> {
        let (stream, handle) = OutputStream::try_default()?;
        Ok(Self {
            _stream: stream,
            handle,
            music: None,
            music_volume: 1.0,
            sfx_volume: 1.0,
        })
    }

    /// Plays a sound file once, fire-and-forget (doesn't block, doesn't need
    /// to be kept alive by the caller).
    pub fn play_sfx_file(&self, path: &Path) -> anyhow::Result<()> {
        let file = File::open(path)?;
        let source = Decoder::new(BufReader::new(file))?;
        let sink = Sink::try_new(&self.handle)?;
        sink.set_volume(self.sfx_volume);
        sink.append(source);
        sink.detach();
        Ok(())
    }

    /// A procedural sine-wave blip — no asset file required. Handy as a
    /// placeholder SFX while prototyping (jump, interact, UI clicks, ...).
    pub fn play_tone(&self, frequency_hz: f32, duration_secs: f32) {
        // `Duration::from_secs_f32` panics on a negative, NaN, or infinite
        // argument. This is a game-/script-reachable API, so a duration
        // that comes out of a bad division or a designer typo would
        // otherwise crash the whole single-threaded engine mid-frame
        // instead of just skipping the blip — sanitize to a finite,
        // non-negative value first (and skip a zero-length source).
        let secs = if duration_secs.is_finite() {
            duration_secs.max(0.0)
        } else {
            0.0
        };
        if secs <= 0.0 {
            return;
        }
        let source = SineWave::new(frequency_hz)
            .take_duration(Duration::from_secs_f32(secs))
            .amplify(0.3 * self.sfx_volume);
        if let Ok(sink) = Sink::try_new(&self.handle) {
            sink.append(source);
            sink.detach();
        }
    }

    /// Plays raw mono samples (-1..=1) once, at the sfx volume — for sounds
    /// a game synthesizes itself.
    pub fn play_samples(&self, samples: &[f32], sample_rate: u32) {
        if samples.is_empty() || self.sfx_volume <= 0.0 {
            return;
        }
        let buffer = rodio::buffer::SamplesBuffer::new(1, sample_rate, samples.to_vec());
        if let Ok(sink) = Sink::try_new(&self.handle) {
            sink.set_volume(self.sfx_volume);
            sink.append(buffer);
            sink.detach();
        }
    }

    /// Plays (and replaces any currently-playing) background music from a
    /// file. Looping buffers the fully-decoded track in memory so it can be
    /// cheaply repeated — fine for typical music-track lengths.
    pub fn play_music_file(&mut self, path: &Path, looped: bool) -> anyhow::Result<()> {
        let file = File::open(path)?;
        let source = Decoder::new(BufReader::new(file))?;
        let sink = Sink::try_new(&self.handle)?;

        if looped {
            let channels = source.channels();
            let sample_rate = source.sample_rate();
            let samples: Vec<f32> = source.convert_samples().collect();
            let buffer = rodio::buffer::SamplesBuffer::new(channels, sample_rate, samples);
            sink.append(buffer.repeat_infinite());
        } else {
            sink.append(source);
        }

        sink.set_volume(self.music_volume);
        self.music = Some(sink);
        Ok(())
    }

    pub fn stop_music(&mut self) {
        if let Some(sink) = self.music.take() {
            sink.stop();
        }
    }

    /// Volume for `play_tone` and `play_sfx_file` (0..=1).
    pub fn set_sfx_volume(&mut self, volume: f32) {
        self.sfx_volume = volume.clamp(0.0, 1.0);
    }

    pub fn sfx_volume(&self) -> f32 {
        self.sfx_volume
    }

    /// The handle, for games that build their own sources.
    pub fn handle(&self) -> &OutputStreamHandle {
        &self.handle
    }

    pub fn set_music_volume(&mut self, volume: f32) {
        self.music_volume = volume.clamp(0.0, 1.0);
        if let Some(sink) = &self.music {
            sink.set_volume(volume);
        }
    }
}
