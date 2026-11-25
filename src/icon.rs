// The app icon, drawn from shapes: a gold pickaxe on a dark rounded square.
// Plain std only, because build.rs includes this file to write the .exe icon.

/// RGBA pixels (straight alpha), `size` × `size`.
pub fn icon_rgba(size: u32) -> Vec<u8> {
    let s = size as f32;
    let aa = 1.2 / s;
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    // Handle runs from bottom-left up to the head; the head is an arc around a point below it.
    let (h0, h1) = ((0.27f32, 0.79f32), (0.64f32, 0.38f32));
    let len = ((h1.0 - h0.0).powi(2) + (h1.1 - h0.1).powi(2)).sqrt();
    let u = ((h1.0 - h0.0) / len, (h1.1 - h0.1) / len);
    let r = 0.40f32;
    let c = (h1.0 - u.0 * r, h1.1 - u.1 * r);
    let half = 52f32.to_radians();
    for y in 0..size {
        for x in 0..size {
            let p = ((x as f32 + 0.5) / s, (y as f32 + 0.5) / s);
            let mut px = [0f32; 4];
            // Background: rounded square with a soft vertical gradient and a thin gold rim.
            let d_bg = round_rect(p, 0.46, 0.2);
            let bg = mix([62.0, 53.0, 48.0], [26.0, 22.0, 20.0], p.1);
            over(&mut px, bg, cover(d_bg, aa));
            over(&mut px, [217.0, 183.0, 104.0], cover((d_bg + 0.03).abs() - 0.008, aa) * 0.7);
            // Handle.
            over(&mut px, [168.0, 122.0, 70.0], cover(segment(p, h0, h1) - 0.045, aa));
            // Head: thick in the middle, pointed at both tips.
            let (dx, dy) = (p.0 - c.0, p.1 - c.1);
            let dist = (dx * dx + dy * dy).sqrt().max(1e-6);
            let ang = ((dx * u.0 + dy * u.1) / dist).clamp(-1.0, 1.0).acos();
            let k = (ang / half).min(1.0);
            let thick = 0.075 * (1.0 - k * k) + 0.006;
            let d_head = ((dist - r).abs() - thick).max((ang - half) * r);
            over(&mut px, [239.0, 224.0, 190.0], cover(d_head, aa));
            out.extend(px.iter().map(|v| v.round().clamp(0.0, 255.0) as u8));
        }
    }
    out
}

fn round_rect(p: (f32, f32), half: f32, rad: f32) -> f32 {
    let q = ((p.0 - 0.5).abs() - half + rad, (p.1 - 0.5).abs() - half + rad);
    (q.0.max(0.0).powi(2) + q.1.max(0.0).powi(2)).sqrt() + q.0.max(q.1).min(0.0) - rad
}

fn segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (pa, ba) = ((p.0 - a.0, p.1 - a.1), (b.0 - a.0, b.1 - a.1));
    let h = ((pa.0 * ba.0 + pa.1 * ba.1) / (ba.0 * ba.0 + ba.1 * ba.1)).clamp(0.0, 1.0);
    ((pa.0 - ba.0 * h).powi(2) + (pa.1 - ba.1 * h).powi(2)).sqrt()
}

fn cover(d: f32, aa: f32) -> f32 {
    (0.5 - d / aa).clamp(0.0, 1.0)
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Paints `col` with coverage `a` over `px` (straight alpha, 0..255 channels).
fn over(px: &mut [f32; 4], col: [f32; 3], a: f32) {
    if a <= 0.0 {
        return;
    }
    let below = px[3] / 255.0;
    let out_a = a + below * (1.0 - a);
    for i in 0..3 {
        px[i] = (col[i] * a + px[i] * below * (1.0 - a)) / out_a;
    }
    px[3] = out_a * 255.0;
}
