//! REST routes for the KERNEXT server.

use crate::auth::{allowed_origins, authorize_request};
use crate::auth_routes;
use crate::db::AuthUser;
use crate::error::ApiError;
use crate::spa;
use crate::state::{AppState, MAX_COMMAND_BYTES, ShellSession};
use crate::ws;
use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
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
/// When it also contains the route manifest (`routes.json`, written by
/// `scripts/seo.mjs`), the SPA shell is served through [`crate::spa`] with
/// per-route head patching and a real HTTP 404 for unknown paths. Without the
/// manifest, every unmatched path still receives the shell with status 200 so
/// client-side routes keep working. API routes always take precedence over
/// the static fallback.
pub fn router_with_static(state: AppState, web_dir: Option<PathBuf>) -> Router {
    let app = Router::new()
        .merge(auth_routes::router())
        .merge(crate::admin::router())
        .route("/api/v1/health", get(health))
        .route("/api/v1/health/database", get(health_database))
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
            match spa::SpaShell::load(&directory) {
                Some(shell) => {
                    let service = spa::static_service(&directory, std::sync::Arc::new(shell));
                    app.fallback_service(service)
                }
                None => {
                    // No route manifest: keep serving the SPA shell for any
                    // path so client-side routes still work after a refresh.
                    let index = directory.join("index.html");
                    let service = ServeDir::new(directory).fallback(ServeFile::new(index));
                    app.fallback_service(service)
                }
            }
        }
        _ => app.fallback(api_not_found),
    };

    app.layer(middleware::from_fn(security_headers))
        .layer(cors_layer())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn cors_layer() -> CorsLayer {
    let origins = allowed_origins()
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::COOKIE,
            header::ACCEPT,
            header::HeaderName::from_static("x-csrf-token"),
        ])
}

/// Adds defensive response headers to every response, including the SPA and
/// the JSON API. HSTS is only emitted in production so local HTTP development
/// is not pinned to HTTPS.
async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    for (name, value) in [
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::X_FRAME_OPTIONS, "DENY"),
        (header::REFERRER_POLICY, "no-referrer"),
        (
            header::HeaderName::from_static("permissions-policy"),
            "geolocation=(), microphone=(), camera=()",
        ),
        (
            header::CONTENT_SECURITY_POLICY,
            "default-src 'self'; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; \
             script-src 'self'; connect-src 'self' ws: wss:; font-src 'self' data:; \
             frame-ancestors 'none'; base-uri 'self'; form-action 'self'",
        ),
    ] {
        headers.insert(name, HeaderValue::from_static(value));
    }

    if production() {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }

    response
}

fn production() -> bool {
    static PRODUCTION: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *PRODUCTION.get_or_init(|| {
        std::env::var("KERNEXT_ENV")
            .map(|value| value.eq_ignore_ascii_case("production"))
            .unwrap_or(false)
    })
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

/// Reports account-database connectivity. Terminal and local mode stay usable
/// when the database is unavailable; this endpoint makes the failure explicit.
async fn health_database(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Some(accounts) = state.accounts() else {
        return Err(ApiError::unavailable(
            "the KERNEXT account database is not configured. Set DATABASE_URL to enable accounts.",
        ));
    };

    let started = std::time::Instant::now();
    sqlx::query("SELECT 1")
        .execute(accounts.db.pool())
        .await
        .map_err(|error| {
            ApiError::unavailable(format!(
                "the KERNEXT account database is not reachable: {error}"
            ))
        })?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "database": "mariadb",
        "latency_ms": started.elapsed().as_millis() as u64,
    })))
}

async fn create_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, ApiError> {
    let user = authorize_terminal(&state, &headers).await?;
    let session = state.create_owned(&request, user.as_ref().map(|user| user.user.id.as_str()))?;
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
    let user = authorize_terminal(&state, &headers).await?;

    let sessions = match user {
        Some(user) => state.list_owned(&user.user.id),
        None => state.list(),
    };

    Ok(Json(SessionListResponse { sessions }))
}

async fn get_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<usize>,
) -> Result<Json<SessionInfo>, ApiError> {
    let user = authorize_terminal(&state, &headers).await?;
    let session = state
        .get(SessionId(id))
        .ok_or_else(|| ApiError::not_found(format!("session {id} does not exist")))?;
    let guard = lock_session(&session)?;
    authorize_owned(&guard, user.as_ref())?;
    let info = guard.info();

    Ok(Json(info))
}

async fn delete_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<usize>,
) -> Result<StatusCode, ApiError> {
    let user = authorize_terminal(&state, &headers).await?;

    let session = state
        .get(SessionId(id))
        .ok_or_else(|| ApiError::not_found(format!("session {id} does not exist")))?;
    {
        let guard = lock_session(&session)?;
        authorize_owned(&guard, user.as_ref())?;
    }
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
    let user = authorize_terminal(&state, &headers).await?;

    if request.command.len() > MAX_COMMAND_BYTES {
        return Err(ApiError::bad_request(format!(
            "the command exceeds the {MAX_COMMAND_BYTES}-byte limit"
        )));
    }

    let session = state
        .get(SessionId(id))
        .ok_or_else(|| ApiError::not_found(format!("session {id} does not exist")))?;
    {
        let guard = lock_session(&session)?;
        authorize_owned(&guard, user.as_ref())?;
    }

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

/// Enforces terminal authorization for one request.
///
/// With `DATABASE_URL` configured, a valid account session cookie is required
/// and the returned user owns every terminal resource. Without it, the server
/// keeps its documented local-only behavior.
async fn authorize_terminal(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<AuthUser>, ApiError> {
    if state.requires_accounts() {
        let user = auth_routes::require_account_active(state, headers).await?;
        Ok(Some(user))
    } else {
        authorize_request(state, headers)?;
        Ok(None)
    }
}

/// Rejects access to sessions owned by another account.
fn authorize_owned(session: &ShellSession, user: Option<&AuthUser>) -> Result<(), ApiError> {
    match user {
        Some(user) if !session.is_owned_by(&user.user.id) => {
            Err(ApiError::not_found("that terminal session does not exist"))
        }
        Some(_) if session.owner().is_none() => {
            Err(ApiError::not_found("that terminal session does not exist"))
        }
        _ => Ok(()),
    }
}

fn lock_session(
    session: &Arc<Mutex<ShellSession>>,
) -> Result<std::sync::MutexGuard<'_, ShellSession>, ApiError> {
    session
        .lock()
        .map_err(|_| ApiError::internal("session lock was poisoned"))
}

/// JSON 404 for unmatched API paths; shared with the SPA shell fallback so
/// unknown `/api/*` requests never receive an HTML body.
pub(crate) async fn api_not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ApiErrorBody {
            code: "not_found".to_owned(),
            message: "the requested KERNEXT API route does not exist".to_owned(),
        }),
    )
        .into_response()
}
