//! Ingress server for the HomeChameleon add-on.
//!
//! The editor is the web build. Theme generation is homechameleon-core.
//! Files are written only under /config/themes and /config/www/homechameleon.

#![forbid(unsafe_code)]

mod api;
mod store;
mod supervisor;

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use api::{content_type_for, Reply, MAX_BODY_BYTES};

#[derive(Clone, Debug)]
struct Settings {
    config_dir: PathBuf,
    web_dir: PathBuf,
    supervisor: String,
    token: String,
}

impl Settings {
    fn from_env() -> Self {
        let token = std::env::var("SUPERVISOR_TOKEN").unwrap_or_default();
        let token = if supervisor::token_ok(&token) {
            token
        } else {
            String::new()
        };
        Self {
            config_dir: PathBuf::from("/config"),
            web_dir: PathBuf::from("/opt/homechameleon/www"),
            supervisor: "http://supervisor/core/api".into(),
            token,
        }
    }
}

fn main() {
    let settings = Settings::from_env();
    if settings.token.is_empty() {
        eprintln!(
            "Supervisor token is missing. Theme files can still be saved. Home Assistant will not be asked to reload themes."
        );
    } else {
        eprintln!("Using the Supervisor token for Home Assistant.");
    }
    let port = 8099;
    let listener = match TcpListener::bind(("0.0.0.0", port)) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("HomeChameleon add-on could not listen on {port}: {err}");
            std::process::exit(1);
        }
    };
    eprintln!("HomeChameleon add-on listening on {port}");
    let stop = Arc::new(AtomicBool::new(false));
    serve(listener, settings, &stop);
}

fn serve(listener: TcpListener, settings: Settings, stop: &AtomicBool) {
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                if let Err(err) = handle_connection(stream, &settings) {
                    eprintln!("connection closed: {err}");
                }
            }
            Err(err) => {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                eprintln!("accept failed: {err}");
                break;
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream, settings: &Settings) -> std::io::Result<()> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
    let (method, target, headers) = match read_headers(&mut stream) {
        Ok(parts) => parts,
        Err(err) => {
            write_reply(&mut stream, &Reply::error(400, "request could not be read"))?;
            return Err(err);
        }
    };
    let length = match content_length(&headers) {
        Ok(length) => length,
        Err(()) => {
            write_reply(&mut stream, &Reply::error(400, "send a Content-Length"))?;
            return Ok(());
        }
    };
    if length > MAX_BODY_BYTES {
        write_reply(&mut stream, &Reply::error(413, "request is over 16 MB"))?;
        return Ok(());
    }
    let mut body = vec![0u8; length];
    if length > 0 {
        stream.read_exact(&mut body)?;
    }
    let (path, _query) = match split_target(&target) {
        Ok(parts) => parts,
        Err(()) => {
            write_reply(
                &mut stream,
                &Reply::error(400, "request path is not usable"),
            )?;
            return Ok(());
        }
    };
    let reply = if method == "GET" && !path.starts_with("/api/") && path != "/health" {
        static_reply(settings, &path)
    } else if matches!(method.as_str(), "POST" | "DELETE" | "GET") {
        match std::str::from_utf8(&body) {
            Ok(text) => api::route(&method, &path, text, settings),
            Err(_) => Reply::error(400, "request JSON could not be read"),
        }
    } else {
        Reply::error(405, "method is not allowed")
    };
    eprintln!("{method} {path} -> {}", reply.status);
    write_reply(&mut stream, &reply)
}

fn static_reply(settings: &Settings, path: &str) -> Reply {
    let request_path = if path == "/" { "/index.html" } else { path };
    let Some(file) = safe_static(&settings.web_dir, request_path) else {
        return missing_editor(path);
    };
    let Ok(bytes) = fs::read(&file) else {
        return missing_editor(path);
    };
    if file.extension().and_then(|ext| ext.to_str()) == Some("html") {
        return Reply::text(200, content_type_for(&file), with_addon_meta(&bytes));
    }
    Reply::text(200, content_type_for(&file), bytes)
}

fn missing_editor(path: &str) -> Reply {
    if path == "/" || path == "/index.html" {
        Reply::text(
            200,
            "text/html; charset=utf-8",
            with_addon_meta(
                b"<!doctype html><meta charset=\"utf-8\"><title>HomeChameleon</title><p>The HomeChameleon editor is not in this add-on image.</p>",
            ),
        )
    } else {
        Reply::error(404, "not found")
    }
}

fn with_addon_meta(html: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(html);
    const TAG: &str = "<meta name=\"homechameleon-host\" content=\"addon\" />";
    if text.contains("name=\"homechameleon-host\"") {
        return text.into_owned().into_bytes();
    }
    if let Some(at) = text.find("</head>") {
        let mut out = String::with_capacity(text.len() + TAG.len());
        out.push_str(&text[..at]);
        out.push_str(TAG);
        out.push_str(&text[at..]);
        return out.into_bytes();
    }
    let mut out = String::from(TAG);
    out.push_str(&text);
    out.into_bytes()
}

fn safe_static(root: &Path, url_path: &str) -> Option<PathBuf> {
    let decoded = api::percent_decode(url_path).ok()?;
    let relative = decoded.trim_start_matches('/');
    if relative.contains('\0') {
        return None;
    }
    let mut clean = PathBuf::new();
    for part in Path::new(relative).components() {
        match part {
            Component::Normal(name) => clean.push(name),
            Component::CurDir => {}
            _ => return None,
        }
    }
    let root = root.canonicalize().ok()?;
    let file = root.join(clean).canonicalize().ok()?;
    if file.starts_with(&root) && file.is_file() {
        Some(file)
    } else {
        None
    }
}

type Header = (String, String);

fn read_headers(stream: &mut TcpStream) -> std::io::Result<(String, String, Vec<Header>)> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        if buf.len() > 32 * 1024 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "headers are too large",
            ));
        }
        if stream.read(&mut byte)? == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "connection closed",
            ));
        }
        buf.push(byte[0]);
        if buf.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let text = String::from_utf8(buf)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    let mut lines = text.split("\r\n");
    let Some(request) = lines.next() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "empty request",
        ));
    };
    let mut parts = request.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("").to_string();
    if method.is_empty() || target.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "bad request line",
        ));
    }
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "bad header",
            ));
        };
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
    }
    Ok((method, target, headers))
}

fn content_length(headers: &[Header]) -> Result<usize, ()> {
    let mut found = None;
    for (name, value) in headers {
        if name == "transfer-encoding" {
            return Err(());
        }
        if name == "content-length" {
            if found.is_some() {
                return Err(());
            }
            found = Some(value.parse::<usize>().map_err(|_| ())?);
        }
    }
    Ok(found.unwrap_or(0))
}

fn split_target(target: &str) -> Result<(String, String), ()> {
    if !target.starts_with('/') || target.starts_with("//") {
        return Err(());
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    Ok((path.to_string(), query.to_string()))
}

fn write_reply(stream: &mut TcpStream, reply: &Reply) -> std::io::Result<()> {
    let reason = match reply.status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Content Too Large",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let header = format!(
        "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        reply.status,
        reply.content_type,
        reply.bytes.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(&reply.bytes)?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use homechameleon_core::Json;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread::{self, JoinHandle};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Running {
        port: u16,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl Drop for Running {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            let _ = TcpStream::connect(("127.0.0.1", self.port));
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    struct Hit {
        header: String,
    }

    fn temp_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "homechameleon-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn start(settings: Settings) -> Running {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let thread = thread::spawn(move || serve(listener, settings, &flag));
        Running {
            port,
            stop,
            thread: Some(thread),
        }
    }

    fn exchange(port: u16, method: &str, path: &str, body: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes()).unwrap();
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let (head, body) = text.split_once("\r\n\r\n").unwrap();
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        (status, body.to_string())
    }

    fn spawn_supervisor() -> (String, Arc<Mutex<Vec<Hit>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        thread::spawn(move || {
            for _ in 0..8 {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let mut buf = vec![0u8; 8192];
                let count = stream.read(&mut buf).unwrap_or(0);
                let text = String::from_utf8_lossy(&buf[..count]).to_string();
                recorded.lock().unwrap().push(Hit { header: text });
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                );
            }
        });
        (format!("http://127.0.0.1:{port}/core/api"), seen)
    }

    #[test]
    fn apply_uses_the_supervisor_token_and_leaves_configuration_alone() {
        let config = temp_dir("apply");
        fs::write(config.join("configuration.yaml"), "default_config:\n").unwrap();
        let original = fs::read(config.join("configuration.yaml")).unwrap();
        let (supervisor, hits) = spawn_supervisor();
        let running = start(Settings {
            config_dir: config.clone(),
            web_dir: temp_dir("www-empty"),
            supervisor,
            token: "supervisor-token".into(),
        });
        let body = Json::Object(vec![
            ("name".into(), Json::from("Desk")),
            ("yaml".into(), Json::from("Desk:\n  modes:\n    dark: {}\n")),
            ("set_default".into(), Json::from(true)),
            ("token".into(), Json::from("user-token")),
            (
                "background".into(),
                Json::Object(vec![("enabled".into(), Json::from(false))]),
            ),
        ])
        .stringify();
        let (status, response) = exchange(running.port, "POST", "/api/apply", &body);
        assert_eq!(status, 200, "{response}");
        let written = fs::read_to_string(config.join("themes").join("desk.yaml")).unwrap();
        assert_eq!(written, "Desk:\n  modes:\n    dark: {}\n");
        assert_eq!(
            fs::read(config.join("configuration.yaml")).unwrap(),
            original
        );
        assert!(!config.join("www").exists());
        let parsed = homechameleon_core::parse_json(&response).unwrap();
        assert_eq!(parsed.get("applied").and_then(Json::as_bool), Some(true));
        assert_eq!(
            parsed.get("default_set").and_then(Json::as_bool),
            Some(true)
        );
        let warnings = parsed.get("warnings").and_then(Json::as_array).unwrap();
        assert!(warnings
            .iter()
            .any(|item| item.as_str().unwrap().contains("does not edit that file")));
        let recorded = hits.lock().unwrap();
        assert!(recorded.len() >= 2);
        for hit in recorded.iter() {
            assert!(hit
                .header
                .contains("Authorization: Bearer supervisor-token"));
            assert!(!hit.header.contains("user-token"));
            assert!(hit.header.contains("/core/api/services/frontend/"));
        }
        let _ = fs::remove_dir_all(&config);
    }

    #[test]
    fn generate_reuses_the_core_and_preview_writes_nothing() {
        let config = temp_dir("generate");
        fs::write(
            config.join("configuration.yaml"),
            format!("{}\n", store::THEMES_INCLUDE),
        )
        .unwrap();
        let running = start(Settings {
            config_dir: config.clone(),
            web_dir: temp_dir("www-empty"),
            supervisor: "http://127.0.0.1:1/core/api".into(),
            token: String::new(),
        });
        let tokyo = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../omarchy/tests/fixtures/tokyo-night.colors.toml"),
        )
        .unwrap();
        let body = Json::Object(vec![
            ("name".into(), Json::from("Tokyo Night")),
            ("style".into(), Json::from("sheer")),
            ("mode".into(), Json::from("dark")),
            (
                "palette".into(),
                Json::Object(vec![
                    ("format".into(), Json::from("omarchy-colors-toml")),
                    ("data".into(), Json::from(tokyo.as_str())),
                ]),
            ),
            (
                "background".into(),
                Json::Object(vec![("enabled".into(), Json::from(false))]),
            ),
            ("apply".into(), Json::from(true)),
            ("set_default".into(), Json::from(false)),
        ])
        .stringify();
        let (status, response) = exchange(running.port, "POST", "/api/generate", &body);
        assert_eq!(status, 200, "{response}");
        let golden = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/homechameleon-core/tests/golden/fixture.txt"),
        )
        .unwrap();
        let section = golden
            .split("===== tokyo-sheer-dark =====\n")
            .nth(1)
            .unwrap();
        let expected = section.split("--- css ---\n").next().unwrap();
        let written = fs::read_to_string(config.join("themes").join("tokyo-night.yaml")).unwrap();
        assert_eq!(written, expected);
        let preview = body.replace("\"apply\":true", "\"apply\":false");
        let before = fs::read_dir(config.join("themes")).unwrap().count();
        let (status, response) = exchange(running.port, "POST", "/api/preview", &preview);
        assert_eq!(status, 200, "{response}");
        let parsed = homechameleon_core::parse_json(&response).unwrap();
        assert_eq!(parsed.get("applied").and_then(Json::as_bool), Some(false));
        assert_eq!(parsed.get("yaml").and_then(Json::as_str), Some(expected));
        assert_eq!(fs::read_dir(config.join("themes")).unwrap().count(), before);
        let (status, _) = exchange(
            running.port,
            "POST",
            "/api/generate",
            &body.replace("\"style\":\"sheer\"", "\"style\":\"nope\""),
        );
        assert_eq!(status, 400);
        let _ = fs::remove_dir_all(&config);
    }

    #[test]
    fn wallpaper_and_delete_stay_in_the_homechameleon_folder() {
        let config = temp_dir("wall");
        fs::write(config.join("configuration.yaml"), "default_config:\n").unwrap();
        let running = start(Settings {
            config_dir: config.clone(),
            web_dir: temp_dir("www-empty"),
            supervisor: "http://127.0.0.1:1/core/api".into(),
            token: String::new(),
        });
        let payload = b"image-bytes";
        let data = Json::from("aW1hZ2UtYnl0ZXM=");
        assert_eq!(
            homechameleon_core::decode_base64("aW1hZ2UtYnl0ZXM=").unwrap(),
            payload
        );
        let body = Json::Object(vec![
            ("name".into(), Json::from("Desk")),
            ("yaml".into(), Json::from("Desk:\n  modes:\n    dark:\n      lovelace-background: \"/local/homechameleon/desk/bg-0123abcd.webp\"\n")),
            ("background".into(), Json::Object(vec![
                ("enabled".into(), Json::from(true)),
                ("path".into(), Json::from("desk/bg-0123abcd.webp")),
                ("data".into(), data),
            ])),
        ])
        .stringify();
        let (status, response) = exchange(running.port, "POST", "/api/apply", &body);
        assert_eq!(status, 200, "{response}");
        let wallpaper = config
            .join("www")
            .join("homechameleon")
            .join("desk")
            .join("bg-0123abcd.webp");
        assert_eq!(fs::read(&wallpaper).unwrap(), payload);
        let (status, _) = exchange(running.port, "DELETE", "/api/theme/desk", "");
        assert_eq!(status, 200);
        assert!(!wallpaper.exists());
        assert!(config.join("configuration.yaml").is_file());
        let _ = fs::remove_dir_all(&config);
    }

    #[test]
    fn background_off_rejects_a_wallpaper_reference() {
        let config = temp_dir("off");
        let running = start(Settings {
            config_dir: config.clone(),
            web_dir: temp_dir("www-empty"),
            supervisor: "http://127.0.0.1:1/core/api".into(),
            token: String::new(),
        });
        let body = r#"{"name":"Desk","yaml":"Desk:\n  lovelace-background: \"/local/x\"\n","background":{"enabled":false}}"#;
        let (status, _) = exchange(running.port, "POST", "/api/apply", body);
        assert_eq!(status, 400);
        assert!(!config.join("themes").exists());
        let _ = fs::remove_dir_all(&config);
    }

    #[test]
    fn ingress_html_is_marked_as_the_add_on() {
        let web = temp_dir("editor");
        fs::write(
            web.join("index.html"),
            "<html><head><title>HomeChameleon</title></head><body></body></html>",
        )
        .unwrap();
        let running = start(Settings {
            config_dir: temp_dir("cfg"),
            web_dir: web,
            supervisor: "http://127.0.0.1:1/core/api".into(),
            token: String::new(),
        });
        let (status, body) = exchange(running.port, "GET", "/", "");
        assert_eq!(status, 200);
        assert!(body.contains("name=\"homechameleon-host\" content=\"addon\""));
    }

    #[test]
    fn environment_reads_only_the_supervisor_token() {
        let _guard = ENV.lock().unwrap();
        let _clear = ClearEnv;
        std::env::set_var("SUPERVISOR_TOKEN", "supervisor-token");
        std::env::set_var("HASS_TOKEN", "user-token");
        std::env::set_var("LONG_LIVED_ACCESS_TOKEN", "user-token");
        let settings = Settings::from_env();
        assert_eq!(settings.token, "supervisor-token");
        assert_eq!(settings.config_dir, PathBuf::from("/config"));
        assert_eq!(settings.supervisor, "http://supervisor/core/api");
        assert_eq!(settings.web_dir, PathBuf::from("/opt/homechameleon/www"));
    }

    static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct ClearEnv;
    impl Drop for ClearEnv {
        fn drop(&mut self) {
            std::env::remove_var("SUPERVISOR_TOKEN");
            std::env::remove_var("HASS_TOKEN");
            std::env::remove_var("LONG_LIVED_ACCESS_TOKEN");
        }
    }
}
