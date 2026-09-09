//! End-to-end test of the embedded server with a raw HTTP client.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use qidir_core::config::{AppPaths, IndexRoot, Settings};
use qidir_core::Engine;
use qidir_server::{serve, ServerConfig};

fn http(port: u16, req: &str) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    s.write_all(req.as_bytes()).unwrap();
    let mut buf = Vec::new();
    s.read_to_end(&mut buf).unwrap();
    let text = String::from_utf8_lossy(&buf).into_owned();
    let status: u16 = text.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let body = text.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body)
}

fn post(port: u16, path: &str, token: Option<&str>, body: &str, origin: Option<&str>) -> (u16, String) {
    let mut req = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
        body.len()
    );
    if let Some(t) = token {
        req.push_str(&format!("X-QIDIR-Token: {t}\r\n"));
    }
    if let Some(o) = origin {
        req.push_str(&format!("Origin: {o}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(body);
    http(port, &req)
}

#[test]
fn serves_ui_and_api() {
    let data = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    std::fs::write(docs.path().join("a.txt"), "Договор аренды офиса").unwrap();
    let mut settings = Settings::default();
    settings.roots.push(IndexRoot { path: docs.path().to_path_buf(), enabled: true });
    settings.watch_changes = false;
    let paths = AppPaths::in_dir(data.path());
    let engine: Arc<Engine> = Engine::open_with_settings(paths.clone(), settings).unwrap();
    engine.index_blocking(false).unwrap();

    let running =
        serve(engine, paths, ServerConfig { port: 0, open_browser: false, idle_timeout: None }).unwrap();
    let port = running.port;

    // The page is served with the token injected.
    let (status, html) = http(port, "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    assert_eq!(status, 200);
    let start = html.find("__QIDIR_TOKEN__=\"").expect("token injected") + "__QIDIR_TOKEN__=\"".len();
    let token = &html[start..start + 32];

    // No token → forbidden; wrong origin → forbidden.
    assert_eq!(post(port, "/api/ping", None, "{}", None).0, 403);
    assert_eq!(post(port, "/api/ping", Some(token), "{}", Some("http://evil.example")).0, 403);

    // Search through the API.
    let (status, body) =
        post(port, "/api/search", Some(token), r#"{"request":{"query":"договоры","limit":10}}"#, None);
    assert_eq!(status, 200, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["ok"], true, "{body}");
    assert_eq!(v["result"]["total"], 1, "{body}");
    assert!(v["result"]["hits"][0]["snippet"].as_str().unwrap().contains("<mark>Договор</mark>"));

    // Unknown command → error payload, not a crash.
    let (_, body) = post(port, "/api/nope", Some(token), "{}", None);
    assert!(body.contains("unknown command"));

    // Progress and app info.
    let (_, body) = post(port, "/api/get_app_info", Some(token), "{}", None);
    assert!(body.contains("\"mode\":\"browser\""));

    running.shutdown();
    running.wait();
}
