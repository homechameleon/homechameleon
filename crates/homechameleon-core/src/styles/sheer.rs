//! Sheer: translucent cards, a backdrop blur, and a bright rim.

pub fn card_mod_css() -> &'static str {
    "ha-card {\n  position: relative;\n  overflow: hidden;\n}\nha-card::after {\n  content: \"\";\n  position: absolute;\n  inset: 0;\n  pointer-events: none;\n  border-radius: inherit;\n  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.35);\n}\n"
}

pub fn shadow(glow: f64, dark: bool) -> String {
    let t = glow / 100.0;
    let drop_a = if dark {
        0.28 + t * 0.30
    } else {
        0.10 + t * 0.16
    };
    let rim = 0.18 + t * 0.42;
    let sheen = 0.04 + t * 0.16;
    let blur = 8.0 + t * 28.0;
    let spread = 4.0 + t * 18.0;
    format!(
        "0 {spread:.0}px {blur:.0}px rgba(0, 0, 0, {drop_a:.2}), inset 0 1px 0 rgba(255, 255, 255, {rim:.2}), inset 0 -1px 0 rgba(255, 255, 255, 0.06), inset 0 0 {glow:.0}px rgba(255, 255, 255, {sheen:.2})"
    )
}

pub fn border(glow: f64) -> String {
    let alpha = 0.22 + (glow / 100.0) * 0.38;
    format!("rgba(255, 255, 255, {alpha:.2})")
}
