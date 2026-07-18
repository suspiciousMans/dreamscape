fn main() {
    // SDL2's Windows registry lookups (WIN_LookupAudioDeviceName, mouse system
    // scale) need advapi32, which isn't pulled in automatically for the
    // x86_64-pc-windows-gnu target.
    println!("cargo:rustc-link-lib=advapi32");
}
