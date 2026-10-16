use base64::Engine;
use image::{Rgba, RgbaImage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}

pub fn pixel_matches(actual: [u8; 3], expected: [u8; 3], tolerance: u8) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(channel, want)| channel.abs_diff(want) <= tolerance)
}

pub fn decode_png_base64(data: &str) -> Result<RgbaImage, String> {
    let trimmed = data.trim();
    let payload = trimmed.split_once(',').map_or(trimmed, |(_, rest)| rest);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .map_err(|err| format!("png base64 invalido: {err}"))?;
    let image = image::load_from_memory(&bytes).map_err(|err| format!("png invalido: {err}"))?;
    Ok(image.to_rgba8())
}

pub fn locate(
    haystack: &RgbaImage,
    needle: &RgbaImage,
    region: Option<Bounds>,
    grayscale: bool,
    confidence: Option<f64>,
) -> Result<Vec<Bounds>, String> {
    if needle.width() == 0 || needle.height() == 0 {
        return Err("the image to find is empty".into());
    }
    if let Some(score) = confidence
        && !(0.0..=1.0).contains(&score)
    {
        return Err("confidence must be between 0 and 1".into());
    }
    let (search, origin_x, origin_y) = match region {
        Some(bounds) => {
            if bounds.left.saturating_add(bounds.width) > haystack.width()
                || bounds.top.saturating_add(bounds.height) > haystack.height()
            {
                return Err("region is outside the screen".into());
            }
            let view = image::imageops::crop_imm(
                haystack,
                bounds.left,
                bounds.top,
                bounds.width,
                bounds.height,
            )
            .to_image();
            (view, bounds.left, bounds.top)
        }
        None => (haystack.clone(), 0, 0),
    };
    if needle.width() > search.width() || needle.height() > search.height() {
        return Ok(Vec::new());
    }
    let found = if let Some(score) = confidence {
        locate_confidence(&search, needle, grayscale, score)
    } else if grayscale {
        locate_exact(&search, needle, true)
    } else {
        locate_exact(&search, needle, false)
    };
    Ok(found
        .into_iter()
        .take(50)
        .map(|bounds| Bounds {
            left: bounds.left + origin_x,
            top: bounds.top + origin_y,
            ..bounds
        })
        .collect())
}

fn luma(pixel: &Rgba<u8>) -> u8 {
    ((pixel[0] as u32 * 299 + pixel[1] as u32 * 587 + pixel[2] as u32 * 114) / 1000) as u8
}

fn locate_exact(haystack: &RgbaImage, needle: &RgbaImage, grayscale: bool) -> Vec<Bounds> {
    let mut found = Vec::new();
    let max_x = haystack.width() - needle.width();
    let max_y = haystack.height() - needle.height();
    for y in 0..=max_y {
        for x in 0..=max_x {
            if window_equals(haystack, needle, x, y, grayscale) {
                found.push(Bounds {
                    left: x,
                    top: y,
                    width: needle.width(),
                    height: needle.height(),
                });
                if found.len() == 50 {
                    return found;
                }
            }
        }
    }
    found
}

fn window_equals(
    haystack: &RgbaImage,
    needle: &RgbaImage,
    origin_x: u32,
    origin_y: u32,
    grayscale: bool,
) -> bool {
    for ny in 0..needle.height() {
        for nx in 0..needle.width() {
            let left = haystack.get_pixel(origin_x + nx, origin_y + ny);
            let right = needle.get_pixel(nx, ny);
            let same = if grayscale {
                luma(left) == luma(right)
            } else {
                left[0] == right[0] && left[1] == right[1] && left[2] == right[2]
            };
            if !same {
                return false;
            }
        }
    }
    true
}

fn locate_confidence(
    haystack: &RgbaImage,
    needle: &RgbaImage,
    grayscale: bool,
    confidence: f64,
) -> Vec<Bounds> {
    let hay = plane(haystack, grayscale);
    let nee = plane(needle, grayscale);
    let (hw, hh) = (haystack.width(), haystack.height());
    let (nw, nh) = (needle.width(), needle.height());
    let comparisons = (hw - nw + 1) as u64 * (hh - nh + 1) as u64 * nw as u64 * nh as u64;
    if comparisons <= 4_000_000 {
        return peaks_at(&hay, hw, hh, &nee, nw, nh, confidence);
    }
    let mut factor = 2u32;
    while factor < 16 {
        let coarse = ((hw / factor).saturating_sub(nw / factor).saturating_add(1) as u64)
            * ((hh / factor).saturating_sub(nh / factor).saturating_add(1) as u64)
            * (nw / factor).max(1) as u64
            * (nh / factor).max(1) as u64;
        if coarse <= 4_000_000 {
            break;
        }
        factor += 1;
    }
    let (hay_s, hw_s, hh_s) = downsample(&hay, hw, hh, factor);
    let (nee_s, nw_s, nh_s) = downsample(&nee, nw, nh, factor);
    if nw_s == 0 || nh_s == 0 || nw_s > hw_s || nh_s > hh_s {
        return Vec::new();
    }
    let coarse = peaks_at(
        &hay_s,
        hw_s,
        hh_s,
        &nee_s,
        nw_s,
        nh_s,
        (confidence * 0.85).clamp(0.0, 1.0),
    );
    let mut refined = Vec::new();
    for candidate in coarse {
        let base_x = candidate.left.saturating_mul(factor);
        let base_y = candidate.top.saturating_mul(factor);
        let x0 = base_x.saturating_sub(factor);
        let y0 = base_y.saturating_sub(factor);
        let x1 = (base_x + factor).min(hw - nw);
        let y1 = (base_y + factor).min(hh - nh);
        let mut best: Option<(u32, u32, f64)> = None;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let score = ncc_at(&hay, hw, &nee, nw, nh, x, y);
                if best.is_none_or(|(_, _, current)| score > current) {
                    best = Some((x, y, score));
                }
            }
        }
        if let Some((x, y, score)) = best
            && score + 1e-6 >= confidence
        {
            refined.push(Bounds {
                left: x,
                top: y,
                width: nw,
                height: nh,
            });
        }
        if refined.len() == 50 {
            break;
        }
    }
    refined.sort_by_key(|bounds| (bounds.top, bounds.left));
    refined.truncate(50);
    refined
}

fn plane(image: &RgbaImage, _grayscale: bool) -> Vec<f32> {
    image.pixels().map(|pixel| luma(pixel) as f32).collect()
}

fn downsample(src: &[f32], width: u32, height: u32, factor: u32) -> (Vec<f32>, u32, u32) {
    let out_w = (width / factor).max(1);
    let out_h = (height / factor).max(1);
    let mut out = vec![0.0; (out_w * out_h) as usize];
    for y in 0..out_h {
        for x in 0..out_w {
            let mut sum = 0.0;
            let mut count = 0.0;
            for dy in 0..factor {
                for dx in 0..factor {
                    let sx = x * factor + dx;
                    let sy = y * factor + dy;
                    if sx < width && sy < height {
                        sum += src[(sy * width + sx) as usize];
                        count += 1.0;
                    }
                }
            }
            out[(y * out_w + x) as usize] = if count == 0.0 { 0.0 } else { sum / count };
        }
    }
    (out, out_w, out_h)
}

fn peaks_at(
    hay: &[f32],
    hw: u32,
    hh: u32,
    needle: &[f32],
    nw: u32,
    nh: u32,
    confidence: f64,
) -> Vec<Bounds> {
    let mut found = Vec::new();
    if nw > hw || nh > hh {
        return found;
    }
    for y in 0..=(hh - nh) {
        for x in 0..=(hw - nw) {
            let score = ncc_at(hay, hw, needle, nw, nh, x, y);
            if score + 1e-6 >= confidence {
                found.push(Bounds {
                    left: x,
                    top: y,
                    width: nw,
                    height: nh,
                });
                if found.len() == 50 {
                    return found;
                }
            }
        }
    }
    found
}

fn ncc_at(hay: &[f32], hw: u32, needle: &[f32], nw: u32, nh: u32, x: u32, y: u32) -> f64 {
    let mut needle_sum = 0.0;
    let count = (nw * nh) as f32;
    for value in needle {
        needle_sum += *value;
    }
    let needle_mean = needle_sum / count;
    let mut hay_sum = 0.0;
    for ny in 0..nh {
        for nx in 0..nw {
            hay_sum += hay[((y + ny) * hw + (x + nx)) as usize];
        }
    }
    let hay_mean = hay_sum / count;
    let mut num = 0.0;
    let mut hay_var = 0.0;
    let mut needle_var = 0.0;
    let mut index = 0usize;
    for ny in 0..nh {
        for nx in 0..nw {
            let h = hay[((y + ny) * hw + (x + nx)) as usize] - hay_mean;
            let n = needle[index] - needle_mean;
            num += h * n;
            hay_var += h * h;
            needle_var += n * n;
            index += 1;
        }
    }
    if hay_var <= f32::EPSILON || needle_var <= f32::EPSILON {
        return if (hay_mean - needle_mean).abs() < 0.5 {
            1.0
        } else {
            0.0
        };
    }
    (num / (hay_var.sqrt() * needle_var.sqrt())) as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn solid(width: u32, height: u32, color: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(width, height, Rgba(color))
    }

    #[test]
    fn tolerance_allows_a_small_channel_gap() {
        assert!(pixel_matches([130, 135, 144], [130, 135, 144], 0));
        assert!(!pixel_matches([140, 125, 134], [130, 135, 144], 0));
        assert!(pixel_matches([140, 125, 134], [130, 135, 144], 10));
    }

    #[test]
    fn exact_locate_finds_the_fixture_box() {
        let mut hay = solid(8, 6, [10, 10, 10, 255]);
        for y in 2..4 {
            for x in 3..6 {
                hay.put_pixel(x, y, Rgba([200, 20, 20, 255]));
            }
        }
        let needle = image::imageops::crop_imm(&hay, 3, 2, 3, 2).to_image();
        let found = locate(&hay, &needle, None, false, None).unwrap();
        assert_eq!(
            found,
            vec![Bounds {
                left: 3,
                top: 2,
                width: 3,
                height: 2
            }]
        );
        let confident = locate(&hay, &needle, None, false, Some(0.99)).unwrap();
        assert_eq!(confident, found);
        let missed = locate(&hay, &needle, None, false, Some(1.0)).unwrap();
        assert!(!missed.is_empty());
    }
}
