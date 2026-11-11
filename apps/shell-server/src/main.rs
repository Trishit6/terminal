use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // `.env` next to the workspace root is the documented configuration path
    // (see `.env.example`). Real environment variables always win; the file
    // only fills in variables that were not already set.
    let _ = dotenvy::dotenv();

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("shell_server=info,tower_http=info"));

    tracing_subscriber::fmt().with_env_filter(filter).init();

    if let Err(error) = shell_server::run().await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
