use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("shell_server=info,tower_http=info"));

    tracing_subscriber::fmt().with_env_filter(filter).init();

    if let Err(error) = shell_server::run().await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
