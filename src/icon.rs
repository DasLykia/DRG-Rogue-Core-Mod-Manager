// The app icon: assets/icon.png (the person's logo), cut to its round badge.
// Plain std + image, because build.rs includes this file to write the .exe icon.

/// RGBA pixels (straight alpha), `size` × `size`.
pub fn icon_rgba(size: u32) -> Vec<u8> {
    let png = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icon.png"));
    let img = image::load_from_memory(png).expect("assets/icon.png is a valid picture").to_rgba8();
    let mut img = image::imageops::resize(&img, size, size, image::imageops::FilterType::Lanczos3);
    // The logo is a round badge on black: everything outside its outer ring becomes transparent.
    let c = size as f32 / 2.0;
    let r = size as f32 * 0.462;
    for (x, y, p) in img.enumerate_pixels_mut() {
        let d = ((x as f32 + 0.5 - c).powi(2) + (y as f32 + 0.5 - c).powi(2)).sqrt();
        let a = (r - d + 0.5).clamp(0.0, 1.0);
        p[3] = (p[3] as f32 * a) as u8;
    }
    img.into_raw()
}
