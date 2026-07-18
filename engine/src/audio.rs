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
}

impl AudioContext {
    pub fn new() -> anyhow::Result<Self> {
        let (stream, handle) = OutputStream::try_default()?;
        Ok(Self {
            _stream: stream,
            handle,
            music: None,
        })
    }

    /// Plays a sound file once, fire-and-forget (doesn't block, doesn't need
    /// to be kept alive by the caller).
    pub fn play_sfx_file(&self, path: &Path) -> anyhow::Result<()> {
        let file = File::open(path)?;
        let source = Decoder::new(BufReader::new(file))?;
        let sink = Sink::try_new(&self.handle)?;
        sink.append(source);
        sink.detach();
        Ok(())
    }

    /// A procedural sine-wave blip — no asset file required. Handy as a
    /// placeholder SFX while prototyping (jump, interact, UI clicks, ...).
    pub fn play_tone(&self, frequency_hz: f32, duration_secs: f32) {
        let source = SineWave::new(frequency_hz)
            .take_duration(Duration::from_secs_f32(duration_secs))
            .amplify(0.3);
        if let Ok(sink) = Sink::try_new(&self.handle) {
            sink.append(source);
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

        self.music = Some(sink);
        Ok(())
    }

    pub fn stop_music(&mut self) {
        if let Some(sink) = self.music.take() {
            sink.stop();
        }
    }

    pub fn set_music_volume(&mut self, volume: f32) {
        if let Some(sink) = &self.music {
            sink.set_volume(volume);
        }
    }
}
