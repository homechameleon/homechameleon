//! Omarchy `colors.toml` and `homechameleon-palette/1` JSON.
//! Unknown color keys are kept. `background_file` is ignored unless the
//! background option is on; the caller decides that.

use std::collections::BTreeMap;

use crate::color::Rgb;
use crate::contrast::relative_luminance;
use crate::json::{self, Json};
use crate::roles::{Roles, Seeds};
use crate::types::{ColorMode, GenerateError, PaletteFormat, PaletteInput};

pub const SCHEMA: &str = "homechameleon-palette/1";

/// Keys written by `homechameleon-sync --export`, in that order.
/// Key order used by `homechameleon-sync --export`.
pub fn export_keys() -> &'static [&'static str] {
    EXPORT_KEYS
}

#[allow(dead_code)]
const EXPORT_KEYS: &[&str] = &[
    "accent",
    "selection",
    "muted",
    "background",
    "dark_background",
    "darker_background",
    "lighter_background",
    "foreground",
    "dark_foreground",
    "light_foreground",
    "bright_foreground",
    "red",
    "yellow",
    "orange",
    "green",
    "cyan",
    "blue",
    "magenta",
    "brown",
    "bright_red",
    "bright_yellow",
    "bright_green",
    "bright_cyan",
    "bright_blue",
    "bright_magenta",
    "color0",
    "color1",
    "color2",
    "color3",
    "color4",
    "color5",
    "color6",
    "color7",
    "color8",
    "color9",
    "color10",
    "color11",
    "color12",
    "color13",
    "color14",
    "color15",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteDocument {
    pub schema: String,
    /// Where the colors came from. The Omarchy exporter writes `"omarchy"`.
    pub source: Option<String>,
    /// Source theme name (an Omarchy theme name when that is the source).
    pub theme: String,
    pub mode: String,
    pub colors: BTreeMap<String, String>,
    pub background_file: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ParsedPalette {
    pub seeds: Seeds,
    pub colors: BTreeMap<String, String>,
    pub theme: Option<String>,
    pub source: Option<String>,
    pub background_file: Option<String>,
}

pub fn parse(input: &PaletteInput) -> Result<ParsedPalette, GenerateError> {
    match input.format {
        PaletteFormat::OmarchyColorsToml => parse_colors_toml(&input.data),
        PaletteFormat::Base16 => parse_base16(&input.data),
        PaletteFormat::HexList => parse_hex_list(&input.data),
        PaletteFormat::Roles => parse_roles_json(&input.data),
        PaletteFormat::HomeChameleonPalette => parse_document(&input.data),
    }
}

pub fn parse_document(data: &str) -> Result<ParsedPalette, GenerateError> {
    let value =
        json::parse(data).map_err(|err| bad(format!("palette JSON could not be read: {err}")))?;
    let doc = document_from_json(&value)?;
    if doc.schema != SCHEMA {
        return Err(bad(format!("palette schema must be {SCHEMA}")));
    }
    if doc.theme.trim().is_empty() {
        return Err(bad("palette theme name is empty"));
    }
    if doc.mode != "dark" && doc.mode != "light" {
        return Err(bad("palette mode must be dark or light"));
    }
    let colors = normalize_colors(&doc.colors)?;
    require_core(&colors)?;
    let mut seeds = seeds_from_omarchy_map(&colors)?;
    seeds.explicit_mode = Some(if doc.mode == "dark" {
        ColorMode::Dark
    } else {
        ColorMode::Light
    });
    Ok(ParsedPalette {
        seeds,
        colors,
        theme: Some(doc.theme),
        source: doc.source,
        background_file: doc.background_file,
    })
}

fn parse_colors_toml(data: &str) -> Result<ParsedPalette, GenerateError> {
    let mut raw = BTreeMap::new();
    let mut mode = None;
    for line in data.lines() {
        let line = strip_comment(line).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            return Err(bad("colors.toml must be a flat table"));
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(bad("colors.toml line is missing '='"));
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"').trim_matches('\'');
        if key == "mode" {
            mode = Some(value.to_string());
            continue;
        }
        if let Some(rgb) = Rgb::parse(value) {
            raw.insert(key.to_string(), rgb.hex());
        }
    }
    require_core(&raw)?;
    let mut colors = raw;
    derive_ansi(&mut colors);
    let mut seeds = seeds_from_omarchy_map(&colors)?;
    seeds.explicit_mode = Some(resolve_mode(mode.as_deref(), seeds.neutral));
    Ok(ParsedPalette {
        seeds,
        colors,
        theme: None,
        source: Some("omarchy".into()),
        background_file: None,
    })
}

fn parse_base16(data: &str) -> Result<ParsedPalette, GenerateError> {
    let mut bases = BTreeMap::new();
    let mut scheme = None;
    for line in data.lines() {
        let line = strip_comment(line).trim().to_string();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim().trim_matches('"').trim_matches('\'');
        if key == "scheme" {
            scheme = Some(value.to_string());
            continue;
        }
        if let Some(rest) = key.strip_prefix("base") {
            if let Some(rgb) = Rgb::parse(value) {
                bases.insert(rest.to_ascii_lowercase(), rgb);
            }
        }
    }
    let bg = bases
        .get("00")
        .copied()
        .ok_or_else(|| bad("base16 is missing base00"))?;
    let text = bases
        .get("05")
        .copied()
        .ok_or_else(|| bad("base16 is missing base05"))?;
    let primary = bases
        .get("0d")
        .or_else(|| bases.get("0e"))
        .copied()
        .ok_or_else(|| bad("base16 is missing base0D"))?;
    let accent = bases
        .get("0e")
        .or_else(|| bases.get("0c"))
        .copied()
        .unwrap_or(primary);
    let mut seeds = Seeds::chromatic(bg, primary, accent);
    seeds.text = Some(text);
    seeds.surface = bases.get("01").copied();
    seeds.surface2 = bases.get("02").copied();
    seeds.text_muted = bases.get("04").copied();
    seeds.outline = bases.get("03").copied();
    seeds.error = bases.get("08").copied();
    seeds.warn = bases.get("0a").copied();
    seeds.success = bases.get("0b").copied();
    seeds.info = bases.get("0c").copied();
    seeds.explicit_mode = Some(if relative_luminance(bg) < 0.4 {
        ColorMode::Dark
    } else {
        ColorMode::Light
    });
    if bases.len() >= 16 {
        let mut ramp = [Rgb::BLACK; 16];
        for (i, slot) in ramp.iter_mut().enumerate() {
            let key = format!("{i:x}");
            if let Some(color) = bases.get(key.as_str()) {
                *slot = *color;
            }
        }
        seeds.ramp = Some(ramp);
    }
    let mut colors = BTreeMap::new();
    colors.insert("accent".into(), primary.hex());
    colors.insert("background".into(), bg.hex());
    colors.insert("foreground".into(), text.hex());
    Ok(ParsedPalette {
        seeds,
        colors,
        theme: scheme,
        source: Some("base16".into()),
        background_file: None,
    })
}

fn parse_hex_list(data: &str) -> Result<ParsedPalette, GenerateError> {
    let colors: Vec<Rgb> = data
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .filter(|part| !part.is_empty())
        .map(|part| {
            Rgb::parse(part).ok_or_else(|| bad(format!("could not read hex color '{part}'")))
        })
        .collect::<Result<_, _>>()?;
    if colors.is_empty() {
        return Err(bad("hex list is empty"));
    }
    if colors.len() >= 16 {
        let mut ramp = [Rgb::BLACK; 16];
        ramp.copy_from_slice(&colors[..16]);
        let mut map = BTreeMap::new();
        for (i, color) in ramp.iter().enumerate() {
            map.insert(format!("color{i}"), color.hex());
        }
        map.insert("background".into(), ramp[0].hex());
        map.insert("foreground".into(), ramp[7].hex());
        map.insert("accent".into(), ramp[4].hex());
        derive_ansi(&mut map);
        let mut seeds = seeds_from_omarchy_map(&map)?;
        seeds.ramp = Some(ramp);
        seeds.explicit_mode = Some(resolve_mode(None, seeds.neutral));
        return Ok(ParsedPalette {
            seeds,
            colors: map,
            theme: None,
            source: Some("hex-list".into()),
            background_file: None,
        });
    }
    let primary = colors[0];
    let accent = colors.get(1).copied().unwrap_or(primary);
    let neutral = colors
        .iter()
        .copied()
        .min_by(|a, b| {
            relative_luminance(*a)
                .partial_cmp(&relative_luminance(*b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(Rgb::new(20, 20, 24));
    let mut seeds = Seeds::chromatic(neutral, primary, accent);
    seeds.explicit_mode = None;
    let mut map = BTreeMap::new();
    map.insert("accent".into(), primary.hex());
    map.insert("background".into(), neutral.hex());
    map.insert(
        "foreground".into(),
        if relative_luminance(neutral) < 0.4 {
            Rgb::WHITE.hex()
        } else {
            Rgb::BLACK.hex()
        },
    );
    Ok(ParsedPalette {
        seeds,
        colors: map,
        theme: None,
        source: Some("hex-list".into()),
        background_file: None,
    })
}

fn parse_roles_json(data: &str) -> Result<ParsedPalette, GenerateError> {
    let value =
        json::parse(data).map_err(|err| bad(format!("role JSON could not be read: {err}")))?;
    let pairs = value
        .as_object()
        .ok_or_else(|| bad("role JSON must be an object"))?;
    let mut colors = BTreeMap::new();
    for (key, item) in pairs {
        let Some(value) = item.as_str() else {
            return Err(bad(format!("role '{key}' is not a hex color")));
        };
        let Some(rgb) = Rgb::parse(value) else {
            return Err(bad(format!("role '{key}' is not a hex color")));
        };
        colors.insert(key.clone(), rgb.hex());
    }
    let primary =
        take(&colors, &["primary", "accent"]).ok_or_else(|| bad("roles need primary or accent"))?;
    let bg =
        take(&colors, &["bg", "background"]).ok_or_else(|| bad("roles need bg or background"))?;
    let text = take(&colors, &["text", "foreground"]).unwrap_or(if relative_luminance(bg) < 0.4 {
        Rgb::WHITE
    } else {
        Rgb::BLACK
    });
    let accent = take(&colors, &["accent"]).unwrap_or(primary);
    let mut seeds = Seeds::chromatic(bg, primary, accent);
    seeds.text = Some(text);
    seeds.surface = take(&colors, &["surface"]);
    seeds.success = take(&colors, &["success"]);
    seeds.warn = take(&colors, &["warn", "warning"]);
    seeds.error = take(&colors, &["error"]);
    seeds.explicit_mode = Some(resolve_mode(None, bg));
    let mut out = BTreeMap::new();
    out.insert("accent".into(), primary.hex());
    out.insert("background".into(), bg.hex());
    out.insert("foreground".into(), text.hex());
    for (key, value) in &colors {
        out.entry(key.clone()).or_insert_with(|| value.clone());
    }
    Ok(ParsedPalette {
        seeds,
        colors: out,
        theme: None,
        source: Some("roles".into()),
        background_file: None,
    })
}

fn seeds_from_omarchy_map(colors: &BTreeMap<String, String>) -> Result<Seeds, GenerateError> {
    let accent = must(colors, "accent")?;
    let background = must(colors, "background")?;
    let foreground = must(colors, "foreground")?;
    let accent_role = opt(colors, "bright_magenta")
        .or_else(|| opt(colors, "magenta"))
        .unwrap_or(accent);
    let mut seeds = Seeds::chromatic(background, accent, accent_role);
    seeds.text = Some(foreground);
    seeds.surface = opt(colors, "lighter_background");
    seeds.surface2 = opt(colors, "dark_background");
    seeds.sidebar = opt(colors, "darker_background");
    seeds.text_muted = opt(colors, "dark_foreground").or_else(|| opt(colors, "muted"));
    seeds.outline = opt(colors, "muted");
    seeds.success = opt(colors, "green");
    seeds.warn = opt(colors, "yellow");
    seeds.error = opt(colors, "red");
    seeds.info = opt(colors, "blue");
    seeds.named = named_from_map(colors);
    if (0..16).all(|i| colors.contains_key(&format!("color{i}"))) {
        let mut ramp = [Rgb::BLACK; 16];
        for (i, slot) in ramp.iter_mut().enumerate() {
            *slot = must(colors, &format!("color{i}"))?;
        }
        seeds.ramp = Some(ramp);
    }
    Ok(seeds)
}

fn named_from_map(colors: &BTreeMap<String, String>) -> Vec<(&'static str, Rgb)> {
    const PAIRS: &[(&str, &str)] = &[
        ("red", "red"),
        ("green", "green"),
        ("yellow", "yellow"),
        ("blue", "blue"),
        ("magenta", "magenta"),
        ("cyan", "cyan"),
        ("orange", "orange"),
        ("brown", "brown"),
    ];
    PAIRS
        .iter()
        .filter_map(|(name, key)| opt(colors, key).map(|rgb| (*name, rgb)))
        .collect()
}

/// Same ANSI fill the Omarchy exporter uses when `color0`–`color15` are absent.
pub fn derive_ansi(colors: &mut BTreeMap<String, String>) {
    let pick = |colors: &BTreeMap<String, String>, keys: &[&str]| -> Option<String> {
        keys.iter().find_map(|key| colors.get(*key).cloned())
    };
    if !colors.contains_key("foreground") {
        if let Some(value) = pick(colors, &["color7"]) {
            colors.insert("foreground".into(), value);
        }
    }
    if !colors.contains_key("background") {
        if let Some(value) = pick(colors, &["color0"]) {
            colors.insert("background".into(), value);
        }
    }
    if !colors.contains_key("muted") {
        if let Some(value) = pick(colors, &["color8", "dark_foreground", "foreground"]) {
            colors.insert("muted".into(), value);
        }
    }
    if !colors.contains_key("bright_foreground") {
        if let Some(value) = pick(colors, &["color15", "foreground"]) {
            colors.insert("bright_foreground".into(), value);
        }
    }
    let pairs = [
        ("color0", "background"),
        ("color1", "red"),
        ("color2", "green"),
        ("color3", "yellow"),
        ("color4", "blue"),
        ("color5", "magenta"),
        ("color6", "cyan"),
        ("color7", "foreground"),
        ("color8", "muted"),
        ("color15", "bright_foreground"),
    ];
    for (dest, src) in pairs {
        if !colors.contains_key(dest) {
            if let Some(value) = colors.get(src).cloned() {
                colors.insert(dest.into(), value);
            }
        }
    }
    fill_bright(colors, "color9", &["bright_red", "red"]);
    fill_bright(colors, "color10", &["bright_green", "green"]);
    fill_bright(colors, "color11", &["bright_yellow", "yellow"]);
    fill_bright(colors, "color12", &["bright_blue", "blue"]);
    fill_bright(colors, "color13", &["bright_magenta", "magenta"]);
    fill_bright(colors, "color14", &["bright_cyan", "cyan"]);
}

fn fill_bright(colors: &mut BTreeMap<String, String>, dest: &str, keys: &[&str]) {
    if colors.contains_key(dest) {
        return;
    }
    if let Some(value) = keys.iter().find_map(|key| colors.get(*key).cloned()) {
        colors.insert(dest.into(), value);
    }
}

pub fn document_from_roles(
    theme: &str,
    source: Option<String>,
    mode: ColorMode,
    roles: &Roles,
    imported: &BTreeMap<String, String>,
    background_file: Option<String>,
) -> PaletteDocument {
    let mut colors = imported.clone();
    let put = |colors: &mut BTreeMap<String, String>, key: &str, rgb: Rgb| {
        colors.insert(key.into(), rgb.hex());
    };
    put(&mut colors, "accent", roles.primary);
    put(&mut colors, "background", roles.bg);
    put(&mut colors, "foreground", roles.text);
    put(&mut colors, "lighter_background", roles.surface);
    put(&mut colors, "dark_background", roles.surface2);
    put(&mut colors, "darker_background", roles.sidebar);
    put(&mut colors, "dark_foreground", roles.text_muted);
    put(&mut colors, "bright_foreground", roles.text);
    put(&mut colors, "muted", roles.outline);
    put(&mut colors, "red", roles.error);
    put(&mut colors, "green", roles.success);
    put(&mut colors, "yellow", roles.warn);
    put(&mut colors, "blue", roles.info);
    put(&mut colors, "magenta", roles.accent);
    let named = |key: &str| roles.named.iter().find(|(n, _)| *n == key).map(|(_, c)| *c);
    if let Some(cyan) = named("cyan") {
        put(&mut colors, "cyan", cyan);
    }
    if let Some(orange) = named("orange") {
        put(&mut colors, "orange", orange);
    }
    if let Some(brown) = named("brown") {
        put(&mut colors, "brown", brown);
    }
    for (i, color) in roles.ramp.iter().enumerate() {
        colors.insert(format!("color{i}"), color.hex());
    }
    for (key, value) in &mut colors {
        if let Some(rgb) = Rgb::parse(value) {
            *value = rgb.hex();
        }
        let _ = key;
    }
    PaletteDocument {
        schema: SCHEMA.into(),
        source,
        theme: theme.to_string(),
        mode: mode.id().into(),
        colors,
        background_file,
    }
}

pub fn to_json(doc: &PaletteDocument) -> String {
    debug_assert!(export_keys().contains(&"accent"));
    let mut out = String::from("{\n  \"schema\": ");
    json::push_string(&mut out, &doc.schema);
    if let Some(source) = &doc.source {
        out.push_str(",\n  \"source\": ");
        json::push_string(&mut out, source);
    }
    out.push_str(",\n  \"theme\": ");
    json::push_string(&mut out, &doc.theme);
    out.push_str(",\n  \"mode\": ");
    json::push_string(&mut out, &doc.mode);
    out.push_str(",\n  \"colors\": {");
    for (i, (key, value)) in doc.colors.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("\n    ");
        json::push_string(&mut out, key);
        out.push_str(": ");
        json::push_string(&mut out, value);
    }
    out.push_str("\n  }");
    if let Some(file) = &doc.background_file {
        out.push_str(",\n  \"background_file\": ");
        json::push_string(&mut out, file);
    }
    out.push_str("\n}\n");
    out
}

fn document_from_json(value: &Json) -> Result<PaletteDocument, GenerateError> {
    let schema = value
        .get("schema")
        .and_then(Json::as_str)
        .unwrap_or("")
        .to_string();
    let theme = value
        .get("theme")
        .and_then(Json::as_str)
        .unwrap_or("")
        .to_string();
    let mode = value
        .get("mode")
        .and_then(Json::as_str)
        .unwrap_or("")
        .to_string();
    let source = value
        .get("source")
        .and_then(Json::as_str)
        .map(str::to_string);
    let background_file = value
        .get("background_file")
        .and_then(Json::as_str)
        .map(str::to_string);
    let pairs = value
        .get("colors")
        .and_then(Json::as_object)
        .ok_or_else(|| bad("palette colors must be an object"))?;
    let mut colors = BTreeMap::new();
    for (key, item) in pairs {
        let Some(text) = item.as_str() else {
            return Err(bad(format!("color '{key}' must be #rrggbb")));
        };
        colors.insert(key.clone(), text.to_string());
    }
    Ok(PaletteDocument {
        schema,
        source,
        theme,
        mode,
        colors,
        background_file,
    })
}

fn normalize_colors(
    raw: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, GenerateError> {
    let mut colors = BTreeMap::new();
    for (key, value) in raw {
        let rgb = Rgb::parse(value).ok_or_else(|| bad(format!("color '{key}' must be #rrggbb")))?;
        colors.insert(key.clone(), rgb.hex());
    }
    Ok(colors)
}

fn require_core(colors: &BTreeMap<String, String>) -> Result<(), GenerateError> {
    for key in ["accent", "background", "foreground"] {
        if !colors.contains_key(key) {
            return Err(bad(format!("palette is missing {key}")));
        }
    }
    Ok(())
}

fn must(colors: &BTreeMap<String, String>, key: &str) -> Result<Rgb, GenerateError> {
    let value = colors
        .get(key)
        .ok_or_else(|| bad(format!("palette is missing {key}")))?;
    Rgb::parse(value).ok_or_else(|| bad(format!("color '{key}' must be #rrggbb")))
}

fn opt(colors: &BTreeMap<String, String>, key: &str) -> Option<Rgb> {
    colors.get(key).and_then(|value| Rgb::parse(value))
}

fn take(colors: &BTreeMap<String, String>, keys: &[&str]) -> Option<Rgb> {
    keys.iter().find_map(|key| opt(colors, key))
}

fn resolve_mode(mode: Option<&str>, background: Rgb) -> ColorMode {
    match mode {
        Some("dark") => ColorMode::Dark,
        Some("light") => ColorMode::Light,
        _ => {
            if relative_luminance(background) < 0.4 {
                ColorMode::Dark
            } else {
                ColorMode::Light
            }
        }
    }
}

fn strip_comment(line: &str) -> &str {
    let mut quote = None;
    for (i, c) in line.char_indices() {
        match c {
            '"' | '\'' => {
                if quote == Some(c) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(c);
                }
            }
            '#' if quote.is_none() => return &line[..i],
            _ => {}
        }
    }
    line
}

fn bad(reason: impl Into<String>) -> GenerateError {
    GenerateError::BadPalette {
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKYO: &str = include_str!("../../../omarchy/tests/fixtures/tokyo-night.colors.toml");

    #[test]
    fn tokyo_night_fixture_keeps_accent_and_an_unknown_key() {
        let parsed = parse_colors_toml(TOKYO).unwrap();
        assert_eq!(
            parsed.colors.get("accent").map(String::as_str),
            Some("#7aa2f7")
        );
        assert_eq!(
            parsed.colors.get("selection").map(String::as_str),
            Some("#292e42")
        );
        assert_eq!(parsed.seeds.explicit_mode, Some(ColorMode::Dark));
        assert_eq!(parsed.seeds.primary.hex(), "#7aa2f7");
        assert_eq!(
            parsed.colors.get("color0").map(String::as_str),
            Some("#1a1b26")
        );
        assert_eq!(
            parsed.colors.get("color1").map(String::as_str),
            Some("#f7768e")
        );
        assert_eq!(
            parsed.colors.get("color7").map(String::as_str),
            Some("#a9b1d6")
        );
        assert!(parsed.seeds.ramp.is_some());
    }

    #[test]
    fn palette_json_requires_schema_and_keeps_extra_keys() {
        let raw = r##"{
            "schema": "homechameleon-palette/1",
            "source": "omarchy",
            "theme": "tokyo-night",
            "mode": "dark",
            "colors": {"accent":"#7AA2F7","background":"#1a1b26","foreground":"#c0caf5","selection":"#292e42"},
            "background_file": "bg-3f9a1c2e.webp"
        }"##;
        let parsed = parse_document(raw).unwrap();
        assert_eq!(parsed.theme.as_deref(), Some("tokyo-night"));
        assert_eq!(parsed.source.as_deref(), Some("omarchy"));
        assert_eq!(parsed.background_file.as_deref(), Some("bg-3f9a1c2e.webp"));
        assert_eq!(
            parsed.colors.get("selection").map(String::as_str),
            Some("#292e42")
        );
        assert_eq!(parsed.seeds.primary.hex(), "#7aa2f7");
    }

    #[test]
    fn export_keys_follow_the_omarchy_hook() {
        assert_eq!(export_keys().len(), 41);
        assert_eq!(export_keys()[0], "accent");
        assert_eq!(*export_keys().last().unwrap(), "color15");
    }

    #[test]
    fn wrong_schema_is_rejected() {
        let raw = r##"{"schema":"other","theme":"x","mode":"dark","colors":{"accent":"#000000","background":"#111111","foreground":"#eeeeee"}}"##;
        assert!(parse_document(raw).is_err());
    }
}
