use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

/// Loads KERNEXT configuration without depending on the launch directory.
///
/// Order:
/// 1. `KERNEXT_ENV_FILE` (explicit path; must exist).
/// 2. `.env` in the current directory (the documented `npm run dev` path).
/// 3. The workspace-root `.env` next to this crate (`../../.env`), so running
///    the binary or `cargo run -p shell-server` from another directory still
///    finds the development configuration.
///
/// Real environment variables always win; dotenvy only fills gaps.
fn load_environment() -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var("KERNEXT_ENV_FILE") {
        let path = PathBuf::from(&explicit);
        if !path.is_file() {
            eprintln!(
                "error: KERNEXT_ENV_FILE points to {explicit:?}, but that file does not exist"
            );
            std::process::exit(1);
        }
        match dotenvy::from_path(&path) {
            Ok(_) => return Some(path),
            Err(error) => {
                eprintln!("error: could not read KERNEXT_ENV_FILE {explicit:?}: {error}");
                std::process::exit(1);
            }
        }
    }

    let workspace_env = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.env");

    for candidate in [PathBuf::from(".env"), workspace_env] {
        if candidate.is_file() {
            match dotenvy::from_path(&candidate) {
                Ok(_) => return Some(candidate),
                Err(error) => {
                    eprintln!(
                        "error: could not read configuration file {:?}: {error}",
                        candidate
                    );
                    std::process::exit(1);
                }
            }
        }
    }

    None
}

#[tokio::main]
async fn main() {
    let env_file = load_environment();

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("kernext_server=info,shell_server=info,tower_http=info")
    });

    tracing_subscriber::fmt().with_env_filter(filter).init();

    match &env_file {
        Some(path) => tracing::info!("loaded environment from {}", path.display()),
        None => tracing::info!(
            "no .env file found; using process environment variables only \
             (set KERNEXT_ENV_FILE to choose a file)"
        ),
    }

    if let Err(error) = shell_server::run().await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
