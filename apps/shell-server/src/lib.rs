//! KERNEXT Axum backend.
//!
//! The server owns real `shell_runtime::Session` instances and routes every
//! command through `shell_builtins::Dispatcher`. The browser never simulates
//! terminal output: it sends command lines over a WebSocket (or REST) and the
//! Rust engine executes them.
//!
//! Two deployment modes exist:
//!
//! - **Account mode** (`DATABASE_URL` set): MariaDB-backed registration,
//!   Argon2id passwords, HttpOnly session cookies, and CSRF protection. Every
//!   terminal session is bound to the signed-in account.
//! - **Local mode** (`DATABASE_URL` unset): the original loopback-only server
//!   with an optional `KERNEXT_SERVER_TOKEN` bearer token. This mode is for
//!   local development and desktop-adjacent workflows; it is not a multi-user
//!   authorization system.

pub mod auth;
pub mod auth_routes;
pub mod db;
pub mod error;
pub mod routes;
pub mod spa;
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
/// be created, the configured account database cannot be reached or migrated,
/// or the Axum server fails.
pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bind = std::env::var("KERNEXT_SERVER_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_owned());

    let mut state = AppState::from_env();

    if let Ok(database_url) = std::env::var("DATABASE_URL")
        && !database_url.trim().is_empty()
    {
        let database = db::Database::connect(&database_url).await?;
        database.migrate().await?;
        tracing::info!("KERNEXT accounts enabled (MariaDB)");
        state = state.with_accounts(state::AuthContext::new(
            database,
            db::AuthConfig::from_env(),
        ));
    }

    if state.token().is_none() && !state.requires_accounts() {
        tracing::warn!(
            "neither DATABASE_URL nor KERNEXT_SERVER_TOKEN is set; the server trusts any client \
             that can reach {bind}"
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

            if let Some(accounts) = cleanup_state.accounts()
                && let Err(error) = accounts.db.purge_sessions().await
            {
                tracing::warn!("KERNEXT session purge failed: {error}");
            }
        }
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("KERNEXT server stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if tokio::signal::ctrl_c().await.is_err() {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}
