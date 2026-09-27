# Dreamscape audio

No audio files. Sound effects are synthesized in `src/sounds.rs`; music is
composed per dream in `src/music.rs` (a style per theme, a score seeded by the
dream, rendered to a seamless loop on a worker thread) and played with
`AudioContext::play_music_samples`, offset to the dream's clock so dreams that
pulse (beat tiles) stay on the beat.

Listen to every theme: `cargo test -p dreamscape write_music_previews -- --ignored --nocapture`
(WAVs land in the system temp folder under `dreamscape_music/`).
