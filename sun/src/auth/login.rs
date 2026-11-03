//! Login: confere usuário e senha e emite o JWT (spec 002).

use argon2::Argon2;
use argon2::password_hash::PasswordVerifier;
use argon2::password_hash::phc::PasswordHash;
use orbit_shared::{LoginRequest, LoginResponse, User};
use sqlx::SqlitePool;

use super::jwt::{self, JwtError, JwtKeys};
use super::register;

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("usuário ou senha incorretos")]
    InvalidCredentials,
    #[error("a task de verificação da senha falhou")]
    HashTask(#[source] tokio::task::JoinError),
    #[error("hash de senha inválido no banco: {0}")]
    CorruptHash(argon2::password_hash::Error),
    #[error("erro no banco")]
    Db(#[source] sqlx::Error),
    #[error("id de usuário fora do intervalo: {0}")]
    IdOutOfRange(i64),
    #[error("falha ao emitir o token")]
    Token(#[source] JwtError),
}

/// Confere as credenciais e devolve o token + usuário.
///
/// Usuário inexistente e senha errada dão o mesmo erro e custam o mesmo argon2,
/// para não revelar quais usuários existem.
pub async fn login(
    db: &SqlitePool,
    keys: &JwtKeys,
    req: LoginRequest,
) -> Result<LoginResponse, LoginError> {
    let username = req.username.to_lowercase();
    let row: Option<(i64, String)> =
        sqlx::query_as("SELECT id, password_hash FROM users WHERE username = ?")
            .bind(&username)
            .fetch_optional(db)
            .await
            .map_err(LoginError::Db)?;
    tracing::debug!(%username, encontrado = row.is_some(), "1/3 busca do usuário");

    let password = req.password;
    let Some((id, password_hash)) = row else {
        // Só o custo do argon2 importa aqui (igualar o tempo de resposta), então o
        // hash gerado, ou um erro ao gerá-lo, é descartado de propósito.
        tokio::task::spawn_blocking(move || drop(register::hash_password(&password)))
            .await
            .map_err(LoginError::HashTask)?;
        return Err(LoginError::InvalidCredentials);
    };

    let matches = tokio::task::spawn_blocking(move || verify_password(&password, &password_hash))
        .await
        .map_err(LoginError::HashTask)??;
    if !matches {
        return Err(LoginError::InvalidCredentials);
    }
    tracing::debug!("2/3 senha confere");

    let id = u32::try_from(id).map_err(|_| LoginError::IdOutOfRange(id))?;
    let user = User { id, username };
    let token = keys.issue(&user, jwt::now()).map_err(LoginError::Token)?;
    tracing::debug!("3/3 token emitido");

    Ok(LoginResponse { token, user })
}

/// `Ok(false)` para senha errada; `Err` só se o hash salvo estiver corrompido.
/// CPU pesado: chame dentro de `spawn_blocking`.
fn verify_password(password: &str, password_hash: &str) -> Result<bool, LoginError> {
    let parsed =
        PasswordHash::new(password_hash).map_err(|err| LoginError::CorruptHash(err.into()))?;
    match Argon2::default().verify_password(password.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::PasswordInvalid) => Ok(false),
        Err(err) => Err(LoginError::CorruptHash(err)),
    }
}
