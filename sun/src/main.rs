use std::env::{self, VarError};
use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, bail};
use tracing_subscriber::EnvFilter;

const DEFAULT_ADDR: &str = "0.0.0.0:7700";
/// Filtro padrão quando `RUST_LOG` não está definido: em dev (`cargo run`) mostra
/// os `debug!` do `sun`; no build de release fica só em `info`.
const DEFAULT_LOG_FILTER: &str = if cfg!(debug_assertions) {
    "info,sun=debug"
} else {
    "info"
};
const DEFAULT_DB_FILE: &str = "orbit.db";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing()?;

    let db_path = db_path()?;
    let db = sun::db::connect(&db_path)
        .await
        .with_context(|| format!("abrindo o banco em {}", db_path.display()))?;
    tracing::info!(path = %db_path.display(), "banco aberto");
    let jwt = sun::JwtKeys::load_or_create(&db)
        .await
        .context("carregando o segredo do JWT")?;
    let state = sun::AppState {
        db,
        jwt,
        ws_auth_timeout: sun::WS_AUTH_TIMEOUT,
    };

    let addr = listen_addr()?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("não foi possível abrir {addr}"))?;

    let local_addr = listener.local_addr().context("lendo o endereço local")?;
    tracing::info!(addr = %local_addr, "sun ouvindo");

    sun::serve(listener, state, shutdown_signal())
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

/// Configura o `tracing` a partir de `RUST_LOG` (padrão: [`DEFAULT_LOG_FILTER`]). Filtro inválido é erro.
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

/// Caminho do banco a partir de `SUN_DB` (padrão: `orbit.db` ao lado do executável).
fn db_path() -> anyhow::Result<PathBuf> {
    if let Some(value) = optional_env("SUN_DB")? {
        return Ok(PathBuf::from(value));
    }
    let exe = env::current_exe().context("descobrindo o caminho do executável")?;
    let dir = exe
        .parent()
        .context("o executável não está dentro de uma pasta")?;
    Ok(dir.join(DEFAULT_DB_FILE))
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
