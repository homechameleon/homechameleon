//! v0.1 QA matrix. Reads Omarchy `colors.toml` files from `QA_THEMES` and
//! generates twenty synthetic photos. Prints one YAML document per case.
//! Native and wasm must print the same bytes.

use std::path::Path;

use homechameleon_core::{
    generate, ContrastTarget, ExtractionMode, GenerateRequest, ImageInput, Mode, PaletteFormat,
    PaletteInput, StyleId,
};

const STYLES: [&str; 4] = ["sheer", "bubbles", "material", "starship"];
const MODES: [(&str, Mode); 2] = [("light", Mode::Light), ("dark", Mode::Dark)];
const PHOTO_EDGE: u32 = 64;

fn main() {
    let dir = std::env::var("QA_THEMES").unwrap_or_else(|_| {
        eprintln!("QA_THEMES must point at an Omarchy themes directory");
        std::process::exit(2);
    });
    let themes = load_themes(Path::new(&dir));
    if themes.is_empty() {
        eprintln!("no colors.toml files under {dir}");
        std::process::exit(2);
    }
    let photos = photos();
    let mut failed = 0u32;
    let mut cases = 0u32;
    for (name, data) in &themes {
        for style in STYLES {
            for (mode_id, mode) in MODES {
                cases += 1;
                let mut request = base(name, style, mode);
                request.palette = Some(PaletteInput {
                    format: PaletteFormat::OmarchyColorsToml,
                    data: data.clone(),
                });
                failed += run_case("theme", name, style, mode_id, &request);
            }
        }
    }
    for (name, image) in &photos {
        for style in STYLES {
            for (mode_id, mode) in MODES {
                cases += 1;
                let mut request = base(name, style, mode);
                request.images = vec![image.clone()];
                failed += run_case("photo", name, style, mode_id, &request);
            }
        }
    }
    eprintln!(
        "cases={cases} themes={} photos={} contrast_fail={failed}",
        themes.len(),
        photos.len()
    );
    if failed > 0 {
        std::process::exit(1);
    }
}

fn base(name: &str, style: &str, mode: Mode) -> GenerateRequest {
    GenerateRequest {
        name: name.to_string(),
        style: StyleId::parse(style).expect("style"),
        mode,
        extraction: ExtractionMode::Normal,
        options: homechameleon_core::ThemeOptions {
            contrast: ContrastTarget::Aa,
            ..homechameleon_core::ThemeOptions::default()
        },
        ..GenerateRequest::default()
    }
}

fn run_case(kind: &str, name: &str, style: &str, mode: &str, request: &GenerateRequest) -> u32 {
    let result = match generate(request) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("FAIL {kind} {name} {style} {mode} generate: {err}");
            return 1;
        }
    };
    if result.theme != name {
        eprintln!(
            "FAIL {kind} {name} {style} {mode} theme field {:?} != {name:?}",
            result.theme
        );
        return 1;
    }
    let keys = top_level_keys(&result.yaml);
    if keys != [name] {
        eprintln!("FAIL {kind} {name} {style} {mode} top-level keys {keys:?}");
        return 1;
    }
    println!("===== {kind} {name} {style} {mode} =====");
    print!("{}", result.yaml);
    if !result.yaml.ends_with('\n') {
        println!();
    }
    if result.contrast.pass {
        return 0;
    }
    for check in &result.contrast.checks {
        if !check.pass {
            eprintln!(
                "FAIL {kind} {name} {style} {mode} {} {} ratio={:.3} minimum={:.1}",
                check.mode, check.pair, check.ratio, check.minimum
            );
        }
    }
    1
}

fn top_level_keys(yaml: &str) -> Vec<String> {
    yaml.lines()
        .filter_map(|line| {
            if line.is_empty() || line.starts_with('#') || line.starts_with(' ') {
                return None;
            }
            let key = line.strip_suffix(':')?;
            let key = key.trim();
            let key = key
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
                .unwrap_or(key);
            Some(key.to_string())
        })
        .collect()
}

fn load_themes(dir: &Path) -> Vec<(String, String)> {
    let mut themes = Vec::new();
    let entries = std::fs::read_dir(dir).unwrap_or_else(|err| {
        eprintln!("cannot read {}: {err}", dir.display());
        std::process::exit(2);
    });
    for entry in entries {
        let entry = entry.unwrap_or_else(|err| {
            eprintln!("cannot read a theme entry: {err}");
            std::process::exit(2);
        });
        let colors = entry.path().join("colors.toml");
        if !colors.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let data = std::fs::read_to_string(&colors).unwrap_or_else(|err| {
            eprintln!("cannot read {}: {err}", colors.display());
            std::process::exit(2);
        });
        themes.push((name, data));
    }
    themes.sort_by(|a, b| a.0.cmp(&b.0));
    themes
}

fn photos() -> Vec<(String, ImageInput)> {
    (1..=20)
        .map(|index| {
            let name = format!("QA Photo {index:02}");
            (name, paint(index, PHOTO_EDGE, PHOTO_EDGE))
        })
        .collect()
}

fn paint(index: u32, width: u32, height: u32) -> ImageInput {
    let mut rgba = vec![0u8; (width as usize) * (height as usize) * 4];
    for y in 0..height {
        for x in 0..width {
            let (r, g, b) = pixel(index, x, y, width, height);
            let at = ((y * width + x) * 4) as usize;
            rgba[at] = r;
            rgba[at + 1] = g;
            rgba[at + 2] = b;
            rgba[at + 3] = 255;
        }
    }
    ImageInput::new(width, height, rgba)
}

fn pixel(index: u32, x: u32, y: u32, width: u32, height: u32) -> (u8, u8, u8) {
    let xt = x * 255 / (width - 1);
    let yt = y * 255 / (height - 1);
    let mix = |a: u32, b: u32, t: u32| -> u8 {
        let a = a as i32;
        let b = b as i32;
        (a + (b - a) * t as i32 / 255) as u8
    };
    match index {
        1 => (mix(255, 180, xt), mix(120, 40, yt), mix(40, 20, xt)),
        2 => (mix(10, 40, yt), mix(80, 160, xt), mix(140, 220, yt)),
        3 => {
            if (x / 8 + y / 8).is_multiple_of(2) {
                (12, 12, 12)
            } else {
                (245, 245, 245)
            }
        }
        4 => (mix(255, 210, xt), mix(220, 190, yt), mix(230, 255, xt)),
        5 => {
            let v = mix(30, 200, xt);
            (v, v, v)
        }
        6 => (mix(180, 255, xt), mix(20, 60, yt), mix(30, 70, xt)),
        7 => (mix(30, 80, yt), mix(90, 170, xt), mix(40, 70, yt)),
        8 => (mix(8, 24, yt), mix(16, 40, xt), mix(48, 110, yt)),
        9 => (mix(210, 160, yt), mix(150, 110, xt), mix(120, 90, yt)),
        10 => (mix(255, 20, xt), mix(0, 255, yt), mix(40, 200, xt)),
        11 => {
            let v = mix(140, 170, xt / 2 + yt / 2);
            (v, mix(u32::from(v), 150, 51), mix(u32::from(v), 145, 76))
        }
        12 => {
            if y < height / 2 {
                (20, 90, 200)
            } else {
                (230, 120, 20)
            }
        }
        13 => (mix(250, 255, xt), mix(248, 252, yt), mix(245, 250, xt)),
        14 => (mix(0, 18, xt), mix(0, 12, yt), mix(0, 16, xt)),
        15 => {
            if x % 10 < 5 {
                (200, 40, 60)
            } else {
                (30, 30, 40)
            }
        }
        16 => {
            let n = x
                .wrapping_mul(374761393)
                .wrapping_add(y.wrapping_mul(668265263));
            ((n >> 16) as u8, (n >> 8) as u8, n as u8)
        }
        17 => {
            if x * height + y * width < width * height {
                (40, 180, 170)
            } else {
                (240, 200, 60)
            }
        }
        18 => (mix(120, 80, yt), mix(90, 60, xt), mix(50, 30, yt)),
        19 => (mix(70, 30, yt), mix(40, 20, xt), mix(120, 80, yt)),
        _ => {
            let band = (x * 3 / width) + (y * 3 / height) * 3;
            match band % 6 {
                0 => (220, 50, 50),
                1 => (240, 180, 40),
                2 => (40, 160, 70),
                3 => (40, 90, 200),
                4 => (150, 60, 180),
                _ => (240, 240, 245),
            }
        }
    }
}
