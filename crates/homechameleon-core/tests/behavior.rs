//! Behavior that has to stay true for every style and both modes.

use saphyr::LoadableYamlNode;

use homechameleon_core::{
    generate, generate_json, slugify, styles, BackgroundOptions, ContrastTarget, ExtractionMode,
    GenerateError, GenerateRequest, ImageInput, Mode, PaletteFormat, PaletteInput, StyleId,
    ThemeOptions, Tweaks, MAX_IMAGES, MAX_IMAGE_BYTES,
};

fn image() -> ImageInput {
    homechameleon_core::gradient(24, 24)
}

fn desk(style: StyleId, mode: Mode) -> GenerateRequest {
    GenerateRequest {
        name: "Desk".into(),
        style,
        mode,
        images: vec![image()],
        ..GenerateRequest::default()
    }
}

#[test]
fn missing_source_and_limits() {
    let err = generate(&GenerateRequest::default()).unwrap_err();
    assert!(matches!(err, GenerateError::MissingSource));
    let mut too_many = desk(StyleId::Sheer, Mode::Dark);
    too_many.images = (0..=MAX_IMAGES).map(|_| image()).collect();
    assert!(matches!(
        generate(&too_many),
        Err(GenerateError::TooManyImages { .. })
    ));
    let mut huge = desk(StyleId::Sheer, Mode::Dark);
    huge.images[0].rgba = vec![0; MAX_IMAGE_BYTES + 1];
    huge.images[0].width = 1;
    huge.images[0].height = 1;
    assert!(matches!(
        generate(&huge),
        Err(GenerateError::ImageTooLarge { .. })
    ));
}

#[test]
fn unknown_style_json() {
    let err = generate_json(r#"{"name":"Desk","style":"glass","images":[]}"#).unwrap_err();
    assert!(matches!(err, GenerateError::UnknownStyle { .. }));
    assert_eq!(err.to_string(), "unknown style 'glass'");
}

#[test]
fn each_style_has_its_own_shape() {
    let mut radii = Vec::new();
    let mut shadows = Vec::new();
    for style in [
        StyleId::Sheer,
        StyleId::Bubbles,
        StyleId::Material,
        StyleId::Flat,
        StyleId::Starship,
    ] {
        let mut request = desk(style, Mode::Dark);
        request.options.card_mod = true;
        let result = generate(&request).unwrap();
        assert!(
            result.contrast.pass,
            "{style:?} {:?}",
            result
                .contrast
                .checks
                .iter()
                .filter(|check| !check.pass)
                .collect::<Vec<_>>()
        );
        radii.push(capture(&result.yaml, "ha-card-border-radius"));
        shadows.push(capture(&result.yaml, "ha-card-box-shadow"));
        match style {
            StyleId::Sheer => {
                assert!(result.yaml.contains("saturate("));
                assert!(result.yaml.contains("@supports"));
                assert!(result.yaml.contains("backdrop-filter"));
            }
            StyleId::Bubbles => {
                assert!(result.yaml.contains("scale(1.015)"));
                assert!(result.yaml.contains("cubic-bezier"));
                assert!(!result.yaml.contains("backdrop-filter"));
            }
            StyleId::Material => {
                assert!(result.yaml.contains("ha-card-border-radius: \"16px\""));
                assert!(result.yaml.contains("opacity:0.08"));
                assert!(!result.yaml.contains("backdrop-filter"));
            }
            StyleId::Flat => {
                assert!(result.yaml.contains("ha-card-box-shadow: \"none\""));
                assert!(result.yaml.contains("letter-spacing:-0.03em"));
                assert!(!result.yaml.contains("backdrop-filter"));
            }
            StyleId::Starship => {
                assert!(result.yaml.contains("console-orange"));
                assert!(result.yaml.contains("var(--primary-color)"));
                assert!(result.yaml.contains("Antonio"));
                assert!(result.yaml.contains("content:\"47\""));
                assert!(result.yaml.contains("border-top-left-radius:52px"));
                assert!(result.yaml.contains("border-bottom-left-radius:40px"));
                assert!(!result.yaml.contains("backdrop-filter"));
            }
        }
    }
    for (index, radius) in radii.iter().enumerate() {
        assert!(
            radii.iter().skip(index + 1).all(|other| other != radius),
            "radii {radii:?}"
        );
        assert!(
            shadows
                .iter()
                .skip(index + 1)
                .all(|other| other != &shadows[index]),
            "shadows {shadows:?}"
        );
    }
}

#[test]
fn styles_are_the_launch_styles() {
    let ids: Vec<_> = styles().into_iter().map(|style| style.id).collect();
    assert_eq!(ids, ["sheer", "bubbles", "material", "flat", "starship"]);
    assert_eq!(slugify("Tokyo Night"), "tokyo-night");
}

#[test]
fn flat_is_solid_with_no_shadow_or_blur() {
    let result = generate(&desk(StyleId::Flat, Mode::Both)).unwrap();
    assert!(result.contrast.pass, "{:?}", result.contrast.checks);
    assert!(result.yaml.contains("ha-card-box-shadow: \"none\""));
    assert!(!result.yaml.contains("backdrop-filter"));
    assert!(result.yaml.contains("ha-card-border-radius: \"8px\""));
    assert!(!result.yaml.contains("card-background-color: \"rgba"));
}

#[test]
fn same_input_same_bytes_and_modes_differ() {
    let request = desk(StyleId::Sheer, Mode::Both);
    let once = generate(&request).unwrap();
    let twice = generate(&request).unwrap();
    assert_eq!(once.yaml, twice.yaml);
    assert_eq!(once.css, twice.css);
    assert!(once.contrast.pass, "{:?}", once.contrast.checks);
    let light = once.light.unwrap().bg;
    let dark = once.dark.unwrap().bg;
    assert_ne!(light, dark);
}

#[test]
fn extraction_modes_are_not_all_the_same() {
    let mut colors = Vec::new();
    for extraction in [
        ExtractionMode::Normal,
        ExtractionMode::Pastel,
        ExtractionMode::Material,
        ExtractionMode::Colorful,
        ExtractionMode::Muted,
    ] {
        let mut request = desk(StyleId::Bubbles, Mode::Dark);
        request.extraction = extraction;
        let result = generate(&request).unwrap();
        assert!(result.contrast.pass);
        colors.push(result.dark.unwrap().primary);
    }
    assert!(colors.iter().any(|color| color != &colors[0]), "{colors:?}");
}

#[test]
fn mood_board_weight_changes_the_primary() {
    let quiet = ImageInput::new(8, 8, solid(8, 8, [20, 40, 90, 255]));
    let loud = ImageInput::new(8, 8, solid(8, 8, [220, 40, 30, 255]));
    let mut left = desk(StyleId::Bubbles, Mode::Dark);
    left.images = vec![
        ImageInput {
            weight: 10.0,
            ..quiet.clone()
        },
        ImageInput {
            weight: 1.0,
            ..loud.clone()
        },
    ];
    let mut right = left.clone();
    right.images[0].weight = 1.0;
    right.images[1].weight = 10.0;
    let a = generate(&left).unwrap().dark.unwrap().primary;
    let b = generate(&right).unwrap().dark.unwrap().primary;
    assert_ne!(a, b, "weights should move the primary");
}

#[test]
fn every_style_passes_contrast_in_both_modes() {
    for style in [
        StyleId::Sheer,
        StyleId::Bubbles,
        StyleId::Material,
        StyleId::Flat,
        StyleId::Starship,
    ] {
        for target in [ContrastTarget::Aa, ContrastTarget::Aaa] {
            let mut request = desk(style, Mode::Both);
            request.options.contrast = target;
            let result = generate(&request).unwrap();
            assert!(
                result.contrast.pass,
                "{style:?} {target:?} {:?}",
                result
                    .contrast
                    .checks
                    .iter()
                    .filter(|c| !c.pass)
                    .collect::<Vec<_>>()
            );
            assert_yaml_keys_match_css(&result.yaml, &result.css);
        }
    }
}

#[test]
fn background_off_does_not_reference_a_wallpaper() {
    let result = generate(&desk(StyleId::Sheer, Mode::Dark)).unwrap();
    assert!(!result.yaml.contains("lovelace-background"));
    assert!(!result.yaml.contains("/local/"));
    assert!(!result.css.contains("lovelace-background"));
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("solid background")));
}

#[test]
fn background_on_uses_the_local_path_and_hash() {
    let mut request = desk(StyleId::Sheer, Mode::Dark);
    request.images[0].use_as_background = true;
    request.background = BackgroundOptions {
        enabled: true,
        blur: 4,
        dim: 0.1,
        ..BackgroundOptions::default()
    };
    let result = generate(&request).unwrap();
    assert!(result.yaml.contains("lovelace-background"));
    assert!(result.yaml.contains("/local/homechameleon/bg-"));
    assert!(result.yaml.contains(".webp"));
    assert!(result.css.contains("--lovelace-background:"));
    assert!(result.warnings.iter().any(|w| w.contains("image encoder")));
    assert!(result.contrast.pass);
}

#[test]
fn card_mod_is_off_unless_asked() {
    let plain = generate(&desk(StyleId::Starship, Mode::Dark)).unwrap();
    assert!(!plain.yaml.contains("card-mod-"));
    let mut request = desk(StyleId::Starship, Mode::Both);
    request.options.card_mod = true;
    let result = generate(&request).unwrap();
    assert!(result.yaml.contains("card-mod-theme: \"desk\""));
    assert!(result.yaml.contains("card-mod-card: |"));
    assert!(result.yaml.contains("var(--primary-color)"));
    for banned in [
        "card-mod-root",
        "card-mod-sidebar",
        "card-mod-more-info",
        "--homechameleon",
    ] {
        assert!(!result.yaml.contains(banned), "{banned}");
        assert!(!result.css.contains(banned), "{banned}");
    }
}

#[test]
fn tweaks_move_roundness_and_saturation() {
    let mut round = desk(StyleId::Bubbles, Mode::Dark);
    round.tweaks = Some(Tweaks {
        roundness: 8.0,
        ..StyleId::Bubbles.default_tweaks()
    });
    let result = generate(&round).unwrap();
    assert!(result.yaml.contains("ha-card-border-radius: \"8px\""));
    let mut sat = desk(StyleId::Bubbles, Mode::Dark);
    sat.tweaks = Some(Tweaks {
        saturation: 80.0,
        ..StyleId::Bubbles.default_tweaks()
    });
    let colored = generate(&sat).unwrap().dark.unwrap().primary;
    let plain = generate(&desk(StyleId::Bubbles, Mode::Dark))
        .unwrap()
        .dark
        .unwrap()
        .primary;
    assert_ne!(colored, plain);
}

#[test]
fn tokyo_night_dark_keeps_accent_when_it_already_contrasts() {
    let toml = include_str!("../../../omarchy/tests/fixtures/tokyo-night.colors.toml");
    let request = GenerateRequest {
        name: "Tokyo Night".into(),
        style: StyleId::Sheer,
        mode: Mode::Dark,
        palette: Some(PaletteInput {
            format: PaletteFormat::OmarchyColorsToml,
            data: toml.into(),
        }),
        ..GenerateRequest::default()
    };
    let result = generate(&request).unwrap();
    assert!(result.contrast.pass, "{:?}", result.contrast.checks);
    let dark = result.dark.unwrap();
    assert_eq!(dark.primary, "#7aa2f7");
    assert_eq!(dark.bg, "#1a1b26");
    assert!(result.yaml.contains("primary-color: \"#7aa2f7\""));
    let palette: serde_json::Value = serde_json::from_str(&result.palette_json).unwrap();
    assert_eq!(palette["schema"], "homechameleon-palette/1");
    assert_eq!(palette["source"], "omarchy");
    assert_eq!(palette["colors"]["selection"], "#292e42");
    assert!(palette.get("background_file").is_none());
}

#[test]
fn yaml_parses_and_css_uses_the_same_keys() {
    let mut request = desk(StyleId::Sheer, Mode::Both);
    request.options = ThemeOptions {
        card_mod: true,
        bubble: true,
        mushroom: true,
        contrast: ContrastTarget::Aa,
    };
    let result = generate(&request).unwrap();
    assert_yaml_keys_match_css(&result.yaml, &result.css);
    let docs = saphyr::Yaml::load_from_str(&result.yaml).expect("saphyr");
    assert!(!docs.is_empty(), "theme yaml should parse");
}

fn solid(width: u32, height: u32, px: [u8; 4]) -> Vec<u8> {
    let mut rgba = Vec::new();
    for _ in 0..width * height {
        rgba.extend_from_slice(&px);
    }
    rgba
}

fn capture(yaml: &str, key: &str) -> String {
    yaml.lines()
        .find_map(|line| {
            let rest = line.trim().strip_prefix(key)?;
            rest.strip_prefix(": \"")
                .and_then(|value| value.strip_suffix('"'))
                .map(str::to_string)
        })
        .unwrap_or_else(|| panic!("missing {key}"))
}

fn assert_yaml_keys_match_css(yaml: &str, css: &str) {
    for line in yaml.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("    ") else {
            continue;
        };
        if rest.starts_with(' ') {
            continue;
        }
        if let Some((key, _)) = rest.split_once(": \"") {
            if key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                let needle = format!("--{key}:");
                assert!(css.contains(&needle), "css missing {needle}");
            }
        }
    }
}
