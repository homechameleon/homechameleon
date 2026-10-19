//! Light and dark roles. Tones come from Material tonal palettes.
//! While a tone is chosen it has to meet the WCAG ratio; nothing repairs it later.

use crate::color::{self, Rgb};
use crate::contrast::{self, meets};
use crate::mc::{from_mc, to_mc};
use crate::types::{ColorMode, ContrastTarget};
use material_colors::blend::harmonize;
use material_colors::hct::Hct;
use material_colors::palette::TonalPalette;

#[derive(Clone, Debug)]
pub struct Seeds {
    pub neutral: Rgb,
    pub primary: Rgb,
    pub accent: Rgb,
    pub text: Option<Rgb>,
    pub text_muted: Option<Rgb>,
    pub surface: Option<Rgb>,
    pub surface2: Option<Rgb>,
    pub sidebar: Option<Rgb>,
    pub outline: Option<Rgb>,
    pub success: Option<Rgb>,
    pub warn: Option<Rgb>,
    pub error: Option<Rgb>,
    pub info: Option<Rgb>,
    pub ramp: Option<[Rgb; 16]>,
    pub explicit_mode: Option<ColorMode>,
    pub named: Vec<(&'static str, Rgb)>,
}

impl Seeds {
    pub fn chromatic(neutral: Rgb, primary: Rgb, accent: Rgb) -> Self {
        Self {
            neutral,
            primary,
            accent,
            text: None,
            text_muted: None,
            surface: None,
            surface2: None,
            sidebar: None,
            outline: None,
            success: None,
            warn: None,
            error: None,
            info: None,
            ramp: None,
            explicit_mode: None,
            named: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Roles {
    pub bg: Rgb,
    pub surface: Rgb,
    pub surface2: Rgb,
    pub sidebar: Rgb,
    pub text: Rgb,
    pub text_muted: Rgb,
    pub outline: Rgb,
    pub primary: Rgb,
    pub on_primary: Rgb,
    pub accent: Rgb,
    pub success: Rgb,
    pub warn: Rgb,
    pub error: Rgb,
    pub info: Rgb,
    pub named: Vec<(&'static str, Rgb)>,
    pub ramp: [Rgb; 16],
}

pub fn assign(
    seeds: &Seeds,
    mode: ColorMode,
    target: ContrastTarget,
    saturation: f64,
    temperature: f64,
) -> (Roles, Vec<String>) {
    let seeds = tweak_seeds(seeds, saturation, temperature);
    let mut warnings = Vec::new();
    let mut roles = if seeds.explicit_mode == Some(mode) {
        from_explicit(&seeds, mode)
    } else {
        from_tones(&seeds, mode)
    };
    enforce(&mut roles, mode, target, &mut warnings);
    if seeds.named.is_empty() || seeds.explicit_mode != Some(mode) {
        roles.named = build_named(seeds.primary, roles.surface, mode, target.ui_ratio());
    } else {
        roles.named = fill_named(
            &seeds.named,
            seeds.primary,
            roles.surface,
            mode,
            target.ui_ratio(),
        );
    }
    roles.ramp = if seeds.explicit_mode == Some(mode) {
        seeds.ramp.unwrap_or_else(|| build_ramp(&roles))
    } else {
        build_ramp(&roles)
    };
    (roles, warnings)
}

pub fn assign_material(
    seeds: &Seeds,
    mode: ColorMode,
    target: ContrastTarget,
    saturation: f64,
    temperature: f64,
) -> (Roles, Vec<String>) {
    // Material uses the same tonal roles as the other styles. The style
    // itself is the elevation, the radius, and the lack of blur.
    assign(seeds, mode, target, saturation, temperature)
}

fn tweak_seeds(seeds: &Seeds, saturation: f64, temperature: f64) -> Seeds {
    if saturation == 0.0 && temperature == 0.0 {
        return seeds.clone();
    }
    let mut next = seeds.clone();
    let tweak = |c: Rgb| color::apply_saturation_temperature(c, saturation, temperature);
    next.primary = tweak(next.primary);
    next.accent = tweak(next.accent);
    next.success = next.success.map(tweak);
    next.warn = next.warn.map(tweak);
    next.error = next.error.map(tweak);
    next.info = next.info.map(tweak);
    next
}

fn from_tones(seeds: &Seeds, mode: ColorMode) -> Roles {
    let dark = mode.is_dark();
    let neutral = neutral_palette(seeds.neutral);
    let (bg_t, surface_t, surface2_t, side_t, text_t, muted_t, outline_t, ui_t, on_t) = if dark {
        (12, 18, 24, 8, 96, 78, 42, 80, 20)
    } else {
        (98, 94, 90, 96, 12, 34, 62, 40, 100)
    };
    let primary = tone(&pal(seeds.primary), ui_t);
    Roles {
        bg: tone(&neutral, bg_t),
        surface: tone(&neutral, surface_t),
        surface2: tone(&neutral, surface2_t),
        sidebar: tone(&neutral, side_t),
        text: tone(&neutral, text_t),
        text_muted: tone(&neutral, muted_t),
        outline: tone(&neutral, outline_t),
        primary,
        on_primary: tone(&pal(seeds.primary), on_t),
        accent: tone(&pal(seeds.accent), ui_t),
        success: tone(
            &pal(seeds
                .success
                .unwrap_or_else(|| state_anchor(145.0, 64.0, seeds.primary))),
            ui_t,
        ),
        warn: tone(
            &pal(seeds
                .warn
                .unwrap_or_else(|| state_anchor(85.0, 72.0, seeds.primary))),
            ui_t,
        ),
        error: tone(
            &pal(seeds
                .error
                .unwrap_or_else(|| state_anchor(25.0, 76.0, seeds.primary))),
            ui_t,
        ),
        info: tone(
            &pal(seeds
                .info
                .unwrap_or_else(|| state_anchor(250.0, 52.0, seeds.primary))),
            ui_t,
        ),
        named: Vec::new(),
        ramp: [Rgb::BLACK; 16],
    }
}

fn from_explicit(seeds: &Seeds, mode: ColorMode) -> Roles {
    let mut roles = from_tones(seeds, mode);
    roles.bg = seeds.neutral;
    if let Some(surface) = seeds.surface {
        roles.surface = surface;
    }
    if let Some(surface2) = seeds.surface2 {
        roles.surface2 = surface2;
    }
    if let Some(sidebar) = seeds.sidebar {
        roles.sidebar = sidebar;
    }
    if let Some(text) = seeds.text {
        roles.text = text;
    }
    if let Some(muted) = seeds.text_muted {
        roles.text_muted = muted;
    }
    if let Some(outline) = seeds.outline {
        roles.outline = outline;
    }
    roles.primary = seeds.primary;
    roles.accent = seeds.accent;
    if let Some(success) = seeds.success {
        roles.success = success;
    }
    if let Some(warn) = seeds.warn {
        roles.warn = warn;
    }
    if let Some(error) = seeds.error {
        roles.error = error;
    }
    if let Some(info) = seeds.info {
        roles.info = info;
    }
    // Prefer the imported background as the label on the primary control when it already contrasts.
    roles.on_primary = seeds.neutral;
    roles
}

fn enforce(roles: &mut Roles, mode: ColorMode, target: ContrastTarget, warnings: &mut Vec<String>) {
    let text_min = target.text_ratio();
    let ui_min = target.ui_ratio();
    let prefer_light = mode.is_dark();
    retone(
        &mut roles.text,
        &[roles.surface, roles.bg],
        text_min,
        prefer_light,
        "text",
        warnings,
    );
    retone(
        &mut roles.text_muted,
        &[roles.surface],
        text_min,
        prefer_light,
        "muted text",
        warnings,
    );
    retone(
        &mut roles.primary,
        &[roles.surface],
        ui_min,
        prefer_light,
        "primary",
        warnings,
    );
    retone(
        &mut roles.accent,
        &[roles.surface],
        ui_min,
        prefer_light,
        "accent",
        warnings,
    );
    retone(
        &mut roles.success,
        &[roles.surface],
        ui_min,
        prefer_light,
        "success",
        warnings,
    );
    retone(
        &mut roles.warn,
        &[roles.surface],
        ui_min,
        prefer_light,
        "warning",
        warnings,
    );
    retone(
        &mut roles.error,
        &[roles.surface],
        ui_min,
        prefer_light,
        "error",
        warnings,
    );
    retone(
        &mut roles.info,
        &[roles.surface],
        ui_min,
        prefer_light,
        "info",
        warnings,
    );
    fit_primary_label(roles, text_min, ui_min, warnings);
}

/// Pick a black or white label, and move the primary tone if that is what it
/// takes for the label to meet the text ratio. A mid primary cannot host a
/// 7:1 label, so AAA has to choose the tone with the label in mind.
fn fit_primary_label(roles: &mut Roles, text_min: f64, ui_min: f64, warnings: &mut Vec<String>) {
    if meets(roles.primary, roles.surface, ui_min) {
        if let Some(label) = label_for(roles.primary, text_min) {
            roles.on_primary = label;
            return;
        }
    }
    let hct: Hct = to_mc(roles.primary).into();
    let palette = TonalPalette::from_hct(hct);
    let start = hct.get_tone().round().clamp(0.0, 100.0) as i32;
    let mut order: Vec<i32> = (0..=100).collect();
    order.sort_by_key(|tone| (tone - start).abs());
    let original = roles.primary;
    for tone_value in order {
        let candidate = from_mc(palette.tone(tone_value));
        if !meets(candidate, roles.surface, ui_min) {
            continue;
        }
        if let Some(label) = label_for(candidate, text_min) {
            if candidate != original {
                warnings.push(
                    "Chose a different primary tone so text on it meets WCAG contrast while the theme is built."
                        .into(),
                );
            }
            roles.primary = candidate;
            roles.on_primary = label;
            return;
        }
    }
    let dark = from_mc(palette.tone(0));
    let light = from_mc(palette.tone(100));
    if contrast::contrast_ratio(Rgb::WHITE, dark) >= contrast::contrast_ratio(Rgb::BLACK, light) {
        roles.primary = dark;
        roles.on_primary = Rgb::WHITE;
    } else {
        roles.primary = light;
        roles.on_primary = Rgb::BLACK;
    }
    warnings.push(
        "Chose a different primary tone so text on it meets WCAG contrast while the theme is built.".into(),
    );
}

fn label_for(primary: Rgb, text_min: f64) -> Option<Rgb> {
    let white = contrast::contrast_ratio(Rgb::WHITE, primary);
    let black = contrast::contrast_ratio(Rgb::BLACK, primary);
    let (label, ratio) = if white >= black {
        (Rgb::WHITE, white)
    } else {
        (Rgb::BLACK, black)
    };
    if ratio + 1e-9 >= text_min {
        Some(label)
    } else {
        None
    }
}

fn retone(
    color: &mut Rgb,
    against: &[Rgb],
    minimum: f64,
    prefer_lighter: bool,
    label: &str,
    warnings: &mut Vec<String>,
) {
    if against.iter().all(|bg| meets(*color, *bg, minimum)) {
        return;
    }
    let next = walk(*color, against, minimum, prefer_lighter);
    if next != *color {
        warnings.push(format!(
            "Chose a different {label} tone so it meets WCAG contrast while the theme is built."
        ));
        *color = next;
    }
}

fn walk(original: Rgb, against: &[Rgb], minimum: f64, prefer_lighter: bool) -> Rgb {
    let hct: Hct = to_mc(original).into();
    let palette = TonalPalette::from_hct(hct);
    let start = hct.get_tone().round().clamp(0.0, 100.0) as i32;
    let order: Vec<i32> = if prefer_lighter {
        (start..=100).chain((0..start).rev()).collect()
    } else {
        (0..=start).rev().chain((start + 1)..=100).collect()
    };
    for tone_value in order {
        let color = from_mc(palette.tone(tone_value));
        if against.iter().all(|bg| meets(color, *bg, minimum)) {
            return color;
        }
    }
    let white_score = against
        .iter()
        .map(|bg| contrast::contrast_ratio(Rgb::WHITE, *bg))
        .fold(f64::MAX, f64::min);
    let black_score = against
        .iter()
        .map(|bg| contrast::contrast_ratio(Rgb::BLACK, *bg))
        .fold(f64::MAX, f64::min);
    if white_score >= black_score {
        Rgb::WHITE
    } else {
        Rgb::BLACK
    }
}

fn neutral_palette(neutral: Rgb) -> TonalPalette {
    let hct: Hct = to_mc(neutral).into();
    let mut chroma = hct.get_chroma().min(16.0);
    loop {
        let sample = from_mc(material_colors::color::Rgb::from(Hct::from(
            hct.get_hue(),
            chroma,
            50.0,
        )));
        if color::oklab_from_rgb(sample).chroma() <= 0.03 || chroma <= 1.0 {
            break;
        }
        chroma -= 1.0;
    }
    TonalPalette::from_hct(Hct::from(hct.get_hue(), chroma, hct.get_tone()))
}

fn pal(rgb: Rgb) -> TonalPalette {
    TonalPalette::from_hct(to_mc(rgb).into())
}

fn tone(palette: &TonalPalette, value: i32) -> Rgb {
    from_mc(palette.tone(value.clamp(0, 100)))
}

pub fn state_anchor(hue: f64, chroma: f64, primary: Rgb) -> Rgb {
    let anchor = from_mc(material_colors::color::Rgb::from(Hct::from(
        hue, chroma, 50.0,
    )));
    from_mc(harmonize(to_mc(anchor), to_mc(primary)))
}

const NAMED_HUES: &[(&str, f64, f64)] = &[
    ("red", 25.0, 80.0),
    ("pink", 350.0, 60.0),
    ("purple", 310.0, 52.0),
    ("deep-purple", 282.0, 56.0),
    ("indigo", 265.0, 50.0),
    ("blue", 250.0, 56.0),
    ("light-blue", 230.0, 50.0),
    ("cyan", 195.0, 46.0),
    ("teal", 175.0, 46.0),
    ("green", 145.0, 56.0),
    ("light-green", 125.0, 62.0),
    ("lime", 100.0, 72.0),
    ("yellow", 95.0, 78.0),
    ("amber", 78.0, 72.0),
    ("orange", 55.0, 76.0),
    ("deep-orange", 35.0, 80.0),
    ("brown", 30.0, 28.0),
    ("blue-grey", 230.0, 16.0),
];

fn build_named(
    primary: Rgb,
    surface: Rgb,
    mode: ColorMode,
    minimum: f64,
) -> Vec<(&'static str, Rgb)> {
    let mut named = Vec::new();
    let prefer_light = mode.is_dark();
    let start = if prefer_light { 80 } else { 42 };
    for (name, hue, chroma) in NAMED_HUES {
        let seed = state_anchor(*hue, *chroma, primary);
        let color = walk(tone(&pal(seed), start), &[surface], minimum, prefer_light);
        named.push((*name, color));
    }
    let grey_start = if prefer_light { 70 } else { 50 };
    let grey = walk(
        tone(&neutral_palette(Rgb::new(128, 128, 128)), grey_start),
        &[surface],
        minimum,
        prefer_light,
    );
    named.push(("grey", grey));
    named.push((
        "light-grey",
        tone(
            &neutral_palette(Rgb::new(180, 180, 180)),
            if prefer_light { 80 } else { 74 },
        ),
    ));
    named.push((
        "dark-grey",
        tone(
            &neutral_palette(Rgb::new(40, 40, 40)),
            if prefer_light { 40 } else { 32 },
        ),
    ));
    named
}

fn fill_named(
    given: &[(&'static str, Rgb)],
    primary: Rgb,
    surface: Rgb,
    mode: ColorMode,
    minimum: f64,
) -> Vec<(&'static str, Rgb)> {
    let generated = build_named(primary, surface, mode, minimum);
    generated
        .into_iter()
        .map(|(name, fallback)| {
            let color = given
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, c)| *c)
                .unwrap_or(fallback);
            (name, color)
        })
        .collect()
}

pub fn tone_of(rgb: Rgb, value: i32) -> Rgb {
    tone(&pal(rgb), value)
}

pub fn named_color(roles: &Roles, key: &str) -> Rgb {
    roles
        .named
        .iter()
        .find(|(n, _)| *n == key)
        .map(|(_, c)| *c)
        .unwrap_or(Rgb::BLACK)
}

pub fn build_ramp(roles: &Roles) -> [Rgb; 16] {
    let cyan = named_color(roles, "cyan");
    let lift = |c: Rgb| {
        let hct: Hct = to_mc(c).into();
        let next = (hct.get_tone() + 12.0).clamp(0.0, 100.0) as i32;
        tone(&pal(c), next)
    };
    [
        roles.bg,
        roles.error,
        roles.success,
        roles.warn,
        roles.info,
        roles.accent,
        cyan,
        roles.text,
        roles.text_muted,
        lift(roles.error),
        lift(roles.success),
        lift(roles.warn),
        lift(roles.info),
        lift(roles.accent),
        lift(cyan),
        lift(roles.text),
    ]
}
