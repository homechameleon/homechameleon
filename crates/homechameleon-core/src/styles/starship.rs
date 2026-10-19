//! Starship Console: dark panels, pill controls, and a colored cap on each card.

pub fn card_mod_css() -> &'static str {
    "ha-card {\n  position: relative;\n  overflow: hidden;\n}\nha-card::before {\n  content: \"\";\n  position: absolute;\n  top: 0;\n  left: 0;\n  right: 0;\n  height: 3px;\n  background: var(--primary-color);\n}\n"
}

pub fn shadow(glow: f64, primary_hex: &str) -> String {
    let t = glow / 100.0;
    let inset = 4.0 + t * 6.0;
    let drop = 0.35 + t * 0.25;
    format!("inset {inset:.0}px 0 0 {primary_hex}, 0 8px 24px rgba(0, 0, 0, {drop:.2})")
}
