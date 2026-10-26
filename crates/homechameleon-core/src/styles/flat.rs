//! Flat: solid surfaces, no shadow, and no blur.

pub fn card_mod_css() -> &'static str {
    "ha-card {\n  overflow: hidden;\n}\n"
}

pub fn shadow() -> String {
    "none".into()
}
