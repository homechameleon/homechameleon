//! sRGB and OKLab helpers. Conversions use the `libm` crate so native and wasm agree.

use libm::{cbrt, hypot, pow};

/// 8-bit sRGB color. The alpha channel is not stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
    };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// `#rrggbb`, lowercase.
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// `r, g, b` for Home Assistant `rgb-*` variables.
    pub fn triplet(self) -> String {
        format!("{}, {}, {}", self.r, self.g, self.b)
    }

    /// Accepts `#rgb`, `#rrggbb`, and the same forms without `#`.
    /// An 8-digit value keeps the first six digits and drops the alpha byte.
    pub fn parse(input: &str) -> Option<Self> {
        let text = input.trim().trim_matches('"').trim_matches('\'');
        let text = text.strip_prefix('#').unwrap_or(text);
        if !text.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        match text.len() {
            3 => {
                let expand = |i: usize| -> Option<u8> {
                    let c = text.as_bytes().get(i).copied()?;
                    let n = hex_val(c)?;
                    Some(n * 16 + n)
                };
                Some(Self::new(expand(0)?, expand(1)?, expand(2)?))
            }
            6 | 8 => {
                let r = u8::from_str_radix(&text[0..2], 16).ok()?;
                let g = u8::from_str_radix(&text[2..4], 16).ok()?;
                let b = u8::from_str_radix(&text[4..6], 16).ok()?;
                Some(Self::new(r, g, b))
            }
            _ => None,
        }
    }
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// OKLab, Björn Ottosson's space. `l` is about 0–1.
#[derive(Clone, Copy, Debug)]
pub struct Oklab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

impl Oklab {
    pub fn chroma(self) -> f64 {
        hypot(self.a, self.b)
    }

    pub fn distance(self, other: Self) -> f64 {
        let dl = self.l - other.l;
        let da = self.a - other.a;
        let db = self.b - other.b;
        hypot(hypot(dl, da), db)
    }
}

pub fn oklab_from_rgb(rgb: Rgb) -> Oklab {
    let r = srgb_to_linear(f64::from(rgb.r) / 255.0);
    let g = srgb_to_linear(f64::from(rgb.g) / 255.0);
    let b = srgb_to_linear(f64::from(rgb.b) / 255.0);
    let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
    let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
    let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
    let l_ = cbrt(l);
    let m_ = cbrt(m);
    let s_ = cbrt(s);
    Oklab {
        l: 0.210_454_255_3 * l_ + 0.793_617_785_0 * m_ - 0.004_072_046_8 * s_,
        a: 1.977_998_495_1 * l_ - 2.428_592_205_0 * m_ + 0.450_593_709_9 * s_,
        b: 0.025_904_037_1 * l_ + 0.782_771_766_2 * m_ - 0.808_675_766_0 * s_,
    }
}

pub fn rgb_from_oklab(lab: Oklab) -> Rgb {
    let l_ = lab.l + 0.396_337_777_4 * lab.a + 0.215_803_757_3 * lab.b;
    let m_ = lab.l - 0.105_561_345_8 * lab.a - 0.063_854_172_8 * lab.b;
    let s_ = lab.l - 0.089_484_177_5 * lab.a - 1.291_485_548_0 * lab.b;
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;
    let r = 4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s;
    let g = -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s;
    let b = -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s;
    Rgb::new(encode_channel(r), encode_channel(g), encode_channel(b))
}

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        pow((c + 0.055) / 1.055, 2.4)
    }
}

fn encode_channel(linear: f64) -> u8 {
    let c = linear.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * pow(c, 1.0 / 2.4) - 0.055
    };
    (s * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Scale OKLab chroma and nudge the blue-yellow axis.
/// A zero saturation and temperature returns `rgb` unchanged, so the default
/// path does not round-trip through OKLab.
pub fn apply_saturation_temperature(rgb: Rgb, saturation: f64, temperature: f64) -> Rgb {
    if saturation == 0.0 && temperature == 0.0 {
        return rgb;
    }
    let mut lab = oklab_from_rgb(rgb);
    let scale = (1.0 + saturation / 100.0).clamp(0.0, 2.0);
    lab.a *= scale;
    lab.b *= scale;
    lab.b += temperature.clamp(-50.0, 50.0) * 0.003;
    rgb_from_oklab(lab)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        for c in [
            Rgb::new(0, 0, 0),
            Rgb::new(255, 255, 255),
            Rgb::new(122, 16, 200),
        ] {
            assert_eq!(Rgb::parse(&c.hex()), Some(c));
        }
        assert_eq!(Rgb::parse("#abc"), Some(Rgb::new(0xaa, 0xbb, 0xcc)));
        assert_eq!(Rgb::parse("7AA2F7"), Some(Rgb::new(0x7a, 0xa2, 0xf7)));
        assert_eq!(
            Rgb::parse("#7aa2f7ff").map(|c| c.hex()),
            Some("#7aa2f7".into())
        );
        assert!(Rgb::parse("nope").is_none());
    }

    #[test]
    fn oklab_roundtrip_is_stable() {
        for c in [
            Rgb::new(10, 20, 40),
            Rgb::new(200, 80, 40),
            Rgb::new(240, 240, 245),
        ] {
            let back = rgb_from_oklab(oklab_from_rgb(c));
            assert!((i16::from(back.r) - i16::from(c.r)).abs() <= 1);
            assert!((i16::from(back.g) - i16::from(c.g)).abs() <= 1);
            assert!((i16::from(back.b) - i16::from(c.b)).abs() <= 1);
        }
    }
}
