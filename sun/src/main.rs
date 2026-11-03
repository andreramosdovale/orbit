use anyhow::Context;
use tracing_subscriber::EnvFilter;

const DEFAULT_ADDR: &str = "0.0.0.0:7700";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let addr = std::env::var("SUN_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.into());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("não foi possível abrir {addr}"))?;

    tracing::info!("sun ouvindo em {}", listener.local_addr()?);
    axum::serve(listener, sun::app()).await?;
    Ok(())
}
