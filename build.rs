fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        std::fs::create_dir_all("assets").expect("create assets dir");
        let path = std::path::Path::new("assets/harmonia.ico");
        if !path.exists() {
            draw_and_save_icon(path);
        }
        embed_windows_icon();
    }
}

fn draw_and_save_icon(path: &std::path::Path) {
    use image::{Rgba, RgbaImage};

    let size = 256u32;
    let mut img = RgbaImage::new(size, size);
    let cx = size as f32 / 2.0;
    let cy = size as f32 / 2.0;
    let radius = cx - 1.5;

    // ── Background: dark-violet radial gradient circle ───────────────────────
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let dx = x as f32 + 0.5 - cx;
        let dy = y as f32 + 0.5 - cy;
        let dist = (dx * dx + dy * dy).sqrt();
        let t = (dist / radius).powi(2).clamp(0.0, 1.0);
        // deep violet center → near-black edge
        let r = lerp(0x3b, 0x0f, t);
        let g = lerp(0x0d, 0x04, t);
        let b = lerp(0x6b, 0x1f, t);
        let aa = ((radius + 1.0 - dist).clamp(0.0, 1.0) * 255.0) as u8;
        *pixel = Rgba([r, g, b, aa]);
    }

    // ── Five equalizer bars forming a waveform silhouette ────────────────────
    // Heights as fraction of circle interior, tallest in the centre
    const HEIGHTS: [f32; 5] = [0.36, 0.56, 0.78, 0.56, 0.36];
    // Violet → purple-300 (bright centre) → pink gradient
    const COLORS: [[u8; 3]; 5] = [
        [0x7c, 0x3a, 0xed], // violet-600
        [0x9b, 0x27, 0xf5], // purple-500
        [0xc0, 0x84, 0xfc], // purple-300  (tallest, brightest)
        [0xd9, 0x46, 0xef], // fuchsia-500
        [0xec, 0x48, 0x99], // pink-500
    ];

    let bar_w = (size as f32 * 0.100) as u32;
    let gap   = (size as f32 * 0.040) as u32;
    let total = 5 * bar_w + 4 * gap;
    let x0    = (cx as u32).saturating_sub(total / 2);

    for i in 0..5usize {
        let bx = x0 + i as u32 * (bar_w + gap);
        let bh = (HEIGHTS[i] * radius * 1.68) as u32;
        let by = (cy as u32).saturating_sub(bh / 2);
        let [cr, cg, cb] = COLORS[i];

        for px in bx..(bx + bar_w).min(size) {
            for py in by..(by + bh).min(size) {
                let dx = px as f32 + 0.5 - cx;
                let dy = py as f32 + 0.5 - cy;
                if (dx * dx + dy * dy).sqrt() < radius - 2.0 {
                    // Subtle top-to-bottom luminance fade
                    let fade = 1.0 - (py - by) as f32 / bh as f32 * 0.25;
                    img.put_pixel(px, py, Rgba([
                        (cr as f32 * fade) as u8,
                        (cg as f32 * fade) as u8,
                        (cb as f32 * fade) as u8,
                        255,
                    ]));
                }
            }
        }
    }

    img.save(path).expect("failed to save icon");
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}

fn embed_windows_icon() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/harmonia.ico");
    if let Err(e) = res.compile() {
        // Non-fatal: app still works, just won't have a custom icon
        eprintln!("cargo:warning=winresource: {e}");
    }
}
