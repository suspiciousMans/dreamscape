//! Audio playback. Native builds use `rodio`; the Emscripten (browser) build
//! has no `cpal` backend, so it mixes into an SDL2 audio device instead,
//! with the same `AudioContext` API.

#[cfg(not(target_os = "emscripten"))]
mod native;
#[cfg(not(target_os = "emscripten"))]
pub use native::AudioContext;

#[cfg(target_os = "emscripten")]
mod web;
#[cfg(target_os = "emscripten")]
pub use web::AudioContext;
