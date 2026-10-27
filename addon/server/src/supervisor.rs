//! Home Assistant calls go through the Supervisor proxy and its token.
//! A user access token is never read and never sent.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub fn token_ok(token: &str) -> bool {
    let bytes = token.as_bytes();
    !bytes.is_empty() && bytes.len() <= 4096 && bytes.iter().all(u8::is_ascii_graphic)
}

pub fn post_service(
    supervisor: &str,
    token: &str,
    service: &str,
    payload: &str,
) -> Result<(), String> {
    if !token_ok(token) {
        return Err(format!(
            "frontend.{service} did not run: Supervisor token is missing"
        ));
    }
    if !service
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(format!("frontend.{service} did not run"));
    }
    let (host, port, root) = parse_supervisor(supervisor)?;
    let path = format!("{root}/services/frontend/{service}");
    let host_header = if port == 80 {
        host.clone()
    } else {
        format!("{host}:{port}")
    };
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {host_header}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        payload.len()
    );
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|err| format!("frontend.{service} did not run: {err}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|err| format!("frontend.{service} did not run: {err}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .map_err(|err| format!("frontend.{service} did not run: {err}"))?;
    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("frontend.{service} did not run: {err}"))?;
    let mut bytes = Vec::new();
    stream
        .read_to_end(&mut bytes)
        .map_err(|err| format!("frontend.{service} did not run: {err}"))?;
    let text = String::from_utf8_lossy(&bytes);
    let status = status_code(&text).ok_or_else(|| format!("frontend.{service} did not run"))?;
    if (200..300).contains(&status) {
        Ok(())
    } else {
        Err(format!("frontend.{service} did not run: HTTP {status}"))
    }
}

fn parse_supervisor(url: &str) -> Result<(String, u16, String), String> {
    let Some(rest) = url.strip_prefix("http://") else {
        return Err("frontend service did not run: Supervisor address is not reachable".into());
    };
    if rest.is_empty() || rest.contains('@') || rest.contains(' ') || rest.contains('\\') {
        return Err("frontend service did not run: Supervisor address is not reachable".into());
    }
    let (authority, path) = match rest.split_once('/') {
        Some((authority, path)) => (
            authority,
            format!("/{path}").trim_end_matches('/').to_string(),
        ),
        None => (rest, String::new()),
    };
    if authority.is_empty() {
        return Err("frontend service did not run: Supervisor address is not reachable".into());
    }
    let (host, port) = if let Some((host, port)) = authority.rsplit_once(':') {
        let port = port.parse::<u16>().map_err(|_| {
            "frontend service did not run: Supervisor address is not reachable".to_string()
        })?;
        (host.to_string(), port)
    } else {
        (authority.to_string(), 80)
    };
    if host.is_empty() {
        return Err("frontend service did not run: Supervisor address is not reachable".into());
    }
    Ok((host, port, path))
}

fn status_code(response: &str) -> Option<u16> {
    let line = response.lines().next()?;
    let mut parts = line.split_whitespace();
    let _http = parts.next()?;
    parts.next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    #[test]
    fn service_calls_send_the_supervisor_token() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 2048];
            let count = stream.read(&mut buf).unwrap_or(0);
            recorded
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&buf[..count]).to_string());
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
        });
        post_service(
            &format!("http://127.0.0.1:{port}/core/api"),
            "supervisor-token",
            "reload_themes",
            "{}",
        )
        .unwrap();
        let hits = seen.lock().unwrap();
        let hit = &hits[0];
        assert!(hit.contains("POST /core/api/services/frontend/reload_themes "));
        assert!(hit.contains("Authorization: Bearer supervisor-token"));
        assert!(!hit.contains("user-token"));
    }

    #[test]
    fn a_user_token_string_is_not_a_supervisor_token() {
        assert!(token_ok("supervisor-token"));
        assert!(!token_ok(""));
        assert!(!token_ok("line\nbreak"));
    }
}
