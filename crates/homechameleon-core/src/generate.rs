//! Turn a request into one theme file, a preview CSS map, and a palette document.

use std::collections::BTreeMap;

use crate::contrast::{self, composite};
use crate::extract;
use crate::palette;
use crate::roles::{self, Roles, Seeds};
use crate::styles;
use crate::types::{
    ColorMode, ContrastCheck, ContrastReport, GenerateError, GenerateRequest, GenerateResult, Mode,
    RoleColors, StyleId, MAX_IMAGES, MAX_IMAGE_BYTES,
};
use crate::yaml::{self, ThemeValue};

pub fn generate(request: &GenerateRequest) -> Result<GenerateResult, GenerateError> {
    validate(request)?;
    let mut warnings = Vec::new();
    let parsed = resolve_source(request, &mut warnings)?;
    let tweaks = request.tweaks();
    let modes = requested_modes(request.mode);
    let mut built: Vec<(ColorMode, Roles, BTreeMap<String, ThemeValue>, f64)> = Vec::new();
    let backdrop = background_parts(request, parsed.background_file.as_deref());
    for mode in modes {
        let (roles, mut role_warnings) = if request.style == StyleId::Material {
            roles::assign_material(
                &parsed.seeds,
                mode,
                request.options.contrast,
                tweaks.saturation,
                tweaks.temperature,
            )
        } else {
            roles::assign(
                &parsed.seeds,
                mode,
                request.options.contrast,
                tweaks.saturation,
                tweaks.temperature,
            )
        };
        warnings.append(&mut role_warnings);
        let slug = crate::types::slugify(&request.name);
        let built_mode = styles::build_mode(
            &styles::ModeInput {
                style: request.style,
                slug: &slug,
                roles: &roles,
                mode,
                tweaks,
                options: &request.options,
                background: &request.background,
                backdrop_rgba: backdrop.pixels.as_deref(),
                backdrop_size: backdrop.size,
                file_name: backdrop.file_name.as_deref(),
            },
            &mut warnings,
        );
        built.push((mode, roles, built_mode.vars, built_mode.card_alpha));
    }
    let slug = crate::types::slugify(&request.name);
    let display_name = request.name.trim();
    let display_name = if display_name.is_empty() {
        "HomeChameleon"
    } else {
        display_name
    };
    let yaml_modes: Vec<(&str, BTreeMap<String, ThemeValue>)> = built
        .iter()
        .map(|(mode, _, vars, _)| (mode.id(), vars.clone()))
        .collect();
    let yaml = yaml::emit(
        display_name,
        request.style.id(),
        request.background.enabled,
        &yaml_modes,
    );
    let css = yaml::emit_css(&yaml_modes);
    let contrast = contrast_report(request, &built, backdrop.pixels.as_deref(), backdrop.size);
    let palette_mode = built
        .first()
        .map(|(mode, _, _, _)| *mode)
        .unwrap_or(ColorMode::Dark);
    let palette_roles = built
        .iter()
        .find(|(mode, _, _, _)| *mode == palette_mode)
        .map(|(_, roles, _, _)| roles)
        .expect("a theme always has a mode");
    let background_file = if request.background.enabled {
        backdrop.file_name.clone()
    } else {
        None
    };
    let theme_name = parsed.theme.clone().unwrap_or_else(|| slug.clone());
    let doc = palette::document_from_roles(
        &theme_name,
        parsed.source.clone(),
        palette_mode,
        palette_roles,
        &parsed.colors,
        background_file,
    );
    let mut light = None;
    let mut dark = None;
    for (mode, roles, _, _) in &built {
        let colors = role_colors(roles);
        match mode {
            ColorMode::Light => light = Some(colors),
            ColorMode::Dark => dark = Some(colors),
        }
    }
    dedupe(&mut warnings);
    Ok(GenerateResult {
        theme: display_name.to_string(),
        slug: slug.clone(),
        style: request.style,
        files: vec![format!("themes/{slug}.yaml")],
        yaml,
        css,
        light,
        dark,
        palette_json: palette::to_json(&doc),
        contrast,
        warnings,
    })
}

struct Resolved {
    seeds: Seeds,
    colors: BTreeMap<String, String>,
    theme: Option<String>,
    source: Option<String>,
    background_file: Option<String>,
}

fn resolve_source(
    request: &GenerateRequest,
    warnings: &mut Vec<String>,
) -> Result<Resolved, GenerateError> {
    if let Some(palette) = &request.palette {
        if !request.images.is_empty() {
            warnings.push(
                "A palette was provided, so image pixels were not used to choose colors.".into(),
            );
        }
        let parsed = palette::parse(palette)?;
        return Ok(Resolved {
            seeds: parsed.seeds,
            colors: parsed.colors,
            theme: parsed.theme,
            source: parsed.source,
            background_file: parsed.background_file,
        });
    }
    if request.images.is_empty() {
        return Err(GenerateError::MissingSource);
    }
    let seeds = extract::from_images(&request.images, request.extraction)
        .map_err(|reason| GenerateError::BadImage { reason })?;
    Ok(Resolved {
        seeds,
        colors: BTreeMap::new(),
        theme: None,
        source: Some("image".into()),
        background_file: None,
    })
}

fn validate(request: &GenerateRequest) -> Result<(), GenerateError> {
    if request.images.len() > MAX_IMAGES {
        return Err(GenerateError::TooManyImages {
            count: request.images.len(),
            limit: MAX_IMAGES,
        });
    }
    for image in &request.images {
        if image.rgba.len() > MAX_IMAGE_BYTES {
            return Err(GenerateError::ImageTooLarge {
                bytes: image.rgba.len(),
                limit: MAX_IMAGE_BYTES,
            });
        }
        let expected = (image.width as usize)
            .checked_mul(image.height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(|| GenerateError::BadImage {
                reason: "image dimensions overflow".into(),
            })?;
        if image.rgba.len() != expected {
            return Err(GenerateError::BadImage {
                reason: format!(
                    "rgba length {} does not match {}x{}",
                    image.rgba.len(),
                    image.width,
                    image.height
                ),
            });
        }
    }
    Ok(())
}

fn requested_modes(mode: Mode) -> Vec<ColorMode> {
    match mode {
        Mode::Light => vec![ColorMode::Light],
        Mode::Dark => vec![ColorMode::Dark],
        Mode::Both => vec![ColorMode::Light, ColorMode::Dark],
    }
}

struct Backdrop {
    pixels: Option<Vec<u8>>,
    size: Option<(u32, u32)>,
    file_name: Option<String>,
}

fn background_parts(request: &GenerateRequest, palette_file: Option<&str>) -> Backdrop {
    if !request.background.enabled {
        return Backdrop {
            pixels: None,
            size: None,
            file_name: None,
        };
    }
    let image = request
        .images
        .iter()
        .find(|image| image.use_as_background)
        .or_else(|| request.images.first());
    let name = request.background.file_name.clone().or_else(|| {
        image
            .map(|image| format!("bg-{}.webp", short_hash(&image.rgba)))
            .or_else(|| palette_file.map(str::to_string))
    });
    match image {
        Some(image) => Backdrop {
            pixels: Some(image.rgba.clone()),
            size: Some((image.width, image.height)),
            file_name: name,
        },
        None => Backdrop {
            pixels: None,
            size: None,
            file_name: name,
        },
    }
}

fn short_hash(bytes: &[u8]) -> String {
    // FNV-1a, 32 bits. Stable across native and wasm, and small enough for the wasm build.
    let mut hash: u32 = 0x811c_9dc5;
    for byte in bytes {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    format!("{hash:08x}")
}

fn contrast_report(
    request: &GenerateRequest,
    built: &[(ColorMode, Roles, BTreeMap<String, ThemeValue>, f64)],
    backdrop: Option<&[u8]>,
    backdrop_size: Option<(u32, u32)>,
) -> ContrastReport {
    let text_min = request.options.contrast.text_ratio();
    let ui_min = request.options.contrast.ui_ratio();
    let mut checks = Vec::new();
    for (mode, roles, _, alpha) in built {
        let push = |checks: &mut Vec<ContrastCheck>,
                    pair: &str,
                    a: crate::color::Rgb,
                    b: crate::color::Rgb,
                    minimum: f64| {
            let ratio = contrast::contrast_ratio(a, b);
            checks.push(ContrastCheck {
                mode: mode.id().into(),
                pair: pair.into(),
                ratio,
                minimum,
                pass: ratio + 1e-9 >= minimum,
            });
        };
        push(
            &mut checks,
            "text/surface",
            roles.text,
            roles.surface,
            text_min,
        );
        push(
            &mut checks,
            "text/background",
            roles.text,
            roles.bg,
            text_min,
        );
        push(
            &mut checks,
            "muted/surface",
            roles.text_muted,
            roles.surface,
            text_min,
        );
        push(
            &mut checks,
            "primary/surface",
            roles.primary,
            roles.surface,
            ui_min,
        );
        push(
            &mut checks,
            "on-primary/primary",
            roles.on_primary,
            roles.primary,
            text_min,
        );
        push(
            &mut checks,
            "success/surface",
            roles.success,
            roles.surface,
            ui_min,
        );
        push(
            &mut checks,
            "warning/surface",
            roles.warn,
            roles.surface,
            ui_min,
        );
        push(
            &mut checks,
            "error/surface",
            roles.error,
            roles.surface,
            ui_min,
        );
        if *alpha < 0.999 {
            let backdrop_color = backdrop_color(request, roles, backdrop, backdrop_size);
            let card = composite(roles.surface, *alpha, backdrop_color);
            push(&mut checks, "text/card", roles.text, card, text_min);
            push(&mut checks, "muted/card", roles.text_muted, card, text_min);
        }
    }
    let pass = checks.iter().all(|check| check.pass);
    ContrastReport { pass, checks }
}

fn backdrop_color(
    request: &GenerateRequest,
    roles: &Roles,
    backdrop: Option<&[u8]>,
    size: Option<(u32, u32)>,
) -> crate::color::Rgb {
    if request.background.enabled {
        if let (Some(bytes), Some((w, h))) = (backdrop, size) {
            if let Some(gray) = contrast::percentile_backdrop(bytes, w, h) {
                return gray;
            }
        }
    }
    roles.bg
}

fn role_colors(roles: &Roles) -> RoleColors {
    RoleColors {
        bg: roles.bg.hex(),
        surface: roles.surface.hex(),
        primary: roles.primary.hex(),
        accent: roles.accent.hex(),
        text: roles.text.hex(),
        success: roles.success.hex(),
        warn: roles.warn.hex(),
        error: roles.error.hex(),
        on_primary: roles.on_primary.hex(),
    }
}

fn dedupe(warnings: &mut Vec<String>) {
    let mut seen = Vec::new();
    warnings.retain(|warning| {
        if seen.contains(warning) {
            false
        } else {
            seen.push(warning.clone());
            true
        }
    });
}
