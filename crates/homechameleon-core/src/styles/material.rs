//! Material: tonal surfaces, elevation, and no blur.

pub fn card_mod_css() -> &'static str {
    "ha-card {\n  overflow: hidden;\n}\n"
}

pub fn shadow(glow: f64, dark: bool) -> String {
    let t = glow / 100.0;
    let a1 = if dark {
        0.24 + t * 0.16
    } else {
        0.08 + t * 0.08
    };
    let a2 = if dark {
        0.18 + t * 0.12
    } else {
        0.06 + t * 0.06
    };
    format!("0 1px 2px rgba(0, 0, 0, {a1:.2}), 0 2px 6px rgba(0, 0, 0, {a2:.2})")
}
