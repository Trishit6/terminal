//! SPA shell serving: per-route head patching, honest 404 statuses, clean
//! URL redirects, and API/static precedence.
//!
//! The manifest written by `scripts/seo.mjs` (`dist/routes.json`) drives the
//! server's SPA behaviour; these tests pin that contract against a fixture
//! web directory that mirrors the built `index.html` markers.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use shell_server::{AppState, router_with_static};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TempWebDir {
    path: PathBuf,
}

impl TempWebDir {
    fn new(shell_html: &str, manifest: Option<&str>) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "kernext-spa-{}-{unique}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(path.join("assets")).expect("temporary directory should be created");
        fs::write(path.join("index.html"), shell_html).expect("shell should be written");
        if let Some(manifest) = manifest {
            fs::write(path.join("routes.json"), manifest).expect("manifest should be written");
        }
        fs::write(path.join("assets").join("app.js"), "// bundled asset\n")
            .expect("asset should be written");
        Self { path }
    }
}

impl Drop for TempWebDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Mirrors the fallback head tags of the built `index.html`.
const SHELL: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
    <title data-kernext-static>KERNEXT — The Next Core of Computing.</title>
    <meta data-kernext-static name="description" content="fallback description" />
    <meta data-kernext-static name="robots" content="index,follow" />
    <meta data-kernext-static property="og:title" content="fallback title" />
    <meta data-kernext-static property="og:description" content="fallback description" />
    <meta data-kernext-static property="og:url" content="https://kernext.example.org/" />
    <link data-kernext-static rel="canonical" href="https://kernext.example.org/" />
  </head>
  <body><div id="root"></div></body>
</html>
"#;

const MANIFEST: &str = r#"{
  "site_url": "https://kernext.example.org",
  "routes": [
    {"path": "/", "title": "KERNEXT — The Next Core of Computing.", "description": "home description", "indexable": true},
    {"path": "/features", "title": "KERNEXT Features", "description": "features description", "indexable": true},
    {"path": "/terminal", "title": "Web terminal — KERNEXT", "description": "terminal description", "indexable": false}
  ],
  "not_found": {"title": "Page not found — KERNEXT", "description": "nothing here"}
}"#;

async fn send(app: &axum::Router, request: Request<Body>) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(request)
        .await
        .expect("router should answer")
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .body(Body::empty())
        .expect("request should build")
}

async fn text(mut response: axum::http::Response<Body>) -> (StatusCode, String, String) {
    let headers = std::mem::take(response.headers_mut());
    let status = response.status();
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let cache_control = headers
        .get(header::CACHE_CONTROL)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body should collect")
        .to_bytes();

    (
        status,
        String::from_utf8_lossy(&body).into_owned(),
        format!("{content_type}|{cache_control}"),
    )
}

#[tokio::test]
async fn known_route_serves_the_shell_with_patched_metadata() {
    let web = TempWebDir::new(SHELL, Some(MANIFEST));
    let app = router_with_static(AppState::from_env(), Some(web.path.clone()));

    let (status, body, headers) = text(send(&app, get("/features")).await).await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("<title data-kernext-static>KERNEXT Features</title>"),
        "title should be patched: {body}"
    );
    assert!(
        body.contains(r#"name="description" content="features description""#),
        "description should be patched: {body}"
    );
    assert!(
        body.contains(r#"property="og:title" content="KERNEXT Features""#),
        "og:title should be patched: {body}"
    );
    assert!(
        body.contains(r#"name="robots" content="index,follow""#),
        "indexable routes stay indexable: {body}"
    );
    assert!(
        body.contains(r#"href="https://kernext.example.org/features""#),
        "canonical should follow the configured origin: {body}"
    );
    assert!(headers.starts_with("text/html"), "got {headers}");
    assert!(headers.ends_with("no-cache"), "got {headers}");
}

#[tokio::test]
async fn unknown_route_serves_the_shell_with_a_real_404() {
    let web = TempWebDir::new(SHELL, Some(MANIFEST));
    let app = router_with_static(AppState::from_env(), Some(web.path.clone()));

    let (status, body, _) = text(send(&app, get("/nope")).await).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(
        body.contains("<title data-kernext-static>Page not found — KERNEXT</title>"),
        "not-found title should be patched: {body}"
    );
    assert!(
        body.contains(r#"name="robots" content="noindex""#),
        "unknown paths must not be indexed: {body}"
    );
    assert!(
        body.contains(r#"name="description" content="nothing here""#),
        "not-found description should be patched: {body}"
    );
}

#[tokio::test]
async fn private_route_is_served_with_noindex() {
    let web = TempWebDir::new(SHELL, Some(MANIFEST));
    let app = router_with_static(AppState::from_env(), Some(web.path.clone()));

    let (status, body, _) = text(send(&app, get("/terminal")).await).await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains(r#"name="robots" content="noindex""#),
        "private routes must carry noindex: {body}"
    );
}

#[tokio::test]
async fn trailing_slashes_redirect_to_the_clean_url() {
    let web = TempWebDir::new(SHELL, Some(MANIFEST));
    let app = router_with_static(AppState::from_env(), Some(web.path.clone()));

    let response = send(&app, get("/features/")).await;
    assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
    assert_eq!(
        response
            .headers()
            .get(header::LOCATION)
            .and_then(|value| value.to_str().ok()),
        Some("/features")
    );

    // The root keeps its single slash.
    let (status, _, _) = text(send(&app, get("/")).await).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn unknown_api_routes_keep_their_json_404() {
    let web = TempWebDir::new(SHELL, Some(MANIFEST));
    let app = router_with_static(AppState::from_env(), Some(web.path.clone()));

    let (status, body, headers) = text(send(&app, get("/api/does-not-exist")).await).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(headers.starts_with("application/json"), "got {headers}");
    assert!(
        body.contains("not_found"),
        "unknown API routes answer JSON, not the SPA shell: {body}"
    );
}

#[tokio::test]
async fn real_files_are_served_by_the_static_service() {
    let web = TempWebDir::new(SHELL, Some(MANIFEST));
    let app = router_with_static(AppState::from_env(), Some(web.path.clone()));

    let (status, body, headers) = text(send(&app, get("/assets/app.js")).await).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("bundled asset"));
    assert!(headers.starts_with("text/javascript"), "got {headers}");
}

#[tokio::test]
async fn head_requests_are_supported_on_known_routes() {
    let web = TempWebDir::new(SHELL, Some(MANIFEST));
    let app = router_with_static(AppState::from_env(), Some(web.path.clone()));

    let response = send(
        &app,
        Request::builder()
            .method("HEAD")
            .uri("/features")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn missing_manifest_keeps_the_legacy_shell_behaviour() {
    // A hand-copied dist without `routes.json` must not break client-side
    // routes: every path still receives the shell with status 200.
    let web = TempWebDir::new(SHELL, None);
    let app = router_with_static(AppState::from_env(), Some(web.path.clone()));

    let (status, body, _) = text(send(&app, get("/nope")).await).await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("<title data-kernext-static>KERNEXT — The Next Core of Computing.</title>"),
        "the unpatched shell is served as-is: {body}"
    );
}
