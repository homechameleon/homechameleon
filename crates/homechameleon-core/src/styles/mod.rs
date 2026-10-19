//! Four launch styles. Each one writes the same variable names the preview CSS uses.

mod bubbles;
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
    paint_colors(&mut vars, roles, alpha);
    paint_shape(&mut vars, style, roles, mode, tweaks, alpha);
    paint_named(&mut vars, roles);
    paint_state(&mut vars, roles);
    paint_type(&mut vars);
    if options.bubble {
        paint_bubble(&mut vars, roles, tweaks);
    }
    if options.mushroom {
        paint_mushroom(&mut vars, roles, tweaks);
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

fn paint_colors(vars: &mut BTreeMap<String, ThemeValue>, roles: &Roles, alpha: f64) {
    let card = if alpha >= 0.999 {
        roles.surface.hex()
    } else {
        rgba(roles.surface, alpha)
    };
    set(vars, "primary-color", roles.primary.hex());
    set(vars, "accent-color", roles.accent.hex());
    set(vars, "primary-background-color", roles.bg.hex());
    set(vars, "secondary-background-color", roles.surface2.hex());
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
    set(vars, "divider-color", rgba(roles.text, 0.12));
    set(vars, "outline-color", roles.outline.hex());
    set(vars, "app-header-background-color", roles.sidebar.hex());
    set(vars, "app-header-text-color", roles.text.hex());
    set(vars, "sidebar-background-color", roles.sidebar.hex());
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
    set(vars, "rgb-card-background-color", roles.surface.triplet());
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
    roles: &Roles,
    mode: ColorMode,
    tweaks: Tweaks,
    alpha: f64,
) {
    let radius = format!("{}px", tweaks.roundness.round() as i32);
    let control = match style {
        StyleId::Bubbles | StyleId::Starship => "999px".into(),
        StyleId::Material => format!("{}px", (tweaks.roundness * 0.5).round() as i32),
        StyleId::Sheer => format!("{}px", (tweaks.roundness * 0.6).round() as i32),
    };
    set(vars, "ha-card-border-radius", radius.clone());
    set(vars, "ha-border-radius-sm", "4px");
    set(vars, "ha-border-radius-md", control.clone());
    set(vars, "ha-border-radius-lg", radius.clone());
    set(vars, "ha-border-radius-xl", radius);
    set(vars, "ha-border-radius-pill", "999px");
    let (shadow, border, border_width, blur, dialog): (
        String,
        String,
        String,
        Option<f64>,
        Option<String>,
    ) = match style {
        StyleId::Sheer => (
            sheer::shadow(tweaks.glow, mode.is_dark()),
            sheer::border(tweaks.glow),
            "1px".into(),
            Some(tweaks.blur),
            Some(format!("blur({}px)", tweaks.blur.round() as i32)),
        ),
        StyleId::Bubbles => (
            bubbles::shadow(tweaks.glow, mode.is_dark()),
            rgba(roles.outline, 0.28),
            "1px".into(),
            Some(tweaks.blur.min(12.0)),
            None,
        ),
        StyleId::Material => (
            material::shadow(tweaks.glow, mode.is_dark()),
            roles.outline.hex(),
            "0px".into(),
            None,
            None,
        ),
        StyleId::Starship => (
            starship::shadow(tweaks.glow, &roles.primary.hex()),
            roles.primary.hex(),
            "1px".into(),
            None,
            None,
        ),
    };
    set(vars, "ha-card-box-shadow", shadow);
    set(vars, "ha-card-border-color", border);
    set(vars, "ha-card-border-width", border_width);
    if let Some(blur) = blur {
        if blur > 0.0 && alpha < 0.999 {
            set(
                vars,
                "ha-card-backdrop-filter",
                format!("blur({}px)", blur.round() as i32),
            );
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

fn paint_type(vars: &mut BTreeMap<String, ThemeValue>) {
    set(vars, "ha-font-family-body", "Roboto, Noto, sans-serif");
    set(vars, "ha-font-family-heading", "Roboto, Noto, sans-serif");
    set(
        vars,
        "ha-font-family-code",
        "Roboto Mono, ui-monospace, monospace",
    );
    set(vars, "ha-font-size-s", "12px");
    set(vars, "ha-font-size-m", "14px");
    set(vars, "ha-font-size-l", "16px");
    set(vars, "ha-font-size-xl", "20px");
    set(vars, "ha-font-size-2xl", "24px");
    set(vars, "ha-font-weight-normal", "400");
    set(vars, "ha-font-weight-medium", "500");
    set(vars, "ha-font-weight-bold", "700");
    set(vars, "ha-line-height-normal", "1.5");
}

fn paint_bubble(vars: &mut BTreeMap<String, ThemeValue>, roles: &Roles, tweaks: Tweaks) {
    let radius = format!("{}px", tweaks.roundness.round() as i32);
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
    set(vars, "bubble-button-border-radius", "999px");
    set(vars, "bubble-icon-border-radius", "999px");
    set(vars, "bubble-select-arrow", "0px");
}

fn paint_mushroom(vars: &mut BTreeMap<String, ThemeValue>, roles: &Roles, tweaks: Tweaks) {
    let radius = match tweaks.roundness {
        r if r >= 24.0 => "999px".into(),
        r => format!("{}px", (r * 0.5).round() as i32),
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
    set(vars, "mush-spacing", "8px");
}

fn card_mod_css(style: StyleId) -> Option<&'static str> {
    Some(match style {
        StyleId::Sheer => sheer::card_mod_css(),
        StyleId::Bubbles => bubbles::card_mod_css(),
        StyleId::Material => material::card_mod_css(),
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
