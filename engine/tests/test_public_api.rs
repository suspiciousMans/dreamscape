//! Compile-time guard for engine re-exports that downstream games
//! (dungeon_oxide, Nightshot, ...) import. If one of these `use`s stops
//! resolving, this test fails to build: a breaking change caught here
//! instead of in every consumer after merge.

#[allow(unused_imports)]
use engine::{glam, glow, sdl2, ui::egui};

#[test]
fn downstream_reexports_resolve() {
    // The real check is that the `use` above compiles.
    let _ = egui::Color32::WHITE;
}
