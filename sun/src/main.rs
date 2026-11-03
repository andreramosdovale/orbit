use std::env::{self, VarError};
use std::net::SocketAddr;

use anyhow::{Context, bail};
use tracing_subscriber::EnvFilter;

const DEFAULT_ADDR: &str = "0.0.0.0:7700";
const DEFAULT_LOG_FILTER: &str = "info";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing()?;

    let addr = listen_addr()?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("não foi possível abrir {addr}"))?;

    let local_addr = listener.local_addr().context("lendo o endereço local")?;
    tracing::info!(addr = %local_addr, "sun ouvindo");

    sun::serve(listener, shutdown_signal())
        .await
        .context("servidor HTTP encerrou com erro")?;

    tracing::info!("sun encerrado");
    Ok(())
}

/// Lê uma variável de ambiente opcional. Ausente → `None`; presente mas não-Unicode → erro.
fn optional_env(name: &str) -> anyhow::Result<Option<String>> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => bail!("{name} contém caracteres inválidos"),
    }
}

/// Configura o `tracing` a partir de `RUST_LOG` (padrão `info`). Filtro inválido é erro.
fn init_tracing() -> anyhow::Result<()> {
    let filter = match optional_env("RUST_LOG")? {
        Some(value) => {
            EnvFilter::try_new(&value).with_context(|| format!("RUST_LOG inválido: {value:?}"))?
        }
        None => EnvFilter::new(DEFAULT_LOG_FILTER),
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init()
        .map_err(|err| anyhow::anyhow!(err))
        .context("inicializando o tracing")
}

/// Endereço de escuta a partir de `SUN_ADDR` (padrão `0.0.0.0:7700`). Valor inválido é erro.
fn listen_addr() -> anyhow::Result<SocketAddr> {
    let value = optional_env("SUN_ADDR")?.unwrap_or_else(|| DEFAULT_ADDR.to_owned());
    value
        .parse()
        .with_context(|| format!("SUN_ADDR inválido: {value:?} (esperado ip:porta)"))
}

/// Completa no Ctrl+C. Se não der para ouvir o sinal, loga e segue rodando até o processo morrer.
async fn shutdown_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => tracing::info!("Ctrl+C recebido; encerrando"),
        Err(err) => {
            tracing::error!(error = %err, "não foi possível ouvir o Ctrl+C; encerramento gracioso desativado");
            std::future::pending::<()>().await;
        }
    }
}
