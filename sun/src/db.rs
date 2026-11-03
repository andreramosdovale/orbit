//! Conexão com o SQLite e migrations (`sun/migrations/`).

use std::path::Path;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("não foi possível abrir o banco")]
    Connect(#[source] sqlx::Error),
    #[error("falha ao aplicar as migrations")]
    Migrate(#[source] sqlx::migrate::MigrateError),
}

/// Abre (ou cria) o banco no arquivo `path` e aplica as migrations pendentes.
pub async fn connect(path: &Path) -> Result<SqlitePool, DbError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .map_err(DbError::Connect)?;
    migrate(&pool).await?;
    Ok(pool)
}

/// Banco só em memória, já migrado. Para testes.
///
/// Cada conexão SQLite em memória é um banco separado, então o pool usa uma
/// única conexão que nunca expira.
pub async fn connect_in_memory() -> Result<SqlitePool, DbError> {
    let options = SqliteConnectOptions::new()
        .in_memory(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .idle_timeout(None)
        .max_lifetime(None)
        .connect_with(options)
        .await
        .map_err(DbError::Connect)?;
    migrate(&pool).await?;
    Ok(pool)
}

async fn migrate(pool: &SqlitePool) -> Result<(), DbError> {
    sqlx::migrate!().run(pool).await.map_err(DbError::Migrate)
}
