//! Material: tonal surfaces, M3 elevation, and a 12–28px shape scale.

pub fn card_mod_css() -> &'static str {
    "ha-card{position:relative;overflow:hidden}\nha-card::after{content:\"\";position:absolute;inset:0;pointer-events:none;background:var(--primary-color);opacity:0}\nha-card:hover::after{opacity:0.08}\nha-card:active::after{opacity:0.12}\nha-card>*{position:relative;z-index:1}\n"
}

/// Level 1 at glow 0, about level 3 at glow 100. Dark mode keeps the same
/// two-layer shape with a heavier ambient term.
pub fn shadow(glow: f64, dark: bool) -> String {
    let t = (glow / 100.0).clamp(0.0, 1.0);
    let y = 1.0 + t * 3.0;
    let blur = 3.0 + t * 5.0;
    let spread = 1.0 + t * 2.0;
    if dark {
        let ambient = 0.28 + t * 0.16;
        let key = 0.40 + t * 0.18;
        format!(
            "0 1px 2px rgba(0,0,0,{key:.2}),0 {y:.0}px {blur:.0}px {spread:.0}px rgba(0,0,0,{ambient:.2})"
        )
    } else {
        format!("0 1px 2px rgba(0,0,0,.3),0 {y:.0}px {blur:.0}px {spread:.0}px rgba(0,0,0,.15)")
    }
}

pub fn card_radius(roundness: f64) -> i32 {
    roundness.clamp(12.0, 28.0).round() as i32
}
