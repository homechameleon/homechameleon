//! Bubbles: blob cards, puffy inner and outer shadows, and bouncy controls.

pub fn card_mod_css() -> &'static str {
    "ha-card{overflow:hidden;border:0;border-radius:var(--ha-card-border-radius) 48px 40px 46px;transition:transform 180ms cubic-bezier(.34,1.56,.64,1)}\nha-card:hover{transform:translateY(-4px) scale(1.015)}\nha-card:active{transform:translateY(2px) scale(.98)}\nha-card ha-icon,ha-card .button,ha-card ha-button{border-radius:999px}\n"
}

pub fn shadow(glow: f64, dark: bool) -> String {
    let t = glow / 100.0;
    if dark {
        let drop = 0.38 + t * 0.28;
        let lift = 0.06 + t * 0.06;
        let y = 8.0 + t * 8.0;
        let blur = 16.0 + t * 16.0;
        format!(
            "{y:.0}px {y:.0}px {blur:.0}px rgba(0,0,0,{drop:.2}),-{y:.0}px -{y:.0}px {blur:.0}px rgba(255,255,255,{lift:.2}),inset 3px 3px 8px rgba(255,255,255,.06),inset -4px -6px 12px rgba(0,0,0,.45)"
        )
    } else {
        let drop = 0.12 + t * 0.10;
        let y = 8.0 + t * 6.0;
        let blur = 16.0 + t * 14.0;
        format!(
            "{y:.0}px {y:.0}px {blur:.0}px rgba(120,90,140,{drop:.2}),-{y:.0}px -{y:.0}px {blur:.0}px rgba(255,255,255,.85),inset 3px 3px 8px rgba(255,255,255,.95),inset -4px -6px 10px rgba(90,70,120,.08)"
        )
    }
}
