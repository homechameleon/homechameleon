//! Bubbles: large corners, soft color, and pill controls.

pub fn card_mod_css() -> &'static str {
    "ha-card {\n  overflow: hidden;\n}\n"
}

pub fn shadow(glow: f64, dark: bool) -> String {
    let t = glow / 100.0;
    let alpha = if dark {
        0.22 + t * 0.28
    } else {
        0.08 + t * 0.14
    };
    let blur = 10.0 + t * 22.0;
    let y = 6.0 + t * 8.0;
    format!("0 {y:.0}px {blur:.0}px rgba(0, 0, 0, {alpha:.2})")
}
