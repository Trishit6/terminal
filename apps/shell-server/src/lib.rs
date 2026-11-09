//! KERNEXT Axum backend.
//!
//! The server owns real `shell_runtime::Session` instances and routes every
//! command through `shell_builtins::Dispatcher`. The browser never simulates
//! terminal output: it sends command lines over a WebSocket (or REST) and the
//! Rust engine executes them.
//!
//! Defaults are deliberately local-only:
//! - bind address `127.0.0.1:7878`
//! - no public authentication layer beyond an optional bearer token
//! - a fixed set of browser origins accepted for CORS and WebSocket upgrades
//!
//! A public deployment must add authentication, sandboxing, and resource limits
//! at the reverse-proxy or container boundary. See `docs/SERVER.md`.

pub mod auth;
pub mod error;
pub mod routes;
pub mod state;
pub mod ws;

pub use error::ApiError;
pub use routes::{router, router_with_static};
pub use state::AppState;

/// Default loopback bind address.
pub const DEFAULT_BIND: &str = "127.0.0.1:7878";

fn web_directory() -> Option<std::path::PathBuf> {
    if let Some(value) = std::env::var_os("KERNEXT_WEB_DIR") {
        return Some(std::path::PathBuf::from(value));
    }

    let candidate = std::path::PathBuf::from("apps/web/dist");
    candidate.is_dir().then_some(candidate)
}

/// Runs the HTTP and WebSocket server until the process is interrupted.
///
/// # Errors
///
/// Returns an error when the bind address cannot be parsed, the listener cannot
/// be created, or the Axum server fails.
pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bind = std::env::var("KERNEXT_SERVER_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_owned());
    let state = AppState::from_env();

    if state.token().is_none() {
        tracing::warn!(
            "KERNEXT_SERVER_TOKEN is not set; the server trusts any client that can reach {bind}"
        );
    }

    let app = router_with_static(state.clone(), web_directory());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!("KERNEXT server listening on http://{bind}");

    let cleanup_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

        loop {
            interval.tick().await;
            cleanup_state.cleanup_idle(std::time::Duration::from_secs(30 * 60));
        }
    });

    axum::serve(listener, app).await?;

    Ok(())
}
