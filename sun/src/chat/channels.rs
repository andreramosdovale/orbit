//! Canais de texto: validação do nome, criação e listagem.

use orbit_shared::TextChannel;
use sqlx::SqlitePool;

/// Tamanho máximo do nome, em caracteres.
const MAX_NAME_CHARS: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("nome de canal inválido")]
    InvalidName,
    #[error("nome de canal já existe")]
    NameTaken,
    #[error("erro no banco")]
    Db(#[source] sqlx::Error),
    #[error("id de canal fora do intervalo: {0}")]
    IdOutOfRange(i64),
}

/// Remove espaços nas pontas e converte para minúsculas. `None` se o resultado
/// não tiver de 1 a 32 letras (com acento), números, `-` ou `_`.
fn normalize_name(name: &str) -> Option<String> {
    let name = name.trim().to_lowercase();
    let valid_chars = name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_');
    let valid_len = !name.is_empty() && name.chars().count() <= MAX_NAME_CHARS;
    (valid_chars && valid_len).then_some(name)
}

fn to_channel(id: i64, name: String) -> Result<TextChannel, ChannelError> {
    let id = u32::try_from(id).map_err(|_| ChannelError::IdOutOfRange(id))?;
    Ok(TextChannel { id, name })
}

/// Todos os canais, na ordem de criação.
pub async fn list_channels(db: &SqlitePool) -> Result<Vec<TextChannel>, ChannelError> {
    let rows: Vec<(i64, String)> = sqlx::query_as("SELECT id, name FROM channels ORDER BY id")
        .fetch_all(db)
        .await
        .map_err(ChannelError::Db)?;
    rows.into_iter()
        .map(|(id, name)| to_channel(id, name))
        .collect()
}

pub async fn create_channel(db: &SqlitePool, name: &str) -> Result<TextChannel, ChannelError> {
    let name = normalize_name(name).ok_or(ChannelError::InvalidName)?;
    let id: i64 = sqlx::query_scalar("INSERT INTO channels (name) VALUES (?) RETURNING id")
        .bind(&name)
        .fetch_one(db)
        .await
        .map_err(|err| match err.as_database_error() {
            Some(db_err) if db_err.is_unique_violation() => ChannelError::NameTaken,
            _ => ChannelError::Db(err),
        })?;
    to_channel(id, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_rules() {
        assert_eq!(normalize_name("geral").as_deref(), Some("geral"));
        assert_eq!(normalize_name("  Música ").as_deref(), Some("música"));
        assert_eq!(normalize_name("dev-2_a").as_deref(), Some("dev-2_a"));
        assert_eq!(normalize_name("a").as_deref(), Some("a"));

        let longest = "á".repeat(MAX_NAME_CHARS);
        assert_eq!(normalize_name(&longest), Some(longest.clone()));

        assert_eq!(normalize_name(""), None);
        assert_eq!(normalize_name("   "), None);
        assert_eq!(normalize_name(&"a".repeat(MAX_NAME_CHARS + 1)), None);
        assert_eq!(normalize_name("dois nomes"), None);
        assert_eq!(normalize_name("sim!"), None);
    }
}
