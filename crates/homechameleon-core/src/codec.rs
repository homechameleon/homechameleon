//! JSON boundary for `generate_json`. Pixels are base64 here and bytes everywhere else.

use crate::b64;
use crate::json::{expect_u32, Json};
use crate::types::{
    BackgroundOptions, ContrastReport, ContrastTarget, ExtractionMode, GenerateError,
    GenerateRequest, GenerateResult, ImageInput, Mode, PaletteFormat, PaletteInput, RoleColors,
    StyleId, ThemeOptions, Tweaks,
};

pub fn request_from_json(value: &Json) -> Result<GenerateRequest, GenerateError> {
    let mut request = GenerateRequest::default();
    if let Some(name) = value.get("name").and_then(Json::as_str) {
        request.name = name.to_string();
    }
    if let Some(style) = value.get("style").and_then(Json::as_str) {
        request.style = StyleId::parse(style)?;
    }
    if let Some(mode) = value.get("mode").and_then(Json::as_str) {
        request.mode = parse_mode(mode)?;
    }
    if let Some(extraction) = value.get("extraction").and_then(Json::as_str) {
        request.extraction = parse_extraction(extraction)?;
    }
    if let Some(palette) = value.get("palette") {
        if !matches!(palette, Json::Null) {
            request.palette = Some(parse_palette(palette)?);
        }
    }
    if let Some(images) = value.get("images") {
        let items = images
            .as_array()
            .ok_or_else(|| bad("images must be an array"))?;
        request.images = items.iter().map(parse_image).collect::<Result<_, _>>()?;
    }
    if let Some(background) = value.get("background") {
        request.background = parse_background(background)?;
    }
    if let Some(options) = value.get("options") {
        request.options = parse_options(options)?;
    }
    if let Some(tweaks) = value.get("tweaks") {
        if !matches!(tweaks, Json::Null) {
            request.tweaks = Some(parse_tweaks(tweaks)?);
        }
    }
    Ok(request)
}

pub fn result_to_json(result: &GenerateResult) -> String {
    let mut pairs = vec![
        ("theme", Json::from(result.theme.as_str())),
        ("slug", Json::from(result.slug.as_str())),
        ("style", Json::from(result.style.id())),
        (
            "files",
            Json::Array(
                result
                    .files
                    .iter()
                    .map(|f| Json::from(f.as_str()))
                    .collect(),
            ),
        ),
        ("yaml", Json::from(result.yaml.as_str())),
        ("css", Json::from(result.css.as_str())),
    ];
    if let Some(light) = &result.light {
        pairs.push(("light", roles_json(light)));
    }
    if let Some(dark) = &result.dark {
        pairs.push(("dark", roles_json(dark)));
    }
    pairs.push(("palette_json", Json::from(result.palette_json.as_str())));
    pairs.push(("contrast", contrast_json(&result.contrast)));
    pairs.push((
        "warnings",
        Json::Array(
            result
                .warnings
                .iter()
                .map(|w| Json::from(w.as_str()))
                .collect(),
        ),
    ));
    Json::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()).stringify()
}

fn roles_json(roles: &RoleColors) -> Json {
    Json::object([
        ("bg", Json::from(roles.bg.as_str())),
        ("surface", Json::from(roles.surface.as_str())),
        ("primary", Json::from(roles.primary.as_str())),
        ("accent", Json::from(roles.accent.as_str())),
        ("text", Json::from(roles.text.as_str())),
        ("success", Json::from(roles.success.as_str())),
        ("warn", Json::from(roles.warn.as_str())),
        ("error", Json::from(roles.error.as_str())),
        ("on_primary", Json::from(roles.on_primary.as_str())),
    ])
}

fn contrast_json(report: &ContrastReport) -> Json {
    let checks = report
        .checks
        .iter()
        .map(|check| {
            Json::object([
                ("mode", Json::from(check.mode.as_str())),
                ("pair", Json::from(check.pair.as_str())),
                ("ratio", Json::Number(check.ratio)),
                ("minimum", Json::Number(check.minimum)),
                ("pass", Json::from(check.pass)),
            ])
        })
        .collect();
    Json::object([
        ("pass", Json::from(report.pass)),
        ("checks", Json::Array(checks)),
    ])
}

fn parse_mode(text: &str) -> Result<Mode, GenerateError> {
    match text {
        "dark" => Ok(Mode::Dark),
        "light" => Ok(Mode::Light),
        "both" => Ok(Mode::Both),
        other => Err(bad(format!("unknown mode '{other}'"))),
    }
}

fn parse_extraction(text: &str) -> Result<ExtractionMode, GenerateError> {
    match text {
        "normal" => Ok(ExtractionMode::Normal),
        "pastel" => Ok(ExtractionMode::Pastel),
        "material" => Ok(ExtractionMode::Material),
        "colorful" => Ok(ExtractionMode::Colorful),
        "muted" => Ok(ExtractionMode::Muted),
        other => Err(bad(format!("unknown extraction mode '{other}'"))),
    }
}

fn parse_palette(value: &Json) -> Result<PaletteInput, GenerateError> {
    let format = value
        .get("format")
        .and_then(Json::as_str)
        .ok_or_else(|| bad("palette format is missing"))?;
    let data = value
        .get("data")
        .and_then(Json::as_str)
        .ok_or_else(|| bad("palette data is missing"))?;
    let format = match format {
        "omarchy-colors-toml" => PaletteFormat::OmarchyColorsToml,
        "base16" => PaletteFormat::Base16,
        "roles" => PaletteFormat::Roles,
        "hex-list" => PaletteFormat::HexList,
        "homechameleon-palette" => PaletteFormat::HomeChameleonPalette,
        other => return Err(bad(format!("unknown palette format '{other}'"))),
    };
    Ok(PaletteInput {
        format,
        data: data.to_string(),
    })
}

fn parse_image(value: &Json) -> Result<ImageInput, GenerateError> {
    let width = expect_u32(value.get("width").unwrap_or(&Json::Null), "width").map_err(bad)?;
    let height = expect_u32(value.get("height").unwrap_or(&Json::Null), "height").map_err(bad)?;
    let rgba_text = value
        .get("rgba")
        .and_then(Json::as_str)
        .ok_or_else(|| bad("image rgba must be base64"))?;
    let rgba = b64::decode(rgba_text).map_err(|err| bad(format!("image rgba {err}")))?;
    let weight = value.get("weight").and_then(Json::as_f64).unwrap_or(1.0);
    let use_as_background = value
        .get("use_as_background")
        .and_then(Json::as_bool)
        .unwrap_or(false);
    Ok(ImageInput {
        width,
        height,
        rgba,
        weight,
        use_as_background,
    })
}

fn parse_background(value: &Json) -> Result<BackgroundOptions, GenerateError> {
    let mut background = BackgroundOptions::default();
    if let Some(enabled) = value.get("enabled").and_then(Json::as_bool) {
        background.enabled = enabled;
    }
    if let Some(blur) = value.get("blur") {
        background.blur = expect_u32(blur, "blur").map_err(bad)?;
    }
    if let Some(dim) = value.get("dim").and_then(Json::as_f64) {
        background.dim = dim;
    }
    if let Some(max_edge) = value.get("max_edge") {
        background.max_edge = expect_u32(max_edge, "max_edge").map_err(bad)?;
    }
    if let Some(name) = value.get("file_name").and_then(Json::as_str) {
        background.file_name = Some(name.to_string());
    }
    Ok(background)
}

fn parse_options(value: &Json) -> Result<ThemeOptions, GenerateError> {
    let mut options = ThemeOptions::default();
    if let Some(card_mod) = value.get("card_mod").and_then(Json::as_bool) {
        options.card_mod = card_mod;
    }
    if let Some(bubble) = value.get("bubble").and_then(Json::as_bool) {
        options.bubble = bubble;
    }
    if let Some(mushroom) = value.get("mushroom").and_then(Json::as_bool) {
        options.mushroom = mushroom;
    }
    if let Some(contrast) = value.get("contrast").and_then(Json::as_str) {
        options.contrast = match contrast {
            "AA" => ContrastTarget::Aa,
            "AAA" => ContrastTarget::Aaa,
            other => return Err(bad(format!("unknown contrast target '{other}'"))),
        };
    }
    Ok(options)
}

fn parse_tweaks(value: &Json) -> Result<Tweaks, GenerateError> {
    let number = |key| -> Result<f64, GenerateError> {
        value
            .get(key)
            .and_then(Json::as_f64)
            .ok_or_else(|| bad(format!("tweaks.{key} must be a number")))
    };
    Ok(Tweaks {
        roundness: number("roundness")?,
        blur: number("blur")?,
        transparency: number("transparency")?,
        glow: number("glow")?,
        saturation: number("saturation")?,
        temperature: number("temperature")?,
    })
}

fn bad(reason: impl Into<String>) -> GenerateError {
    GenerateError::BadPalette {
        reason: reason.into(),
    }
}
