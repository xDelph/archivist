use ingest::{IngestConfig, build_router};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing("ingest");

    let config = IngestConfig::from_env();
    let listener = TcpListener::bind(config.bind_address()).await?;
    let router = build_router(config)?;

    tracing::info!("ingest listening on {}", listener.local_addr()?);
    axum::serve(listener, router).await?;

    Ok(())
}

fn init_tracing(service_name: &str) {
    let env_filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| format!("{service_name}=debug,tower_http=info"));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .try_init();
}
