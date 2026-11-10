//! REST routes for the KERNEXT server.

use crate::auth::{ALLOWED_ORIGINS, authorize_request};
use crate::error::ApiError;
use crate::state::{AppState, MAX_COMMAND_BYTES, ShellSession};
use crate::ws;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use shell_api::{
    API_VERSION, ApiErrorBody, CreateSessionRequest, CreateSessionResponse, ExecuteCommandRequest,
    ExecuteCommandResponse, HealthResponse, PRODUCT_NAME, SessionId, SessionInfo,
    SessionListResponse,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

/// Builds the complete server router.
pub fn router(state: AppState) -> Router {
    router_with_static(state, None)
}

/// Builds the router and optionally serves the built web client.
///
/// `web_dir` is expected to contain `index.html` and Vite's `assets/` folder.
/// Unmatched paths fall back to `index.html` so client-side router URLs work
/// after a refresh. API routes always take precedence over the static fallback.
pub fn router_with_static(state: AppState, web_dir: Option<PathBuf>) -> Router {
    let app = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/sessions", get(list_sessions).post(create_session))
        .route(
            "/api/v1/sessions/{id}",
            get(get_session).delete(delete_session),
        )
        .route("/api/v1/sessions/{id}/execute", post(execute_command))
        .route("/api/v1/sessions/{id}/ws", get(ws::session_socket))
        .route("/api/v1/pty", get(ws::pty_socket));

    let app = match web_dir {
        Some(directory) if directory.is_dir() => {
            let index = directory.join("index.html");
            // `fallback` keeps the index response's own 200 status;
            // `not_found_service` would force 404 even when the SPA shell is
            // served successfully, which makes a client-side route such as
            // `/terminal` look like a failed request on refresh.
            let service = ServeDir::new(directory).fallback(ServeFile::new(index));
            app.fallback_service(service)
        }
        _ => app.fallback(not_found),
    };

    app.layer(cors_layer())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn cors_layer() -> CorsLayer {
    let origins = ALLOWED_ORIGINS
        .iter()
        .map(|origin| HeaderValue::from_static(origin))
        .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_owned(),
        product: PRODUCT_NAME.to_owned(),
        version: shell_core::VERSION.to_owned(),
        api_version: API_VERSION.to_owned(),
        sessions: state.len(),
        platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        pty_available: ws::pty_binary().is_some(),
    })
}

async fn create_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, ApiError> {
    authorize_request(&state, &headers)?;
    let session = state.create(&request)?;
    let info = lock_session(&session)?.info();

    Ok(Json(CreateSessionResponse {
        success: true,
        session: Some(info),
        error: None,
    }))
}

async fn list_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SessionListResponse>, ApiError> {
    authorize_request(&state, &headers)?;

    Ok(Json(SessionListResponse {
        sessions: state.list(),
    }))
}

async fn get_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<usize>,
) -> Result<Json<SessionInfo>, ApiError> {
    authorize_request(&state, &headers)?;
    let session = state
        .get(SessionId(id))
        .ok_or_else(|| ApiError::not_found(format!("session {id} does not exist")))?;
    let info = lock_session(&session)?.info();

    Ok(Json(info))
}

async fn delete_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<usize>,
) -> Result<StatusCode, ApiError> {
    authorize_request(&state, &headers)?;

    let session = state
        .remove(SessionId(id))
        .ok_or_else(|| ApiError::not_found(format!("session {id} does not exist")))?;

    if let Ok(mut guard) = session.lock() {
        guard.stop();
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn execute_command(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<usize>,
    Json(request): Json<ExecuteCommandRequest>,
) -> Result<Json<ExecuteCommandResponse>, ApiError> {
    authorize_request(&state, &headers)?;

    if request.command.len() > MAX_COMMAND_BYTES {
        return Err(ApiError::bad_request(format!(
            "the command exceeds the {MAX_COMMAND_BYTES}-byte limit"
        )));
    }

    let session = state
        .get(SessionId(id))
        .ok_or_else(|| ApiError::not_found(format!("session {id} does not exist")))?;

    let response = tokio::task::spawn_blocking(move || {
        let mut guard = lock_session(&session)?;
        let overrides = guard.begin_overrides(&request)?;
        let mut response = guard.execute(&request.command);
        guard.end_overrides(overrides);
        response.session = guard.info();
        Ok::<_, ApiError>(response)
    })
    .await
    .map_err(|error| ApiError::internal(format!("command task failed: {error}")))??;

    Ok(Json(response))
}

fn lock_session(
    session: &Arc<Mutex<ShellSession>>,
) -> Result<std::sync::MutexGuard<'_, ShellSession>, ApiError> {
    session
        .lock()
        .map_err(|_| ApiError::internal("session lock was poisoned"))
}

async fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ApiErrorBody {
            code: "not_found".to_owned(),
            message: "the requested KERNEXT API route does not exist".to_owned(),
        }),
    )
        .into_response()
}
