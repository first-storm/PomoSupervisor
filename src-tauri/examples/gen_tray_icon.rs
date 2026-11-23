//! Draws the menu-bar template icon (a tomato silhouette). Run once: cargo run --example gen_tray_icon
use image::{Rgba, RgbaImage};
fn main() {
    let n = 44u32;
    let img = RgbaImage::from_fn(n, n, |x, y| {
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        // body
        let (cx, cy, r) = (22.0, 25.0, 15.5);
        let d = ((fx - cx).powi(2) + ((fy - cy) * 1.08).powi(2)).sqrt();
        let body = (r + 0.5 - d).clamp(0.0, 1.0);
        // leaf: a small ellipse tilted on top + stem
        let leaf_l = { let (dx, dy) = (fx - 16.5, fy - 9.0); ((dx / 5.5).powi(2) + (dy / 2.4).powi(2)).sqrt() };
        let leaf_r = { let (dx, dy) = (fx - 27.5, fy - 9.0); ((dx / 5.5).powi(2) + (dy / 2.4).powi(2)).sqrt() };
        let stem = if (fx - 22.0).abs() < 1.6 && fy > 3.0 && fy < 11.0 { 1.0 } else { 0.0 };
        let leaf = ((1.0 - leaf_l) * 3.0).clamp(0.0, 1.0).max(((1.0 - leaf_r) * 3.0).clamp(0.0, 1.0)).max(stem);
        // a gap between leaves and body
        let gap = if fy > 10.5 && fy < 12.5 && d > r - 4.5 { 0.0 } else { 1.0 };
        let a = (body * gap).max(leaf);
        Rgba([0, 0, 0, (a * 255.0) as u8])
    });
    img.save("icons/tray.png").unwrap();
}
