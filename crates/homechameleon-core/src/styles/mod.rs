//! Launch styles. Each one writes the same variable names the preview CSS uses.

mod bubbles;
mod flat;
mod material;
mod sheer;
mod starship;

use std::collections::BTreeMap;

use crate::color::Rgb;
use crate::contrast::{self, percentile_backdrop};
use crate::roles::{self, Roles};
use crate::types::{BackgroundOptions, ColorMode, StyleId, ThemeOptions, Tweaks};
use crate::yaml::ThemeValue;

pub struct BuiltMode {
    pub vars: BTreeMap<String, ThemeValue>,
    pub card_alpha: f64,
}

pub struct ModeInput<'a> {
    pub style: StyleId,
    pub slug: &'a str,
    pub roles: &'a Roles,
    pub mode: ColorMode,
    pub tweaks: Tweaks,
    pub options: &'a ThemeOptions,
    pub background: &'a BackgroundOptions,
    pub backdrop_rgba: Option<&'a [u8]>,
    pub backdrop_size: Option<(u32, u32)>,
    pub file_name: Option<&'a str>,
}

pub fn build_mode(input: &ModeInput<'_>, warnings: &mut Vec<String>) -> BuiltMode {
    let style = input.style;
    let slug = input.slug;
    let roles = input.roles;
    let mode = input.mode;
    let tweaks = input.tweaks;
    let options = input.options;
    let background = input.background;
    let file_name = input.file_name;
    let mut vars = BTreeMap::new();
    let alpha = card_alpha(input, warnings);
    paint_colors(&mut vars, style, roles, mode, alpha);
    paint_shape(&mut vars, style, mode, tweaks, alpha);
    paint_named(&mut vars, roles);
    paint_state(&mut vars, roles);
    paint_type(&mut vars, style);
    if style == StyleId::Starship {
        paint_console(&mut vars, roles, tweaks);
    }
    if options.bubble {
        paint_bubble(&mut vars, style, roles, tweaks);
    }
    if options.mushroom {
        paint_mushroom(&mut vars, style, roles, tweaks);
    }
    if background.enabled {
        if let Some(name) = file_name {
            vars.insert(
                "lovelace-background".into(),
                ThemeValue::Scalar(format!(
                    "center / cover no-repeat fixed url(\"/local/homechameleon/{name}\")"
                )),
            );
            if background.blur > 0 || background.dim > 0.0 {
                warnings.push(
                    "Background blur and dim are left to the image encoder. This file only references the image.".into(),
                );
            }
        } else {
            warnings.push(
                "Background is enabled, but no image was provided, so the theme does not reference a wallpaper.".into(),
            );
        }
    }
    if options.card_mod {
        if let Some(css) = card_mod_css(style) {
            vars.insert("card-mod-theme".into(), ThemeValue::Scalar(slug.into()));
            vars.insert("card-mod-card".into(), ThemeValue::Block(css.to_string()));
        }
    }
    BuiltMode {
        vars,
        card_alpha: alpha,
    }
}

fn card_alpha(input: &ModeInput<'_>, warnings: &mut Vec<String>) -> f64 {
    let style = input.style;
    let roles = input.roles;
    let tweaks = input.tweaks;
    let background = input.background;
    let backdrop_rgba = input.backdrop_rgba;
    let backdrop_size = input.backdrop_size;
    let preferred = 1.0 - tweaks.transparency / 100.0;
    if preferred >= 0.999 {
        return 1.0;
    }
    let (backdrop, note) = backdrop_for(style, roles, background, backdrop_rgba, backdrop_size);
    if let Some(note) = note {
        warnings.push(note);
    }
    let text_min = input.options.contrast.text_ratio();
    let checks = [(roles.text, text_min), (roles.text_muted, text_min)];
    let raised = contrast::raise_alpha(roles.surface, backdrop, preferred, &checks);
    if raised > preferred + 0.005 {
        warnings.push(
            "Raised card opacity so text on the translucent card meets WCAG contrast.".into(),
        );
    }
    raised
}

fn backdrop_for(
    style: StyleId,
    roles: &Roles,
    background: &BackgroundOptions,
    rgba: Option<&[u8]>,
    size: Option<(u32, u32)>,
) -> (Rgb, Option<String>) {
    if background.enabled {
        if let (Some(bytes), Some((w, h))) = (rgba, size) {
            if let Some(gray) = percentile_backdrop(bytes, w, h) {
                return (gray, None);
            }
        }
        return (
            roles.bg,
            Some("Background is enabled, but no image was provided, so contrast was checked against the solid background.".into()),
        );
    }
    let note = if style == StyleId::Sheer {
        Some("Sheer is translucent. With the background off, contrast was checked against the solid background color.".into())
    } else {
        None
    };
    (roles.bg, note)
}

fn paint_colors(
    vars: &mut BTreeMap<String, ThemeValue>,
    style: StyleId,
    roles: &Roles,
    mode: ColorMode,
    alpha: f64,
) {
    let (bg, surface, surface2, sidebar) = console_surfaces(style, roles, mode);
    let card_rgb = surface;
    let card = if alpha >= 0.999 {
        card_rgb.hex()
    } else {
        rgba(card_rgb, alpha)
    };
    let divider_alpha = if style == StyleId::Flat { 0.38 } else { 0.12 };
    set(vars, "primary-color", roles.primary.hex());
    set(vars, "accent-color", roles.accent.hex());
    set(vars, "primary-background-color", bg.hex());
    set(vars, "secondary-background-color", surface2.hex());
    set(vars, "card-background-color", card.clone());
    set(vars, "ha-card-background", card);
    set(vars, "primary-text-color", roles.text.hex());
    set(vars, "secondary-text-color", roles.text_muted.hex());
    set(vars, "text-primary-color", roles.on_primary.hex());
    set(
        vars,
        "disabled-text-color",
        roles::named_color(roles, "grey").hex(),
    );
    set(vars, "divider-color", rgba(roles.text, divider_alpha));
    set(vars, "outline-color", roles.outline.hex());
    set(vars, "app-header-background-color", sidebar.hex());
    set(vars, "app-header-text-color", roles.text.hex());
    set(vars, "sidebar-background-color", sidebar.hex());
    set(vars, "sidebar-text-color", roles.text.hex());
    set(vars, "sidebar-selected-text-color", roles.primary.hex());
    set(vars, "sidebar-selected-icon-color", roles.primary.hex());
    set(vars, "sidebar-icon-color", rgba(roles.text, 0.60));
    set(vars, "success-color", roles.success.hex());
    set(vars, "warning-color", roles.warn.hex());
    set(vars, "error-color", roles.error.hex());
    set(vars, "info-color", roles.info.hex());
    set(vars, "rgb-primary-color", roles.primary.triplet());
    set(vars, "rgb-accent-color", roles.accent.triplet());
    set(vars, "rgb-primary-text-color", roles.text.triplet());
    set(vars, "rgb-secondary-text-color", roles.text_muted.triplet());
    set(vars, "rgb-text-primary-color", roles.on_primary.triplet());
    set(vars, "rgb-card-background-color", card_rgb.triplet());
    set(vars, "rgb-success-color", roles.success.triplet());
    set(vars, "rgb-warning-color", roles.warn.triplet());
    set(vars, "rgb-error-color", roles.error.triplet());
    set(vars, "rgb-info-color", roles.info.triplet());
    set(
        vars,
        "light-primary-color",
        roles::tone_of(roles.primary, 80).hex(),
    );
    set(
        vars,
        "dark-primary-color",
        roles::tone_of(roles.primary, 30).hex(),
    );
    set(vars, "black-color", "#000000");
    set(vars, "white-color", "#ffffff");
}

fn paint_shape(
    vars: &mut BTreeMap<String, ThemeValue>,
    style: StyleId,
    mode: ColorMode,
    tweaks: Tweaks,
    alpha: f64,
) {
    let round = tweaks.roundness.round() as i32;
    let (radius, sm, md, lg, xl, pill): (String, String, String, String, String, String) =
        match style {
            StyleId::Sheer => {
                let radius = format!("{round}px");
                let control = format!("{}", (tweaks.roundness * 0.65).round() as i32);
                (
                    radius.clone(),
                    "8px".into(),
                    format!("{control}px"),
                    radius.clone(),
                    radius,
                    "999px".into(),
                )
            }
            StyleId::Bubbles => {
                let radius = format!("{round}px");
                (
                    radius.clone(),
                    "18px".into(),
                    "999px".into(),
                    radius.clone(),
                    radius,
                    "999px".into(),
                )
            }
            StyleId::Material => {
                let card = material::card_radius(tweaks.roundness);
                (
                    format!("{card}px"),
                    "8px".into(),
                    "12px".into(),
                    format!("{card}px"),
                    "28px".into(),
                    "999px".into(),
                )
            }
            StyleId::Flat => {
                let radius = format!("{round}px");
                (
                    radius.clone(),
                    "0px".into(),
                    "0px".into(),
                    radius.clone(),
                    radius,
                    "0px".into(),
                )
            }
            StyleId::Starship => (
                format!("0px {round}px {round}px {round}px"),
                "4px".into(),
                "999px".into(),
                format!("{round}px"),
                format!("{round}px"),
                "999px".into(),
            ),
        };
    set(vars, "ha-card-border-radius", radius);
    set(vars, "ha-border-radius-sm", sm);
    set(vars, "ha-border-radius-md", md);
    set(vars, "ha-border-radius-lg", lg);
    set(vars, "ha-border-radius-xl", xl);
    set(vars, "ha-border-radius-pill", pill);
    let (shadow, border, border_width, filter, dialog): (
        String,
        String,
        String,
        Option<String>,
        Option<String>,
    ) = match style {
        StyleId::Sheer => (
            sheer::shadow(tweaks.glow, mode.is_dark()),
            sheer::border(tweaks.glow),
            "1px".into(),
            (tweaks.blur > 0.0).then(|| sheer::backdrop(tweaks.blur)),
            Some(sheer::backdrop(tweaks.blur.max(8.0))),
        ),
        StyleId::Bubbles => (
            bubbles::shadow(tweaks.glow, mode.is_dark()),
            "transparent".into(),
            "0px".into(),
            None,
            None,
        ),
        StyleId::Material => (
            material::shadow(tweaks.glow, mode.is_dark()),
            "transparent".into(),
            "0px".into(),
            None,
            None,
        ),
        StyleId::Flat => (
            flat::shadow(),
            "transparent".into(),
            "0px".into(),
            None,
            None,
        ),
        StyleId::Starship => (
            "none".into(),
            "transparent".into(),
            "0px".into(),
            None,
            None,
        ),
    };
    set(vars, "ha-card-box-shadow", shadow);
    set(vars, "ha-card-border-color", border);
    set(vars, "ha-card-border-width", border_width);
    if let Some(filter) = filter {
        if alpha < 0.999 {
            set(vars, "ha-card-backdrop-filter", filter);
        }
    }
    if let Some(dialog) = dialog {
        set(vars, "ha-dialog-scrim-backdrop-filter", dialog);
    }
}

fn paint_named(vars: &mut BTreeMap<String, ThemeValue>, roles: &Roles) {
    for (name, color) in &roles.named {
        set(vars, &format!("{name}-color"), color.hex());
    }
    let get = |key: &str| roles::named_color(roles, key);
    set(vars, "energy-grid-consumption-color", get("blue").hex());
    set(vars, "energy-solar-color", get("amber").hex());
    set(vars, "energy-non-fossil-color", get("green").hex());
    set(vars, "energy-battery-out-color", get("teal").hex());
    set(vars, "energy-battery-in-color", get("pink").hex());
    set(vars, "energy-gas-color", get("deep-orange").hex());
    set(vars, "energy-water-color", get("cyan").hex());
    set(vars, "codemirror-keyword", get("purple").hex());
    set(vars, "codemirror-operator", get("cyan").hex());
    set(vars, "codemirror-variable", roles.text.hex());
    set(vars, "codemirror-variable-2", get("blue").hex());
    set(vars, "codemirror-variable-3", get("teal").hex());
    set(vars, "codemirror-builtin", get("indigo").hex());
    set(vars, "codemirror-atom", get("purple").hex());
    set(vars, "codemirror-number", get("red").hex());
    set(vars, "codemirror-def", get("blue").hex());
    set(vars, "codemirror-string", get("green").hex());
    set(vars, "codemirror-string-2", get("teal").hex());
    set(vars, "codemirror-comment", get("grey").hex());
    set(vars, "codemirror-tag", get("red").hex());
    set(vars, "codemirror-meta", get("deep-purple").hex());
    set(vars, "codemirror-attribute", get("orange").hex());
    set(vars, "codemirror-property", get("blue").hex());
    set(vars, "codemirror-qualifier", get("cyan").hex());
    set(vars, "codemirror-type", get("amber").hex());
}

fn paint_state(vars: &mut BTreeMap<String, ThemeValue>, roles: &Roles) {
    let get = |key: &str| roles::named_color(roles, key);
    set(vars, "state-active-color", get("amber").hex());
    set(vars, "state-inactive-color", get("grey").hex());
    set(vars, "state-unavailable-color", get("dark-grey").hex());
    set(vars, "state-light-active-color", get("amber").hex());
    set(vars, "state-climate-heat-color", get("deep-orange").hex());
    set(vars, "state-climate-cool-color", get("blue").hex());
    set(vars, "state-lock-locked-color", get("green").hex());
    set(vars, "state-lock-unlocked-color", get("red").hex());
    set(vars, "state-lock-jammed-color", get("red").hex());
    set(vars, "state-cover-active-color", get("purple").hex());
    set(vars, "state-person-home-color", get("green").hex());
}

fn paint_type(vars: &mut BTreeMap<String, ThemeValue>, style: StyleId) {
    let (body, heading, code, sizes, weights, line) = match style {
        StyleId::Sheer => (
            "\"Segoe UI\", Noto, sans-serif",
            "\"Segoe UI\", Noto, sans-serif",
            "ui-monospace, monospace",
            ("12px", "14px", "16px", "22px", "28px"),
            ("400", "500", "600"),
            "1.45",
        ),
        StyleId::Bubbles => (
            "ui-rounded, Nunito, sans-serif",
            "ui-rounded, Nunito, sans-serif",
            "ui-monospace, monospace",
            ("13px", "16px", "18px", "26px", "34px"),
            ("500", "700", "800"),
            "1.3",
        ),
        StyleId::Material => (
            "Roboto, Noto, sans-serif",
            "Roboto, Noto, sans-serif",
            "\"Roboto Mono\", ui-monospace, monospace",
            ("12px", "14px", "16px", "22px", "28px"),
            ("400", "500", "700"),
            "1.5",
        ),
        StyleId::Flat => (
            "\"Segoe UI\", \"Helvetica Neue\", sans-serif",
            "\"Segoe UI\", \"Helvetica Neue\", sans-serif",
            "ui-monospace, monospace",
            ("13px", "16px", "20px", "32px", "44px"),
            ("500", "700", "800"),
            "1.2",
        ),
        StyleId::Starship => (
            "Antonio, \"Arial Narrow\", sans-serif",
            "Antonio, \"Arial Narrow\", sans-serif",
            "Antonio, \"Arial Narrow\", sans-serif",
            ("12px", "14px", "16px", "22px", "28px"),
            ("700", "700", "700"),
            "1.15",
        ),
    };
    set(vars, "ha-font-family-body", body);
    set(vars, "ha-font-family-heading", heading);
    set(vars, "ha-font-family-code", code);
    set(vars, "ha-font-size-s", sizes.0);
    set(vars, "ha-font-size-m", sizes.1);
    set(vars, "ha-font-size-l", sizes.2);
    set(vars, "ha-font-size-xl", sizes.3);
    set(vars, "ha-font-size-2xl", sizes.4);
    set(vars, "ha-font-weight-normal", weights.0);
    set(vars, "ha-font-weight-medium", weights.1);
    set(vars, "ha-font-weight-bold", weights.2);
    set(vars, "ha-line-height-normal", line);
}

fn paint_bubble(
    vars: &mut BTreeMap<String, ThemeValue>,
    style: StyleId,
    roles: &Roles,
    tweaks: Tweaks,
) {
    let radius = match style {
        StyleId::Flat => "0px".into(),
        StyleId::Material => format!("{}px", material::card_radius(tweaks.roundness)),
        StyleId::Starship => "999px".into(),
        StyleId::Bubbles | StyleId::Sheer => format!("{}px", tweaks.roundness.round() as i32),
    };
    set(vars, "bubble-border-radius", radius);
    set(vars, "bubble-accent-color", roles.primary.hex());
    set(
        vars,
        "bubble-icon-background-color",
        rgba(roles.primary, 0.18),
    );
    set(vars, "bubble-main-background-color", roles.surface.hex());
    set(
        vars,
        "bubble-secondary-background-color",
        roles.surface2.hex(),
    );
    let control = match style {
        StyleId::Flat => "0px",
        StyleId::Material => "20px",
        StyleId::Sheer | StyleId::Bubbles | StyleId::Starship => "999px",
    };
    set(vars, "bubble-button-border-radius", control);
    set(vars, "bubble-icon-border-radius", control);
    set(vars, "bubble-select-arrow", "0px");
}

fn paint_mushroom(
    vars: &mut BTreeMap<String, ThemeValue>,
    style: StyleId,
    roles: &Roles,
    tweaks: Tweaks,
) {
    let radius = match style {
        StyleId::Flat => "0px".into(),
        StyleId::Material => "12px".into(),
        StyleId::Starship | StyleId::Bubbles => "999px".into(),
        StyleId::Sheer => {
            let r = tweaks.roundness;
            if r >= 24.0 {
                "999px".into()
            } else {
                format!("{}px", (r * 0.5).round() as i32)
            }
        }
    };
    let spacing = match style {
        StyleId::Flat => "16px",
        StyleId::Bubbles => "12px",
        StyleId::Starship => "6px",
        StyleId::Sheer | StyleId::Material => "8px",
    };
    set(vars, "mush-rgb-primary", roles.primary.triplet());
    set(vars, "mush-rgb-state-entity", roles.accent.triplet());
    set(
        vars,
        "mush-rgb-state-light",
        roles::named_color(roles, "amber").triplet(),
    );
    set(
        vars,
        "mush-rgb-state-fan",
        roles::named_color(roles, "cyan").triplet(),
    );
    set(
        vars,
        "mush-rgb-state-cover",
        roles::named_color(roles, "purple").triplet(),
    );
    set(vars, "mush-rgb-state-lock", roles.success.triplet());
    set(vars, "mush-rgb-state-alarm", roles.error.triplet());
    set(
        vars,
        "mush-rgb-state-climate",
        roles::named_color(roles, "deep-orange").triplet(),
    );
    set(vars, "mush-rgb-state-person", roles.success.triplet());
    set(vars, "mush-card-primary-font-size", "14px");
    set(vars, "mush-card-secondary-font-size", "12px");
    set(vars, "mush-card-primary-line-height", "1.4");
    set(vars, "mush-icon-border-radius", radius);
    set(vars, "mush-control-border-radius", "999px");
    set(vars, "mush-chip-border-radius", "999px");
    set(vars, "mush-spacing", spacing);
}

fn paint_console(vars: &mut BTreeMap<String, ThemeValue>, roles: &Roles, tweaks: Tweaks) {
    let blocks = starship::console_blocks(roles.primary);
    set(vars, "console-orange", blocks.orange.hex());
    set(vars, "console-peach", blocks.peach.hex());
    set(vars, "console-lavender", blocks.lavender.hex());
    set(vars, "console-blue", blocks.blue.hex());
    set(vars, "console-red", blocks.red.hex());
    set(vars, "console-ink", blocks.ink.hex());
    set(
        vars,
        "console-cap",
        format!("{}px", tweaks.roundness.round() as i32),
    );
    let bar = starship::bar_thickness(tweaks.glow);
    set(vars, "console-bar", format!("{bar}px"));
    set(
        vars,
        "ha-card-box-shadow",
        format!(
            "inset 14px 0 0 {}, inset 0 {bar}px 0 {}",
            blocks.blue.hex(),
            blocks.orange.hex()
        ),
    );
}

fn console_surfaces(style: StyleId, roles: &Roles, mode: ColorMode) -> (Rgb, Rgb, Rgb, Rgb) {
    if style != StyleId::Starship {
        return (roles.bg, roles.surface, roles.surface2, roles.sidebar);
    }
    if mode.is_dark() {
        (
            mix(roles.bg, Rgb::BLACK, 0.86),
            mix(roles.surface, Rgb::BLACK, 0.78),
            mix(roles.surface2, Rgb::BLACK, 0.62),
            mix(roles.sidebar, Rgb::BLACK, 0.92),
        )
    } else {
        (
            mix(roles.bg, Rgb::WHITE, 0.55),
            mix(roles.surface, Rgb::WHITE, 0.35),
            mix(roles.surface2, Rgb::WHITE, 0.2),
            mix(roles.sidebar, Rgb::WHITE, 0.4),
        )
    }
}

fn mix(a: Rgb, b: Rgb, toward_b: f64) -> Rgb {
    let t = toward_b.clamp(0.0, 1.0);
    let channel = |from: u8, to: u8| -> u8 {
        (f64::from(from) * (1.0 - t) + f64::from(to) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Rgb::new(channel(a.r, b.r), channel(a.g, b.g), channel(a.b, b.b))
}

fn card_mod_css(style: StyleId) -> Option<&'static str> {
    Some(match style {
        StyleId::Sheer => sheer::card_mod_css(),
        StyleId::Bubbles => bubbles::card_mod_css(),
        StyleId::Material => material::card_mod_css(),
        StyleId::Flat => flat::card_mod_css(),
        StyleId::Starship => starship::card_mod_css(),
    })
}

fn set(vars: &mut BTreeMap<String, ThemeValue>, key: &str, value: impl Into<String>) {
    vars.insert(key.into(), ThemeValue::Scalar(value.into()));
}

fn rgba(color: Rgb, alpha: f64) -> String {
    let alpha = alpha.clamp(0.0, 1.0);
    format!("rgba({}, {}, {}, {alpha:.2})", color.r, color.g, color.b)
}
