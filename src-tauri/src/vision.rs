use base64::Engine;
use image::{DynamicImage, RgbaImage, imageops::FilterType};
use image_hasher::{HasherConfig, ImageHash};

pub struct Shot {
    pub hash: ImageHash,
    pub jpeg_b64: String,
    pub width: u32,
    pub height: u32,
}

/// Longest side at most this; decision APIs reject sides over 1568 and don't resize.
const MAX_SIDE: u32 = 1024;

/// Downscale so both sides fit in MAX_SIDE, keeping the aspect (portrait displays too).
pub fn fit(img: DynamicImage) -> DynamicImage {
    if img.width() > MAX_SIDE || img.height() > MAX_SIDE { img.resize(MAX_SIDE, MAX_SIDE, FilterType::Triangle) } else { img }
}

pub fn encode_jpeg(img: &DynamicImage) -> anyhow::Result<String> {
    let mut buf = Vec::new();
    let rgb = DynamicImage::ImageRgb8(img.to_rgb8());
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 70).encode_image(&rgb)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(buf))
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
        .or_else(|_| xcap::Monitor::all().map_err(anyhow::Error::from)?.into_iter().next().ok_or(anyhow::anyhow!(crate::i18n::tr().err_no_display)))?;
    let raw = monitor.capture_image()?;
    if looks_blank(&raw) {
        anyhow::bail!(crate::i18n::tr().err_blank_shot);
    }
    let img = fit(DynamicImage::ImageRgba8(raw));
    Ok(Shot { hash: hash(&img), jpeg_b64: encode_jpeg(&img)?, width: img.width(), height: img.height() })
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
    fn fit_bounds_both_sides() {
        let portrait = fit(DynamicImage::ImageRgba8(RgbaImage::new(1440, 2560)));
        assert_eq!(portrait.height(), 1024);
        assert!(portrait.width() >= 280 && portrait.width() < 1024);
        let wide = fit(DynamicImage::ImageRgba8(RgbaImage::new(3024, 1964)));
        assert_eq!(wide.width(), 1024);
        let small = fit(DynamicImage::ImageRgba8(RgbaImage::new(800, 600)));
        assert_eq!((small.width(), small.height()), (800, 600));
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
