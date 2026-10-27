//! Flat: no shadow, sharp color blocks, and a strong type hierarchy.

pub fn card_mod_css() -> &'static str {
    "ha-card{overflow:hidden;box-shadow:none;border:0;border-radius:var(--ha-card-border-radius,8px);background:var(--secondary-background-color)}\nha-card .card-header .name,ha-card h1{font-weight:800;letter-spacing:-0.03em}\nha-card hui-generic-entity-row,ha-card .entity{border-bottom:1px solid var(--divider-color);padding:10px 0}\n"
}

pub fn shadow() -> String {
    "none".into()
}
