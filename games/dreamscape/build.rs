// Browser (Emscripten) build only: bundle the game's assets into the page's
// virtual filesystem at the same relative paths the native build reads from
// (cwd is "/"), and mount a persistent save folder (see web/pre.js).
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=web/pre.js");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("emscripten") {
        return;
    }
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    for arg in [
        format!("--preload-file={dir}/assets@/games/dreamscape/assets"),
        format!("--preload-file={dir}/profiles@/games/dreamscape/profiles"),
        format!("--pre-js={dir}/web/pre.js"),
        "-lidbfs.js".into(),
        "-sFORCE_FILESYSTEM=1".into(),
        "-sEXPORTED_RUNTIME_METHODS=FS".into(),
        "-sSTACK_SIZE=4MB".into(),
        // SDL2's audio backend calls these from EM_ASM.
        "-sDEFAULT_LIBRARY_FUNCS_TO_INCLUDE=$autoResumeAudioContext,$dynCall".into(),
    ] {
        println!("cargo:rustc-link-arg-bin=dreamscape={arg}");
    }
}
