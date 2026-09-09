//! A tiny embedded HTTP server that exposes the QIDIR engine to the bundled
//! web UI so the application works in **any browser**, without the WebView2
//! runtime. Used by the desktop app as a fallback (`QIDIR.exe --browser` or
//! when WebView2 is missing) and by `qidir serve`.
//!
//! Security model: the server binds to `127.0.0.1` only, every API call must
//! carry a per-process random token that is injected into the served page,
//! and cross-origin requests are rejected. Nothing is reachable from the
//! network or from other web sites open in the browser.

mod api;
pub mod shell;

use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use include_dir::{include_dir, Dir};
use parking_lot::Mutex;
use qidir_core::config::AppPaths;
use qidir_core::Engine;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

/// The compiled Svelte frontend (`apps/desktop/dist`), embedded at build time.
static DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../apps/desktop/dist");

/// Server options.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// TCP port; `0` picks a free one.
    pub port: u16,
    /// Open the default browser once the server is up.
    pub open_browser: bool,
    /// Exit when no UI has talked to the server for this long (the page
    /// sends a heartbeat while open). `None` = run forever.
    pub idle_timeout: Option<Duration>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { port: 0, open_browser: true, idle_timeout: Some(Duration::from_secs(10 * 60)) }
    }
}

/// Shared state of a running server.
pub struct ServerState {
    pub engine: Arc<Engine>,
    pub paths: AppPaths,
    pub token: String,
    pub last_seen: Mutex<Instant>,
    pub shutdown: AtomicBool,
}

/// Handle to a server running on a background thread.
pub struct Running {
    pub url: String,
    pub port: u16,
    state: Arc<ServerState>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Running {
    /// Block until the server stops (shutdown request or idle timeout).
    pub fn wait(mut self) {
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }

    pub fn shutdown(&self) {
        self.state.shutdown.store(true, Ordering::SeqCst);
    }
}

/// Start the server on a background thread and return its URL.
pub fn serve(engine: Arc<Engine>, paths: AppPaths, cfg: ServerConfig) -> Result<Running, String> {
    let server = Server::http(("127.0.0.1", cfg.port)).map_err(|e| format!("cannot bind local port: {e}"))?;
    let port = server.server_addr().to_ip().map(|a| a.port()).unwrap_or(cfg.port);
    let token = random_token();
    let url = format!("http://127.0.0.1:{port}/");
    let state = Arc::new(ServerState {
        engine,
        paths,
        token,
        last_seen: Mutex::new(Instant::now()),
        shutdown: AtomicBool::new(false),
    });
    tracing::info!(%url, "QIDIR browser mode: local server started");

    let server = Arc::new(server);
    let accept_state = state.clone();
    let accept_server = server.clone();
    let idle = cfg.idle_timeout;
    let thread = std::thread::Builder::new()
        .name("qidir-http".into())
        .spawn(move || {
            loop {
                if accept_state.shutdown.load(Ordering::SeqCst) {
                    break;
                }
                if let Some(limit) = idle {
                    if accept_state.last_seen.lock().elapsed() > limit {
                        tracing::info!("no UI activity for {:?}, shutting down", limit);
                        break;
                    }
                }
                match accept_server.recv_timeout(Duration::from_millis(500)) {
                    Ok(Some(request)) => {
                        let st = accept_state.clone();
                        std::thread::spawn(move || handle(request, &st));
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::warn!(error = %e, "accept failed");
                        break;
                    }
                }
            }
            accept_server.unblock();
        })
        .map_err(|e| e.to_string())?;

    if cfg.open_browser {
        shell::open_url(&url);
    }
    Ok(Running { url, port, state, thread: Some(thread) })
}

fn random_token() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    // `RandomState` is seeded from the OS RNG; two independent draws give
    // 128 bits, plenty for a same-machine CSRF token.
    let mut out = String::with_capacity(32);
    for _ in 0..2 {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(std::process::id() as u64);
        h.write_u128(Instant::now().elapsed().as_nanos());
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

fn handle(mut request: Request, state: &ServerState) {
    let url = request.url().to_string();
    let path = url.split('?').next().unwrap_or("/").to_string();
    let method = request.method().clone();

    let response = if let Some(cmd) = path.strip_prefix("/api/") {
        if method != Method::Post {
            text(405, "method not allowed")
        } else if !token_ok(&request, state) {
            tracing::warn!(cmd, "rejected API call without a valid token");
            text(403, "forbidden")
        } else {
            *state.last_seen.lock() = Instant::now();
            let mut body = String::new();
            let _ = std::io::Read::read_to_string(request.as_reader(), &mut body);
            let args: serde_json::Value = if body.trim().is_empty() {
                serde_json::Value::Object(Default::default())
            } else {
                serde_json::from_str(&body).unwrap_or(serde_json::Value::Null)
            };
            match api::dispatch(state, cmd, args) {
                Ok(result) => json(200, serde_json::json!({ "ok": true, "result": result })),
                Err(e) => json(200, serde_json::json!({ "ok": false, "error": e })),
            }
        }
    } else if method == Method::Get || method == Method::Head {
        static_file(&path, state)
    } else {
        text(405, "method not allowed")
    };
    let _ = request.respond(response);
}

fn token_ok(request: &Request, state: &ServerState) -> bool {
    let header_token = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("x-qidir-token"))
        .map(|h| h.value.as_str().to_string());
    // Same-origin only: browsers attach an Origin header to cross-site
    // requests; a missing header means same-origin or a non-browser client.
    let origin_ok = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("origin"))
        .map(|h| {
            h.value.as_str().starts_with("http://127.0.0.1:")
                || h.value.as_str().starts_with("http://localhost:")
        })
        .unwrap_or(true);
    origin_ok && header_token.as_deref() == Some(state.token.as_str())
}

type Resp = Response<Cursor<Vec<u8>>>;

fn text(status: u16, body: &str) -> Resp {
    Response::from_string(body)
        .with_status_code(StatusCode(status))
        .with_header(header("Content-Type", "text/plain; charset=utf-8"))
}

fn json(status: u16, value: serde_json::Value) -> Resp {
    Response::from_string(value.to_string())
        .with_status_code(StatusCode(status))
        .with_header(header("Content-Type", "application/json; charset=utf-8"))
        .with_header(header("Cache-Control", "no-store"))
}

fn header(k: &str, v: &str) -> Header {
    Header::from_bytes(k.as_bytes(), v.as_bytes()).expect("static header")
}

fn static_file(path: &str, state: &ServerState) -> Resp {
    let rel = path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    let file = DIST.get_file(rel).or_else(|| DIST.get_file("index.html"));
    let Some(file) = file else {
        return text(404, "not found");
    };
    let is_index = file.path().to_string_lossy() == "index.html";
    let mime = mime_for(&file.path().to_string_lossy());
    let mut bytes = file.contents().to_vec();
    if is_index {
        // Inject the API token and switch the UI into browser mode.
        let inject = format!("<script>window.__QIDIR_TOKEN__=\"{}\";</script>", state.token);
        let html = String::from_utf8_lossy(&bytes).into_owned();
        let html = match html.find("<head>") {
            Some(i) => format!("{}{}{}", &html[..i + 6], inject, &html[i + 6..]),
            None => format!("{inject}{html}"),
        };
        bytes = html.into_bytes();
    }
    let cache = if is_index { "no-store" } else { "public, max-age=31536000, immutable" };
    Response::from_data(bytes)
        .with_status_code(StatusCode(200))
        .with_header(header("Content-Type", mime))
        .with_header(header("Cache-Control", cache))
        .with_header(header("X-Content-Type-Options", "nosniff"))
        .with_header(header("Referrer-Policy", "no-referrer"))
}

fn mime_for(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" | "md" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
