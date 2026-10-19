//! Image to seeds: Material Color Utilities quantize + score, then OKLab k-means
//! for the neutral that the scorer throws away.

use crate::color::{self, Rgb};
use crate::kmeans;
use crate::mc::{from_mc, to_mc};
use crate::roles::Seeds;
use crate::types::{ExtractionMode, ImageInput};
use material_colors::hct::Hct;
use material_colors::quantize::{Quantizer, QuantizerCelebi};
use material_colors::score::Score;

const PIXEL_BUDGET: usize = 128 * 128;

pub fn from_images(images: &[ImageInput], mode: ExtractionMode) -> Result<Seeds, String> {
    let pixels = sample_all(images)?;
    if pixels.is_empty() {
        return Err("no opaque pixels to read".into());
    }
    let mc_pixels: Vec<_> = pixels.iter().copied().map(to_mc).collect();
    let quant = QuantizerCelebi::quantize(&mc_pixels, 128);
    let filter = !matches!(mode, ExtractionMode::Colorful);
    let ranked = Score::score(&quant.color_to_count, Some(8), None, Some(filter));
    if ranked.is_empty() {
        return Err("the quantizer returned no colors".into());
    }
    let labs: Vec<_> = pixels.iter().copied().map(color::oklab_from_rgb).collect();
    let neutral = dominant_neutral(&labs).unwrap_or(pixels[0]);
    let primary = from_mc(ranked[0]);
    let accent = pick_accent(&ranked, primary);
    let (primary, accent) = apply_mode(mode, primary, accent);
    Ok(Seeds::chromatic(neutral, primary, accent))
}

fn sample_all(images: &[ImageInput]) -> Result<Vec<Rgb>, String> {
    let mut weights = Vec::with_capacity(images.len());
    for image in images {
        let w = if image.weight.is_finite() && image.weight > 0.0 {
            image.weight
        } else {
            0.0
        };
        weights.push(w);
    }
    if weights.iter().all(|w| *w == 0.0) {
        weights.fill(1.0);
    }
    let total: f64 = weights.iter().sum();
    let mut out = Vec::new();
    for (image, weight) in images.iter().zip(weights) {
        let share = ((weight / total) * PIXEL_BUDGET as f64).round().max(1.0) as usize;
        out.extend(sample_image(image, share)?);
    }
    Ok(out)
}

fn sample_image(image: &ImageInput, share: usize) -> Result<Vec<Rgb>, String> {
    let expected = (image.width as usize)
        .checked_mul(image.height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "image dimensions overflow".to_string())?;
    if image.rgba.len() != expected {
        return Err(format!(
            "rgba length {} does not match {}x{}",
            image.rgba.len(),
            image.width,
            image.height
        ));
    }
    let mut opaque = Vec::new();
    let (rows, _) = image.rgba.as_chunks::<4>();
    for px in rows {
        if px[3] >= 128 {
            opaque.push(Rgb::new(px[0], px[1], px[2]));
        }
    }
    if opaque.is_empty() {
        return Ok(opaque);
    }
    // Repeat a small image so its weight still changes how many pixels it owns.
    let mut picked = Vec::with_capacity(share);
    if opaque.len() >= share {
        let stride = opaque.len() as f64 / share as f64;
        for i in 0..share {
            let idx = ((i as f64) * stride) as usize;
            picked.push(opaque[idx.min(opaque.len() - 1)]);
        }
    } else {
        for i in 0..share {
            picked.push(opaque[i % opaque.len()]);
        }
    }
    Ok(picked)
}

fn dominant_neutral(points: &[color::Oklab]) -> Option<Rgb> {
    let groups = kmeans::cluster(points, 5.min(points.len()), 12);
    let neutral = groups
        .iter()
        .filter(|g| g.count > 0 && g.center.chroma() <= 0.06)
        .max_by_key(|g| g.count);
    let chosen = neutral.or_else(|| {
        groups.iter().filter(|g| g.count > 0).min_by(|a, b| {
            a.center
                .chroma()
                .partial_cmp(&b.center.chroma())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    })?;
    Some(color::rgb_from_oklab(chosen.center))
}

fn pick_accent(ranked: &[material_colors::color::Rgb], primary: Rgb) -> Rgb {
    let primary_hue = hue_of(primary);
    for color in ranked.iter().skip(1) {
        let rgb = from_mc(*color);
        if hue_distance(hue_of(rgb), primary_hue) >= 60.0 {
            return rgb;
        }
    }
    complement(primary)
}

fn apply_mode(mode: ExtractionMode, primary: Rgb, accent: Rgb) -> (Rgb, Rgb) {
    match mode {
        ExtractionMode::Normal => (primary, accent),
        ExtractionMode::Pastel => (
            shift_chroma(primary, 0.55, Some(80.0)),
            shift_chroma(accent, 0.55, Some(80.0)),
        ),
        ExtractionMode::Muted => (
            shift_chroma(primary, 0.35, None),
            shift_chroma(accent, 0.35, None),
        ),
        ExtractionMode::Colorful => (
            shift_chroma(primary, 1.35, None),
            shift_chroma(accent, 1.35, None),
        ),
        ExtractionMode::Material => material_seeds(primary),
    }
}

/// Tonal-spot seeds: primary near tone 40, accent a hue step away.
/// This stays on HCT palettes and does not pull the dynamic-scheme engine.
fn material_seeds(primary: Rgb) -> (Rgb, Rgb) {
    let hct: Hct = to_mc(primary).into();
    let primary = from_mc(material_colors::color::Rgb::from(Hct::from(
        hct.get_hue(),
        hct.get_chroma().clamp(36.0, 72.0),
        40.0,
    )));
    let accent = from_mc(material_colors::color::Rgb::from(Hct::from(
        (hct.get_hue() + 60.0) % 360.0,
        hct.get_chroma().clamp(24.0, 48.0),
        40.0,
    )));
    (primary, accent)
}

fn shift_chroma(rgb: Rgb, scale: f64, tone_toward: Option<f64>) -> Rgb {
    let hct: Hct = to_mc(rgb).into();
    let chroma = (hct.get_chroma() * scale).clamp(0.0, 120.0);
    let tone = if let Some(target) = tone_toward {
        hct.get_tone() + (target - hct.get_tone()) * 0.45
    } else {
        hct.get_tone()
    };
    from_mc(material_colors::color::Rgb::from(Hct::from(
        hct.get_hue(),
        chroma,
        tone,
    )))
}

pub fn hue_of(rgb: Rgb) -> f64 {
    let hct: Hct = to_mc(rgb).into();
    hct.get_hue()
}

pub fn hue_distance(a: f64, b: f64) -> f64 {
    let d = (a - b).abs() % 360.0;
    if d > 180.0 {
        360.0 - d
    } else {
        d
    }
}

fn complement(rgb: Rgb) -> Rgb {
    let hct: Hct = to_mc(rgb).into();
    let hue = (hct.get_hue() + 180.0) % 360.0;
    from_mc(material_colors::color::Rgb::from(Hct::from(
        hue,
        hct.get_chroma(),
        hct.get_tone(),
    )))
}
