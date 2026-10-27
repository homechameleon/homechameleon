//! Starship Console: an original console layout.
//! Thick color bars, rounded caps, a sweep corner, and small numeric blocks.
//! Names, marks, and type from that television series are not used.

use crate::color::Rgb;
use crate::contrast;
use crate::roles;

pub struct ConsoleBlocks {
    pub orange: Rgb,
    pub peach: Rgb,
    pub lavender: Rgb,
    pub blue: Rgb,
    pub red: Rgb,
    pub ink: Rgb,
}

pub fn card_mod_css() -> &'static str {
    "ha-card{position:relative;overflow:hidden;border:0;box-shadow:none;border-radius:0 var(--console-cap) var(--console-cap) var(--console-cap);background:var(--console-orange) 0 0/42% var(--console-bar) no-repeat,var(--console-peach) 44% 0/14% var(--console-bar) no-repeat,var(--console-lavender) 60% 0/12% var(--console-bar) no-repeat,var(--console-blue) 0 var(--console-bar)/12px calc(100% - var(--console-bar)) no-repeat,var(--primary-background-color);padding:calc(var(--console-bar) + 8px) 12px 12px 22px;text-transform:uppercase}\nha-card::before{content:\"\";position:absolute;left:0;top:0;width:42%;height:var(--console-bar);border-radius:0 999px 999px 0;background:var(--console-orange,var(--primary-color));pointer-events:none}\nha-card::after{content:\"47\";position:absolute;top:0;right:8px;width:40px;height:var(--console-bar);border-radius:0 0 10px 10px;background:var(--console-red);color:var(--console-ink);text-align:center;font:700 12px/var(--console-bar) var(--ha-font-family-heading)}\n"
}

/// Pastel blocks harmonized toward the user's primary, light enough for black ink.
pub fn console_blocks(primary: Rgb) -> ConsoleBlocks {
    ConsoleBlocks {
        orange: block(42.0, 72.0, primary),
        peach: block(28.0, 46.0, primary),
        lavender: block(312.0, 38.0, primary),
        blue: block(232.0, 48.0, primary),
        red: block(18.0, 68.0, primary),
        ink: Rgb::BLACK,
    }
}

pub fn bar_thickness(glow: f64) -> i32 {
    (16.0 + (glow / 100.0).clamp(0.0, 1.0) * 8.0).round() as i32
}

fn block(hue: f64, chroma: f64, primary: Rgb) -> Rgb {
    let seed = roles::state_anchor(hue, chroma, primary);
    for tone in [84, 80, 88, 76, 92, 72, 96] {
        let color = roles::tone_of(seed, tone);
        if contrast::meets(Rgb::BLACK, color, 4.5) {
            return color;
        }
    }
    roles::tone_of(seed, 96)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_are_distinct_and_readable() {
        let paint = console_blocks(Rgb::new(0x7a, 0xa2, 0xf7));
        let colors = [
            paint.orange,
            paint.peach,
            paint.lavender,
            paint.blue,
            paint.red,
        ];
        for (index, color) in colors.iter().enumerate() {
            assert!(
                contrast::contrast_ratio(paint.ink, *color) >= 4.5,
                "{color:?}"
            );
            for other in colors.iter().skip(index + 1) {
                assert_ne!(color.hex(), other.hex());
            }
        }
    }
}
