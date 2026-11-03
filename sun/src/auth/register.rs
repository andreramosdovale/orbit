//! Auto-cadastro: validação, hash da senha (argon2id) e inserção no banco.

use argon2::Argon2;
use argon2::password_hash::PasswordHasher;
use orbit_shared::{RegisterRequest, User};
use sqlx::SqlitePool;

const MIN_USERNAME_LEN: usize = 3;
const MAX_USERNAME_LEN: usize = 32;
const MIN_PASSWORD_LEN: usize = 8;
const MAX_PASSWORD_LEN: usize = 128;

#[derive(Debug, thiserror::Error)]
pub enum RegisterError {
    #[error("nome de usuário inválido")]
    InvalidUsername,
    #[error("senha inválida")]
    InvalidPassword,
    #[error("nome de usuário já existe")]
    UsernameTaken,
    #[error("falha ao gerar o hash da senha: {0}")]
    Hash(argon2::password_hash::Error),
    #[error("a task de hash da senha falhou")]
    HashTask(#[source] tokio::task::JoinError),
    #[error("erro no banco")]
    Db(#[source] sqlx::Error),
    #[error("id de usuário fora do intervalo: {0}")]
    IdOutOfRange(i64),
}

/// Valida, gera o hash e grava o usuário. Retorna o usuário público.
pub async fn register_user(db: &SqlitePool, req: RegisterRequest) -> Result<User, RegisterError> {
    let username = normalize_username(&req.username)?;
    tracing::debug!(%username, "1/4 nome de usuário normalizado e válido");
    validate_password(&req.password)?;
    tracing::debug!("2/4 senha dentro das regras de tamanho");

    let password = req.password;
    let password_hash = tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(RegisterError::HashTask)??;
    tracing::debug!("3/4 hash argon2id gerado");

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO users (username, password_hash) VALUES (?, ?) RETURNING id",
    )
    .bind(&username)
    .bind(&password_hash)
    .fetch_one(db)
    .await
    .map_err(|err| match err.as_database_error() {
        Some(db_err) if db_err.is_unique_violation() => RegisterError::UsernameTaken,
        _ => RegisterError::Db(err),
    })?;

    tracing::debug!(id, "4/4 usuário inserido no banco");

    let id = u32::try_from(id).map_err(|_| RegisterError::IdOutOfRange(id))?;
    Ok(User { id, username })
}

/// Converte para minúsculas e valida: 3 a 32 caracteres `a-z`, `0-9` ou `_`.
fn normalize_username(raw: &str) -> Result<String, RegisterError> {
    let username = raw.to_lowercase();
    let len_ok = (MIN_USERNAME_LEN..=MAX_USERNAME_LEN).contains(&username.len());
    let chars_ok = username
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if len_ok && chars_ok {
        Ok(username)
    } else {
        Err(RegisterError::InvalidUsername)
    }
}

/// 8 a 128 caracteres (contando caracteres, não bytes).
fn validate_password(password: &str) -> Result<(), RegisterError> {
    let len = password.chars().count();
    if (MIN_PASSWORD_LEN..=MAX_PASSWORD_LEN).contains(&len) {
        Ok(())
    } else {
        Err(RegisterError::InvalidPassword)
    }
}

/// Hash argon2id com salt aleatório, no formato PHC (`$argon2id$v=19$...`).
/// CPU pesado: chame dentro de `spawn_blocking`.
pub(super) fn hash_password(password: &str) -> Result<String, RegisterError> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(RegisterError::Hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_rules() {
        assert_eq!(
            normalize_username("Fulano_42").ok().as_deref(),
            Some("fulano_42")
        );
        let too_long = "a".repeat(33);
        for bad in [
            "ab",
            too_long.as_str(),
            "com espaço",
            "acentuação",
            "a-b",
            "",
        ] {
            assert!(normalize_username(bad).is_err(), "deveria rejeitar {bad:?}");
        }
    }

    #[test]
    fn password_rules() {
        assert!(validate_password(&"a".repeat(8)).is_ok());
        assert!(validate_password(&"a".repeat(128)).is_ok());
        assert!(validate_password(&"a".repeat(7)).is_err());
        assert!(validate_password(&"a".repeat(129)).is_err());
        // 8 caracteres multibyte contam como 8, não como 16 bytes.
        assert!(validate_password("çççççççç").is_ok());
    }
}
