//! Fixed inputs for golden files and the native/wasm parity check.
//! Every byte comes from code or from the Omarchy fixture in this repository.

use crate::types::{
    BackgroundOptions, ExtractionMode, GenerateRequest, ImageInput, Mode, PaletteFormat,
    PaletteInput, StyleId, ThemeOptions,
};
const TOKYO: &str = include_str!("../../../omarchy/tests/fixtures/tokyo-night.colors.toml");

pub fn fixture_report() -> String {
    let mut out = String::new();
    for (title, request) in cases() {
        let result = crate::generate(&request).unwrap_or_else(|err| panic!("{title}: {err}"));
        out.push_str("===== ");
        out.push_str(title);
        out.push_str(" =====\n");
        out.push_str(&result.yaml);
        out.push_str("--- css ---\n");
        out.push_str(&result.css);
        out.push_str("--- warnings ---\n");
        for warning in &result.warnings {
            out.push_str(warning);
            out.push('\n');
        }
        out.push_str(&format!(
            "--- contrast {} ---\n",
            if result.contrast.pass { "pass" } else { "fail" }
        ));
    }
    out
}

fn cases() -> Vec<(&'static str, GenerateRequest)> {
    let image = gradient(32, 32);
    vec![
        (
            "sheer-both",
            GenerateRequest {
                name: "Desk".into(),
                style: StyleId::Sheer,
                mode: Mode::Both,
                images: vec![image.clone()],
                ..GenerateRequest::default()
            },
        ),
        (
            "bubbles-light",
            GenerateRequest {
                name: "Desk".into(),
                style: StyleId::Bubbles,
                mode: Mode::Light,
                images: vec![image.clone()],
                ..GenerateRequest::default()
            },
        ),
        (
            "material-dark",
            GenerateRequest {
                name: "Desk".into(),
                style: StyleId::Material,
                mode: Mode::Dark,
                extraction: ExtractionMode::Material,
                images: vec![image.clone()],
                ..GenerateRequest::default()
            },
        ),
        (
            "starship-card-mod",
            GenerateRequest {
                name: "Desk".into(),
                style: StyleId::Starship,
                mode: Mode::Both,
                images: vec![image.clone()],
                options: ThemeOptions {
                    card_mod: true,
                    ..ThemeOptions::default()
                },
                ..GenerateRequest::default()
            },
        ),
        (
            "tokyo-sheer-dark",
            GenerateRequest {
                name: "Tokyo Night".into(),
                style: StyleId::Sheer,
                mode: Mode::Dark,
                palette: Some(PaletteInput {
                    format: PaletteFormat::OmarchyColorsToml,
                    data: TOKYO.into(),
                }),
                ..GenerateRequest::default()
            },
        ),
        (
            "sheer-background",
            GenerateRequest {
                name: "Desk".into(),
                style: StyleId::Sheer,
                mode: Mode::Dark,
                images: vec![ImageInput {
                    use_as_background: true,
                    ..image
                }],
                background: BackgroundOptions {
                    enabled: true,
                    blur: 8,
                    dim: 0.2,
                    ..BackgroundOptions::default()
                },
                ..GenerateRequest::default()
            },
        ),
    ]
}

pub fn gradient(width: u32, height: u32) -> ImageInput {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let r = ((x * 7 + 20) % 256) as u8;
            let g = ((y * 5 + 30) % 256) as u8;
            let b = (70 + ((x + y) % 50)) as u8;
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    ImageInput::new(width, height, rgba)
}
