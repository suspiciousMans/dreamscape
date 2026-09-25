fn main() {
    // SDL2's Windows registry lookups (WIN_LookupAudioDeviceName, mouse system
    // scale) need advapi32, which isn't pulled in automatically for the
    // x86_64-pc-windows-gnu target. Other platforms don't have (or need) it.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=advapi32");
    }
}
