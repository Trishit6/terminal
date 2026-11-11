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

pub mod admin;
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

    // Resolve from both the current directory and the crate location so
    // launching the server from `apps/shell-server` still serves the built
    // front end instead of returning the API 404 for `/`.
    [
        std::path::PathBuf::from("apps/web/dist"),
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/web/dist"),
    ]
    .into_iter()
    .find(|candidate| candidate.is_dir())
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

    let production = std::env::var("KERNEXT_ENV")
        .map(|value| value.eq_ignore_ascii_case("production"))
        .unwrap_or(false);
    let database_url = std::env::var("DATABASE_URL")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());

    if database_url.is_none() && production {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "DATABASE_URL is required when KERNEXT_ENV=production; account \
             authentication cannot start without MariaDB",
        )
        .into());
    }

    if let Some(database_url) = database_url {
        if !database_url.starts_with("mysql://") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "DATABASE_URL must be a mysql:// connection URL \
                 (mysql://USER:PASSWORD@HOST:PORT/DATABASE)",
            )
            .into());
        }

        let database = db::Database::connect(&database_url).await?;
        database.migrate().await?;
        database.verify_schema().await?;
        seed_development_admin(&database).await;
        tracing::info!("KERNEXT accounts enabled (MariaDB)");
        state = state.with_accounts(state::AuthContext::new(
            database,
            db::AuthConfig::from_env(),
        ));
        admin::record_start(&state).await;
    } else {
        tracing::warn!(
            "DATABASE_URL is not set; account features are disabled and the server \
             runs in local mode. Set DATABASE_URL in .env or the process environment \
             to enable authentication."
        );
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

/// Creates the development administrator when explicitly enabled.
///
/// The seed never runs in production and never overwrites an existing account.
/// Credentials are read from the environment and are never logged; only the
/// outcome (created / already present) is recorded.
async fn seed_development_admin(database: &db::Database) {
    let production = std::env::var("KERNEXT_ENV")
        .map(|value| value.eq_ignore_ascii_case("production"))
        .unwrap_or(false);
    let enabled = std::env::var("KERNEXT_SEED_ADMIN")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if !seed_admin_allowed(production, enabled) {
        if enabled {
            tracing::warn!("KERNEXT_SEED_ADMIN is set but ignored because KERNEXT_ENV=production");
        }
        return;
    }

    let username = std::env::var("KERNEXT_ADMIN_USERNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "admin".to_owned());
    let email = std::env::var("KERNEXT_ADMIN_EMAIL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "admin@admin.com".to_owned());
    let password = std::env::var("KERNEXT_ADMIN_PASSWORD")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "password".to_owned());

    if password == "password" {
        tracing::warn!(
            "the development administrator seed is using the default development password; \
             change it on first sign-in"
        );
    }

    match database.seed_admin(&username, &email, &password).await {
        Ok(db::AdminSeed::Created) => {
            tracing::info!("development administrator account created");
        }
        Ok(db::AdminSeed::Existing) => {
            tracing::info!("development administrator already exists; left unchanged");
        }
        Err(error) => {
            tracing::warn!("development administrator seed failed: {error}");
        }
    }
}

/// The development administrator seed is opt-in and never allowed in
/// production, regardless of `KERNEXT_SEED_ADMIN`.
const fn seed_admin_allowed(production: bool, enabled: bool) -> bool {
    enabled && !production
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

#[cfg(test)]
mod tests {
    use super::seed_admin_allowed;

    #[test]
    fn default_admin_seed_never_runs_in_production() {
        assert!(!seed_admin_allowed(true, true));
        assert!(!seed_admin_allowed(true, false));
        assert!(!seed_admin_allowed(false, false));
        assert!(seed_admin_allowed(false, true));
    }
}
