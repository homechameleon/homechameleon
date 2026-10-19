//! Bridge to `material-colors`. The crate is built with `libm` and without
//! `std`, so its HCT math matches between native tests and wasm.

use crate::color::Rgb;
use material_colors::color::Rgb as McRgb;

pub fn to_mc(rgb: Rgb) -> McRgb {
    McRgb::new(rgb.r, rgb.g, rgb.b)
}

pub fn from_mc(rgb: McRgb) -> Rgb {
    Rgb::new(rgb.red, rgb.green, rgb.blue)
}
