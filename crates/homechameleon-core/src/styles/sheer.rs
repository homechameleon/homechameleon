//! Sheer: frosted panels, a saturated backdrop blur, a light rim, and a sheen.

pub fn card_mod_css() -> &'static str {
    "ha-card{position:relative;overflow:hidden;-webkit-backdrop-filter:var(--ha-card-backdrop-filter,blur(18px) saturate(160%));backdrop-filter:var(--ha-card-backdrop-filter,blur(18px) saturate(160%))}\nha-card::before{content:\"\";position:absolute;inset:0;pointer-events:none;border-radius:inherit;background:linear-gradient(165deg,rgba(255,255,255,.34),transparent 56%);box-shadow:inset 0 1px 0 rgba(255,255,255,.58)}\nha-card>*{position:relative;z-index:1}\n@supports not ((backdrop-filter:blur(1px)) or (-webkit-backdrop-filter:blur(1px))){ha-card{background:var(--secondary-background-color);backdrop-filter:none;-webkit-backdrop-filter:none}}\n"
}

pub fn backdrop(blur: f64) -> String {
    let px = blur.round().clamp(0.0, 40.0) as i32;
    let saturate = 145 + (blur / 40.0 * 40.0).round() as i32;
    format!("blur({px}px) saturate({saturate}%)")
}

pub fn shadow(glow: f64, dark: bool) -> String {
    let t = glow / 100.0;
    let drop_a = if dark {
        0.32 + t * 0.28
    } else {
        0.12 + t * 0.16
    };
    let rim = 0.28 + t * 0.40;
    let y = 18.0 + t * 16.0;
    let blur = 40.0 + t * 36.0;
    format!(
        "0 {y:.0}px {blur:.0}px rgba(0,0,0,{drop_a:.2}),inset 0 1px 0 rgba(255,255,255,{rim:.2}),inset 0 -1px 0 rgba(255,255,255,.08)"
    )
}

pub fn border(glow: f64) -> String {
    let alpha = 0.28 + (glow / 100.0) * 0.40;
    format!("rgba(255, 255, 255, {alpha:.2})")
}
