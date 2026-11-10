//! HTTP integration coverage for the KERNEXT server.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use shell_api::{
    CreateSessionResponse, ExecuteCommandResponse, HealthResponse, SessionListResponse,
};
use shell_server::{AppState, router};
use tower::ServiceExt;

async fn send(app: &axum::Router, request: Request<Body>) -> (StatusCode, Vec<u8>) {
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("router should answer");

    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body should collect")
        .to_bytes()
        .to_vec();

    (status, body)
}

fn json_request(method: &str, uri: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .expect("request should build")
}

#[tokio::test]
async fn health_reports_the_product_and_version() {
    let app = router(AppState::from_env());
    let (status, body) = send(
        &app,
        Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);

    let health: HealthResponse = serde_json::from_slice(&body).expect("health should be JSON");
    assert_eq!(health.status, "ok");
    assert_eq!(health.product, "KERNEXT");
    assert_eq!(health.api_version, "v1");
}

#[tokio::test]
async fn session_lifecycle_executes_real_commands() {
    let app = router(AppState::from_env());

    let (status, body) = send(&app, json_request("POST", "/api/v1/sessions", "{}")).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

    let created: CreateSessionResponse =
        serde_json::from_slice(&body).expect("create response should be JSON");
    let session = created.session.expect("session should be created");
    assert_eq!(session.state, "running");

    let uri = format!("/api/v1/sessions/{}/execute", session.id.0);
    let (status, body) = send(
        &app,
        json_request("POST", &uri, r#"{"command":"echo kernext-server"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

    let executed: ExecuteCommandResponse =
        serde_json::from_slice(&body).expect("execute response should be JSON");
    assert!(executed.success, "{executed:?}");
    assert!(executed.output.contains("kernext-server"), "{executed:?}");

    let (status, body) = send(
        &app,
        Request::builder()
            .uri("/api/v1/sessions")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let listed: SessionListResponse =
        serde_json::from_slice(&body).expect("list response should be JSON");
    assert_eq!(listed.sessions.len(), 1);

    let (status, _body) = send(
        &app,
        Request::builder()
            .method("DELETE")
            .uri(format!("/api/v1/sessions/{}", session.id.0))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn missing_sessions_return_not_found() {
    let app = router(AppState::from_env());
    let (status, body) = send(
        &app,
        Request::builder()
            .uri("/api/v1/sessions/9999")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(String::from_utf8_lossy(&body).contains("does not exist"));
}

#[tokio::test]
async fn client_side_routes_serve_the_spa_shell_with_ok() {
    let directory = std::env::temp_dir().join(format!(
        "kernext-spa-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock should be after the epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).expect("temporary web directory should be created");
    std::fs::write(
        directory.join("index.html"),
        "<!doctype html><title>KERNEXT SPA</title>",
    )
    .expect("index should be written");

    let app = shell_server::router_with_static(AppState::from_env(), Some(directory.clone()));
    let (status, body) = send(
        &app,
        Request::builder()
            .uri("/terminal")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(
        status,
        StatusCode::OK,
        "a client-side route must not look like a failed request"
    );
    assert!(String::from_utf8_lossy(&body).contains("KERNEXT SPA"));
}

#[tokio::test]
async fn oversized_commands_are_rejected() {
    let app = router(AppState::from_env());
    let (status, body) = send(&app, json_request("POST", "/api/v1/sessions", "{}")).await;
    assert_eq!(status, StatusCode::OK);

    let created: CreateSessionResponse =
        serde_json::from_slice(&body).expect("create response should be JSON");
    let id = created.session.expect("session should exist").id.0;

    let command = "a".repeat(shell_server::state::MAX_COMMAND_BYTES + 1);
    let payload = serde_json::json!({ "command": command }).to_string();
    let (status, _body) = send(
        &app,
        json_request("POST", &format!("/api/v1/sessions/{id}/execute"), &payload),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}
