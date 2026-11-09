//! Bearer-token authentication for mutating endpoints.
//!
//! The server binds to localhost by default. When `KERNEXT_SERVER_TOKEN` is set,
//! every session and terminal endpoint requires `Authorization: Bearer <token>`.
//! This is a local development control, not a multi-user authorization system.

use crate::error::ApiError;
use crate::state::AppState;
use axum::http::{HeaderMap, header};

/// Browser origins allowed to talk to the local server.
///
/// The Vite development server, the bundled server origin, and Tauri's custom
/// origins are the only browser origins accepted. Requests without an `Origin`
/// header (curl, the desktop Rust side, integration tests) are allowed.
pub const ALLOWED_ORIGINS: &[&str] = &[
    "http://localhost:5173",
    "http://127.0.0.1:5173",
    "http://localhost:4173",
    "http://127.0.0.1:4173",
    "http://localhost:7878",
    "http://127.0.0.1:7878",
    "tauri://localhost",
    "http://tauri.localhost",
    "https://tauri.localhost",
];

/// Validates a browser `Origin` header when one is present.
pub fn authorize_origin(headers: &HeaderMap) -> Result<(), ApiError> {
    let Some(origin) = headers.get(header::ORIGIN) else {
        return Ok(());
    };

    let origin = origin
        .to_str()
        .map_err(|_| ApiError::unauthorized("the Origin header is not valid UTF-8"))?;

    if ALLOWED_ORIGINS.contains(&origin) {
        Ok(())
    } else {
        Err(ApiError::unauthorized(format!(
            "origin {origin:?} is not allowed to control this KERNEXT server"
        )))
    }
}

/// Validates origin and bearer-token requirements.
pub fn authorize_request(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    authorize_origin(headers)?;
    authorize(state, headers)
}

/// Validates a WebSocket upgrade.
///
/// Browsers cannot attach an `Authorization` header to `WebSocket`, so the
/// token may also arrive as a `?token=` query parameter. The header still wins
/// when both are present, and the comparison stays constant-time.
pub fn authorize_ws(
    state: &AppState,
    headers: &HeaderMap,
    query_token: Option<&str>,
) -> Result<(), ApiError> {
    authorize_origin(headers)?;

    if state.token().is_none() {
        return Ok(());
    }

    let header_token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    authorize_token(state, header_token.or(query_token))
}

/// Validates the request against the configured token.
pub fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let provided = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    authorize_token(state, provided)
}

fn authorize_token(state: &AppState, provided: Option<&str>) -> Result<(), ApiError> {
    let Some(expected) = state.token() else {
        return Ok(());
    };

    match provided {
        Some(candidate) if constant_time_eq(candidate.as_bytes(), expected.as_bytes()) => Ok(()),
        Some(_) => Err(ApiError::unauthorized("the bearer token is not valid")),
        None => Err(ApiError::unauthorized(
            "this server requires an Authorization: Bearer token",
        )),
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut difference = 0u8;
    for (left, right) in left.iter().zip(right) {
        difference |= left ^ right;
    }

    difference == 0
}
