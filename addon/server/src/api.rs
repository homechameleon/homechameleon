//! HTTP API for the ingress editor. Generation goes through homechameleon-core.

use std::path::Path;

use homechameleon_core::{
    decode_base64, generate_json, parse_json, slugify, styles, GenerateError, Json,
};

use crate::store;
use crate::supervisor::{self, post_service};
use crate::Settings;

const MAX_YAML_BYTES: usize = 1_000_000;
pub const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

pub struct Reply {
    pub status: u16,
    pub content_type: &'static str,
    pub bytes: Vec<u8>,
}

impl Reply {
    pub fn json(status: u16, value: &Json) -> Self {
        Self {
            status,
            content_type: "application/json; charset=utf-8",
            bytes: value.stringify().into_bytes(),
        }
    }

    pub fn text(status: u16, content_type: &'static str, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            content_type,
            bytes: body.into(),
        }
    }

    pub fn error(status: u16, message: &str) -> Self {
        Self::json(
            status,
            &Json::Object(vec![("error".into(), Json::from(message))]),
        )
    }
}

pub fn route(method: &str, path: &str, body: &str, settings: &Settings) -> Reply {
    let path = if path.len() > 1 {
        path.trim_end_matches('/')
    } else {
        path
    };
    match (method, path) {
        ("GET", "/api/styles") => styles_reply(),
        ("POST", "/api/apply") => apply(body, settings),
        ("POST", "/api/generate") => generate(body, settings, true),
        ("POST", "/api/preview") => generate(body, settings, false),
        ("DELETE", path) if path.starts_with("/api/theme/") => {
            delete(path.trim_start_matches("/api/theme/"), settings)
        }
        ("GET", "/health") => Reply::text(200, "text/plain; charset=utf-8", "ok\n"),
        _ if path.starts_with("/api/") => Reply::error(404, "not found"),
        _ => Reply::error(404, "not found"),
    }
}

fn styles_reply() -> Reply {
    let items = styles()
        .into_iter()
        .map(|style| {
            Json::Object(vec![
                ("id".into(), Json::from(style.id)),
                ("name".into(), Json::from(style.name)),
                ("summary".into(), Json::from(style.summary)),
            ])
        })
        .collect();
    Reply::json(
        200,
        &Json::Object(vec![("styles".into(), Json::Array(items))]),
    )
}

fn apply(body: &str, settings: &Settings) -> Reply {
    let value = match object(body) {
        Ok(value) => value,
        Err(reply) => return reply,
    };
    let yaml = match value.get("yaml").and_then(Json::as_str) {
        Some(yaml) if !yaml.trim().is_empty() => yaml,
        _ => return Reply::error(400, "yaml is required"),
    };
    if yaml.len() > MAX_YAML_BYTES {
        return Reply::error(400, "yaml is too large");
    }
    let display = display_name(&value);
    let slug = slugify(&display);
    let enabled = background_enabled(&value);
    if !enabled && (yaml.contains("lovelace-background:") || yaml.contains("/local/")) {
        return Reply::error(
            400,
            "background is off, so the theme cannot reference a wallpaper",
        );
    }
    let mut warnings = extra_warnings(&value);
    let wallpaper = if enabled {
        let background = value.get("background");
        let relative = background
            .and_then(|item| item.get("path"))
            .and_then(Json::as_str);
        let encoded = background
            .and_then(|item| item.get("data"))
            .and_then(Json::as_str);
        let (Some(relative), Some(encoded)) = (relative, encoded) else {
            return Reply::error(400, "background image is missing");
        };
        let bytes = match decode_base64(encoded) {
            Ok(bytes) => bytes,
            Err(_) => return Reply::error(400, "background image is not base64"),
        };
        if bytes.len() > homechameleon_core::MAX_IMAGE_BYTES {
            return Reply::error(413, "image is over 10 MB");
        }
        let path = match store::background_path(&settings.config_dir, &slug, relative) {
            Ok(path) => path,
            Err(err) => return Reply::error(400, &err),
        };
        Some((path, bytes, relative.to_string()))
    } else {
        None
    };
    let www_existed = store::www_directory_exists(&settings.config_dir);
    let yaml_path = match store::theme_yaml_path(&settings.config_dir, &slug) {
        Ok(path) => path,
        Err(err) => return Reply::error(400, &err),
    };
    if let Err(err) = store::atomic_write(&settings.config_dir, &yaml_path, yaml.as_bytes()) {
        return Reply::error(400, &err);
    }
    let mut files = vec![format!("themes/{slug}.yaml")];
    if let Some((path, bytes, relative)) = wallpaper {
        if let Err(err) = store::atomic_write(&settings.config_dir, &path, &bytes) {
            return Reply::error(400, &err);
        }
        files.push(format!("www/homechameleon/{relative}"));
        if !www_existed {
            warnings.push("Restart Home Assistant once so files under /local/ are served.".into());
        }
    }
    if let Some(warning) = store::themes_include_warning(&settings.config_dir) {
        warnings.push(warning);
    }
    let default_set = finish_write(
        settings,
        &display,
        flag(&value, "set_default"),
        &mut warnings,
    );
    Reply::json(
        200,
        &public_body(&display, &files, true, default_set, &warnings, None, None),
    )
}

fn generate(body: &str, settings: &Settings, allow_write: bool) -> Reply {
    let value = match object(body) {
        Ok(value) => value,
        Err(reply) => return reply,
    };
    let generated = match generate_json(body) {
        Ok(text) => text,
        Err(err) => return Reply::error(error_status(&err), &err.to_string()),
    };
    let result = match parse_json(&generated) {
        Ok(result) => result,
        Err(err) => return Reply::error(500, &err),
    };
    if let Some(message) = result.get("error").and_then(Json::as_str) {
        return Reply::error(400, message);
    }
    let mut warnings = extra_warnings(&value);
    if let Some(items) = result.get("warnings").and_then(Json::as_array) {
        for item in items {
            if let Some(text) = item.as_str() {
                warnings.push(text.to_string());
            }
        }
    }
    let theme = result
        .get("theme")
        .and_then(Json::as_str)
        .unwrap_or("theme")
        .to_string();
    let slug = result
        .get("slug")
        .and_then(Json::as_str)
        .unwrap_or("")
        .to_string();
    let yaml = result.get("yaml").and_then(Json::as_str).unwrap_or("");
    let write = allow_write && value.get("apply").and_then(Json::as_bool).unwrap_or(true);
    let mut files = Vec::new();
    let mut applied = false;
    let mut default_set = false;
    if write {
        if yaml.is_empty() || yaml.len() > MAX_YAML_BYTES {
            return Reply::error(400, "yaml is too large");
        }
        let yaml_path = match store::theme_yaml_path(&settings.config_dir, &slug) {
            Ok(path) => path,
            Err(err) => return Reply::error(400, &err),
        };
        if let Err(err) = store::atomic_write(&settings.config_dir, &yaml_path, yaml.as_bytes()) {
            return Reply::error(400, &err);
        }
        files.push(format!("themes/{slug}.yaml"));
        applied = true;
        if background_enabled(&value) {
            warnings.push(
                "Background is on, but no image was sent, so no wallpaper was written.".into(),
            );
        }
        if let Some(warning) = store::themes_include_warning(&settings.config_dir) {
            warnings.push(warning);
        }
        default_set = finish_write(settings, &theme, flag(&value, "set_default"), &mut warnings);
    }
    let contrast = result.get("contrast").cloned();
    let palette = palette_field(&result);
    let mut body = public_body(
        &theme,
        &files,
        applied,
        default_set,
        &warnings,
        contrast.as_ref(),
        Some(&palette),
    );
    if !write {
        if let Json::Object(pairs) = &mut body {
            pairs.push(("yaml".into(), Json::from(yaml)));
        }
    }
    Reply::json(200, &body)
}

fn delete(slug: &str, settings: &Settings) -> Reply {
    let slug = match percent_decode(slug) {
        Ok(slug) if store::valid_slug(&slug) => slug,
        _ => return Reply::error(400, "theme name is not a safe file name"),
    };
    match store::delete_theme(&settings.config_dir, &slug) {
        Ok(true) => {
            let mut warnings = Vec::new();
            if let Some(warning) = reload(settings) {
                warnings.push(warning);
            }
            Reply::json(
                200,
                &Json::Object(vec![
                    ("theme".into(), Json::from(slug.as_str())),
                    ("deleted".into(), Json::from(true)),
                    ("warnings".into(), string_array(&warnings)),
                ]),
            )
        }
        Ok(false) => Reply::error(404, "theme not found"),
        Err(err) => Reply::error(400, &err),
    }
}

fn finish_write(
    settings: &Settings,
    theme: &str,
    set_default: bool,
    warnings: &mut Vec<String>,
) -> bool {
    if let Some(warning) = reload(settings) {
        warnings.push(warning);
    }
    if !set_default {
        return false;
    }
    match call(settings, "set_theme", &theme_body(theme)) {
        Ok(()) => true,
        Err(err) => {
            warnings.push(err);
            false
        }
    }
}

fn reload(settings: &Settings) -> Option<String> {
    call(settings, "reload_themes", "{}").err()
}

fn call(settings: &Settings, service: &str, payload: &str) -> Result<(), String> {
    if !supervisor::token_ok(&settings.token) {
        return Err(format!(
            "frontend.{service} did not run: Supervisor token is missing"
        ));
    }
    post_service(&settings.supervisor, &settings.token, service, payload)
}

fn theme_body(theme: &str) -> String {
    Json::Object(vec![
        ("name".into(), Json::from(theme)),
        ("name_dark".into(), Json::from(theme)),
    ])
    .stringify()
}

fn public_body(
    theme: &str,
    files: &[String],
    applied: bool,
    default_set: bool,
    warnings: &[String],
    contrast: Option<&Json>,
    palette: Option<&Json>,
) -> Json {
    let contrast = contrast.cloned().unwrap_or_else(|| {
        Json::Object(vec![
            ("pass".into(), Json::from(true)),
            ("checks".into(), Json::Array(vec![])),
        ])
    });
    let palette = palette.cloned().unwrap_or_else(|| Json::Object(vec![]));
    Json::Object(vec![
        ("theme".into(), Json::from(theme)),
        ("files".into(), string_array(files)),
        ("palette".into(), palette),
        ("contrast".into(), contrast),
        ("applied".into(), Json::from(applied)),
        ("default_set".into(), Json::from(default_set)),
        ("warnings".into(), string_array(warnings)),
    ])
}

fn palette_field(result: &Json) -> Json {
    let mut pairs = Vec::new();
    if let Some(light) = result.get("light") {
        pairs.push(("light".into(), light.clone()));
    }
    if let Some(dark) = result.get("dark") {
        pairs.push(("dark".into(), dark.clone()));
    }
    Json::Object(pairs)
}

fn object(body: &str) -> Result<Json, Reply> {
    match parse_json(body) {
        Ok(value) if value.as_object().is_some() => Ok(value),
        Ok(_) => Err(Reply::error(400, "request JSON must be an object")),
        Err(_) => Err(Reply::error(400, "request JSON could not be read")),
    }
}

fn display_name(value: &Json) -> String {
    value
        .get("name")
        .and_then(Json::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("theme")
        .to_string()
}

fn background_enabled(value: &Json) -> bool {
    value
        .get("background")
        .and_then(|item| item.get("enabled"))
        .and_then(Json::as_bool)
        .unwrap_or(false)
}

fn flag(value: &Json, key: &str) -> bool {
    value.get(key).and_then(Json::as_bool).unwrap_or(false)
}

fn extra_warnings(value: &Json) -> Vec<String> {
    if value.get("dashboard").is_some() {
        vec!["Starter dashboards are not built by this version.".into()]
    } else {
        Vec::new()
    }
}

fn string_array(items: &[String]) -> Json {
    Json::Array(items.iter().map(|item| Json::from(item.as_str())).collect())
}

fn error_status(err: &GenerateError) -> u16 {
    match err {
        GenerateError::ImageTooLarge { .. } => 413,
        _ => 400,
    }
}

pub fn percent_decode(input: &str) -> Result<String, ()> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                if index + 2 >= bytes.len() {
                    return Err(());
                }
                let high = from_hex(bytes[index + 1])?;
                let low = from_hex(bytes[index + 2])?;
                out.push((high << 4) | low);
                index += 3;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| ())
}

fn from_hex(byte: u8) -> Result<u8, ()> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(()),
    }
}

pub fn content_type_for(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("woff2") => "font/woff2",
        Some("txt") => "text/plain; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}
