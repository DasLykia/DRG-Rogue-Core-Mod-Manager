// Turns assets/icon.png into the .exe icon (and assets/app.ico for the Velopack installer).
include!("src/icon.rs");

fn main() {
    println!("cargo:rerun-if-changed=src/icon.rs");
    println!("cargo:rerun-if-changed=assets/icon.png");
    let ico = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("assets").join("app.ico");
    image::save_buffer(&ico, &icon_rgba(256), 256, 256, image::ColorType::Rgba8).expect("write icon");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") || std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("gnu") {
        return;
    }
    // shortcut: windres's preprocessor splits paths that contain spaces, so it runs in the
    // temp folder, which has none on this PC; switch to an .rc-free resource writer if that changes.
    let tmp = std::env::temp_dir().join("cmm-icon-build");
    std::fs::create_dir_all(&tmp).expect("temp folder");
    std::fs::copy(&ico, tmp.join("app.ico")).expect("copy icon");
    std::fs::write(tmp.join("app.rc"), "1 ICON \"app.ico\"\n").expect("write rc");
    let ok = std::process::Command::new("windres")
        .current_dir(&tmp)
        .args(["app.rc", "-O", "coff", "-o", "app_icon.o"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        println!("cargo:warning=windres not available: the .exe gets no icon (the window still has one)");
        return;
    }
    let obj = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("app_icon.o");
    std::fs::copy(tmp.join("app_icon.o"), &obj).expect("copy icon object");
    println!("cargo:rustc-link-arg-bins={}", obj.display());
}
