//! Palette extraction and Home Assistant theme generation.
//!
//! `generate` takes raw RGBA images or a palette and returns one theme YAML
//! file plus a preview CSS map that uses the same variables. Output is
//! deterministic: the same input produces the same bytes on native and wasm.
//!
//! HomeChameleon is not affiliated with or endorsed by the Open Home Foundation
//! or Home Assistant, and is not affiliated with ha-chameleon.

#![forbid(unsafe_code)]

mod b64;
mod codec;
mod color;
mod contrast;
mod extract;
mod fixture;
mod generate;
mod json;
mod kmeans;
mod mc;
mod palette;
mod roles;
mod styles;
mod types;
mod yaml;

#[cfg(feature = "wasm")]
mod wasm;

pub use b64::decode as decode_base64;
pub use color::Rgb;
pub use fixture::{fixture_report, gradient};
pub use generate::generate;
pub use json::{parse as parse_json, Json};
pub use palette::{PaletteDocument, SCHEMA};
pub use types::{
    slugify, styles, BackgroundOptions, ContrastReport, ContrastTarget, ExtractionMode,
    GenerateError, GenerateRequest, GenerateResult, ImageInput, Mode, PaletteFormat, PaletteInput,
    RoleColors, StyleId, StyleInfo, ThemeOptions, Tweaks, MAX_IMAGES, MAX_IMAGE_BYTES,
};

/// JSON in, JSON out. Image `rgba` fields are standard base64.
/// An unknown style returns [`GenerateError::UnknownStyle`].
pub fn generate_json(request: &str) -> Result<String, GenerateError> {
    let value = json::parse(request).map_err(|err| GenerateError::BadPalette {
        reason: format!("request JSON could not be read: {err}"),
    })?;
    if let Some(style) = value.get("style").and_then(json::Json::as_str) {
        StyleId::parse(style)?;
    }
    let request = codec::request_from_json(&value)?;
    let result = generate(&request)?;
    Ok(codec::result_to_json(&result))
}
