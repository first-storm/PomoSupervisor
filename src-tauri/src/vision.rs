use base64::Engine;
use image::{DynamicImage, RgbaImage, imageops::FilterType};
use image_hasher::{HasherConfig, ImageHash};

pub struct Shot {
    pub hash: ImageHash,
    pub jpeg_b64: String,
}

/// An all-dark capture is what macOS returns without Screen Recording permission.
pub fn looks_blank(img: &RgbaImage) -> bool {
    let step = (img.width() * img.height() / 2000).max(1) as usize;
    !img.pixels().step_by(step).any(|p| p[0] > 8 || p[1] > 8 || p[2] > 8)
}

pub fn hash(img: &DynamicImage) -> ImageHash {
    HasherConfig::new().hash_size(8, 8).to_hasher().hash_image(img)
}

/// Capture the display under the point (the active window's centre), downscale, hash, encode.
pub fn capture(x: i32, y: i32) -> anyhow::Result<Shot> {
    let monitor = xcap::Monitor::from_point(x, y)
        .or_else(|_| xcap::Monitor::all().map_err(anyhow::Error::from)?.into_iter().next().ok_or(anyhow::anyhow!("没有显示器")))?;
    let raw = monitor.capture_image()?;
    if looks_blank(&raw) {
        anyhow::bail!("截图为空：请在系统设置中授予“屏幕录制”权限");
    }
    let img = DynamicImage::ImageRgba8(raw);
    let img = if img.width() > 1024 { img.resize(1024, 1024 * img.height() / img.width(), FilterType::Triangle) } else { img };
    let hash = hash(&img);
    let mut buf = Vec::new();
    let rgb = DynamicImage::ImageRgb8(img.to_rgb8());
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 70).encode_image(&rgb)?;
    Ok(Shot { hash, jpeg_b64: base64::engine::general_purpose::STANDARD.encode(buf) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn blank_detection() {
        let black = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 255]));
        assert!(looks_blank(&black));
        let mut lit = black.clone();
        for p in lit.pixels_mut().step_by(3) {
            *p = Rgba([200, 200, 200, 255]);
        }
        assert!(!looks_blank(&lit));
    }

    #[test]
    fn near_identical_images_hash_close() {
        let a = DynamicImage::ImageRgba8(RgbaImage::from_fn(200, 100, |x, y| Rgba([(x % 256) as u8, (y * 2) as u8, 50, 255])));
        let mut b = a.to_rgba8();
        b.put_pixel(10, 10, Rgba([255, 0, 0, 255]));
        let b = DynamicImage::ImageRgba8(b);
        let c = DynamicImage::ImageRgba8(RgbaImage::from_fn(200, 100, |x, y| Rgba([(y * 2) as u8, 255 - (x % 256) as u8, 50, 255])));
        assert!(hash(&a).dist(&hash(&b)) <= 6);
        assert!(hash(&a).dist(&hash(&c)) > 6);
    }
}
