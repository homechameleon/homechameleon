//! WCAG 2.x contrast. Role colors are chosen against these ratios.
//! There is no separate repair step after a theme is built.

use crate::color::Rgb;
use libm::pow;

/// Relative luminance, 0–1, from the WCAG definition.
pub fn relative_luminance(rgb: Rgb) -> f64 {
    let r = channel(rgb.r);
    let g = channel(rgb.g);
    let b = channel(rgb.b);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn channel(c: u8) -> f64 {
    let s = f64::from(c) / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        pow((s + 0.055) / 1.055, 2.4)
    }
}

/// Contrast ratio in the range 1–21.
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    let l1 = relative_luminance(a);
    let l2 = relative_luminance(b);
    let (hi, lo) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}

pub fn meets(a: Rgb, b: Rgb, minimum: f64) -> bool {
    contrast_ratio(a, b) + 1e-9 >= minimum
}

/// `alpha` is the coverage of `fg` over `bg` (1 = opaque).
pub fn composite(fg: Rgb, alpha: f64, bg: Rgb) -> Rgb {
    let a = alpha.clamp(0.0, 1.0);
    let mix = |f: u8, b: u8| -> u8 {
        let v = f64::from(f) * a + f64::from(b) * (1.0 - a);
        v.round().clamp(0.0, 255.0) as u8
    };
    Rgb::new(mix(fg.r, bg.r), mix(fg.g, bg.g), mix(fg.b, bg.b))
}

/// Gray at the 75th-percentile relative luminance of the opaque pixels.
/// Sheer checks text against the card color composited over this gray.
pub fn percentile_backdrop(rgba: &[u8], width: u32, height: u32) -> Option<Rgb> {
    let expected = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if rgba.len() < expected {
        return None;
    }
    let mut lums = Vec::new();
    let (rows, _) = rgba[..expected].as_chunks::<4>();
    for px in rows {
        if px[3] < 128 {
            continue;
        }
        lums.push(relative_luminance(Rgb::new(px[0], px[1], px[2])));
    }
    if lums.is_empty() {
        return None;
    }
    lums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((lums.len() - 1) as f64 * 0.75).floor() as usize;
    Some(gray_from_luminance(lums[idx]))
}

fn gray_from_luminance(y: f64) -> Rgb {
    let y = y.clamp(0.0, 1.0);
    let c = if y <= 0.003_130_8 {
        12.92 * y
    } else {
        1.055 * pow(y, 1.0 / 2.4) - 0.055
    };
    let v = (c * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb::new(v, v, v)
}

/// Smallest opacity of `paint` over `backdrop`, at or above `preferred`,
/// for which every `(color, minimum)` pair passes. Opacity is an integer percent.
pub fn raise_alpha(paint: Rgb, backdrop: Rgb, preferred: f64, checks: &[(Rgb, f64)]) -> f64 {
    let start = (preferred.clamp(0.0, 1.0) * 100.0).round() as i32;
    let start = start.clamp(0, 100);
    let ok = |pct: i32| {
        let card = composite(paint, f64::from(pct) / 100.0, backdrop);
        checks.iter().all(|(color, min)| meets(*color, card, *min))
    };
    if ok(start) {
        return f64::from(start) / 100.0;
    }
    for pct in (start + 1)..=100 {
        if ok(pct) {
            return f64::from(pct) / 100.0;
        }
    }
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_on_white_is_21() {
        let ratio = contrast_ratio(Rgb::BLACK, Rgb::WHITE);
        assert!((ratio - 21.0).abs() < 1e-6, "{ratio}");
    }

    #[test]
    fn identical_colors_are_1() {
        let c = Rgb::new(40, 80, 120);
        assert!((contrast_ratio(c, c) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn composite_endpoints() {
        let fg = Rgb::new(0, 0, 0);
        let bg = Rgb::new(255, 255, 255);
        assert_eq!(composite(fg, 1.0, bg), fg);
        assert_eq!(composite(fg, 0.0, bg), bg);
    }
}
