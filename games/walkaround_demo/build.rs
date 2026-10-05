fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let icon_path = std::path::Path::new("icon.ico");
    if !icon_path.exists() {
        println!(
            "cargo:warning=no icon.ico next to Cargo.toml; the executable will use the default icon"
        );
        return;
    }

    let mut res = winres::WindowsResource::new();
    res.set_icon("icon.ico");
    if let Err(err) = res.compile() {
        println!("cargo:warning=failed to embed icon.ico: {err}");
    }
}
