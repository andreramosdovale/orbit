//! JWT HS256 do login (ADR 0008): segredo guardado no SQLite, token válido por 30 dias.

use std::time::Duration;

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use orbit_shared::User;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Validade do token.
pub const TOKEN_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Nome da linha em `secrets` que guarda a chave do JWT.
const SECRET_NAME: &str = "jwt";
/// Tamanho do segredo gerado (256 bits, o mesmo do HMAC-SHA256).
const SECRET_LEN: usize = 32;

/// Conteúdo do token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claims {
    /// Id do usuário, como string.
    pub sub: String,
    pub username: String,
    /// Emissão (Unix, segundos).
    pub iat: u64,
    /// Expiração (Unix, segundos).
    pub exp: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    #[error("falha ao gerar o token")]
    Encode(#[source] jsonwebtoken::errors::Error),
    #[error("token inválido")]
    Invalid(#[source] jsonwebtoken::errors::Error),
    #[error("token expirado")]
    Expired,
}

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("erro no banco ao carregar o segredo do JWT")]
    Db(#[source] sqlx::Error),
    #[error("não foi possível gerar bytes aleatórios: {0}")]
    Random(getrandom::Error),
    #[error("o segredo do JWT no banco tem {0} bytes; esperado {SECRET_LEN}")]
    BadLength(usize),
}

/// Chaves de assinatura e verificação, derivadas do segredo do banco.
#[derive(Clone)]
pub struct JwtKeys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl JwtKeys {
    /// Lê o segredo de `secrets`. Na primeira vez, gera um aleatório e grava.
    pub async fn load_or_create(db: &SqlitePool) -> Result<Self, SecretError> {
        let mut fresh = [0u8; SECRET_LEN];
        getrandom::fill(&mut fresh).map_err(SecretError::Random)?;

        // `DO NOTHING` mantém o segredo existente: só grava se ainda não houver um.
        let inserted = sqlx::query(
            "INSERT INTO secrets (name, value) VALUES (?, ?) ON CONFLICT(name) DO NOTHING",
        )
        .bind(SECRET_NAME)
        .bind(&fresh[..])
        .execute(db)
        .await
        .map_err(SecretError::Db)?
        .rows_affected();
        if inserted > 0 {
            tracing::info!("segredo do JWT gerado e salvo no banco");
        }

        let secret: Vec<u8> = sqlx::query_scalar("SELECT value FROM secrets WHERE name = ?")
            .bind(SECRET_NAME)
            .fetch_one(db)
            .await
            .map_err(SecretError::Db)?;
        if secret.len() != SECRET_LEN {
            return Err(SecretError::BadLength(secret.len()));
        }
        Ok(Self::from_secret(&secret))
    }

    fn from_secret(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }

    /// Emite um token para `user`, com `now` em segundos Unix.
    pub fn issue(&self, user: &User, now: u64) -> Result<String, JwtError> {
        let claims = Claims {
            sub: user.id.to_string(),
            username: user.username.clone(),
            iat: now,
            exp: now.saturating_add(TOKEN_TTL.as_secs()),
        };
        jsonwebtoken::encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(JwtError::Encode)
    }

    /// Confere assinatura, algoritmo e expiração (contra `now`, sem folga).
    pub fn verify(&self, token: &str, now: u64) -> Result<Claims, JwtError> {
        let mut validation = Validation::new(Algorithm::HS256);
        // A expiração é checada abaixo contra `now`, para os testes controlarem o relógio.
        validation.validate_exp = false;
        validation.set_required_spec_claims(&["exp", "sub"]);

        let claims = jsonwebtoken::decode::<Claims>(token, &self.decoding, &validation)
            .map_err(JwtError::Invalid)?
            .claims;
        if claims.exp <= now {
            return Err(JwtError::Expired);
        }
        Ok(claims)
    }
}

/// Agora, em segundos Unix.
pub fn now() -> u64 {
    jsonwebtoken::get_current_timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_800_000_000;

    fn keys(byte: u8) -> JwtKeys {
        JwtKeys::from_secret(&[byte; SECRET_LEN])
    }

    fn fulano() -> User {
        User {
            id: 7,
            username: "fulano".into(),
        }
    }

    #[test]
    fn issued_token_verifies_with_claims() {
        let keys = keys(1);
        let token = keys.issue(&fulano(), NOW).unwrap();

        let claims = keys.verify(&token, NOW).unwrap();

        assert_eq!(claims.sub, "7");
        assert_eq!(claims.username, "fulano");
        assert_eq!(claims.iat, NOW);
        assert_eq!(claims.exp, NOW + 30 * 24 * 60 * 60);
    }

    #[test]
    fn expired_token_is_rejected() {
        let keys = keys(1);
        let token = keys.issue(&fulano(), NOW).unwrap();
        let exp = NOW + TOKEN_TTL.as_secs();

        assert!(keys.verify(&token, exp - 1).is_ok());
        assert!(matches!(keys.verify(&token, exp), Err(JwtError::Expired)));
    }

    #[test]
    fn token_from_other_secret_is_rejected() {
        let token = keys(1).issue(&fulano(), NOW).unwrap();

        assert!(matches!(
            keys(2).verify(&token, NOW),
            Err(JwtError::Invalid(_))
        ));
    }

    #[test]
    fn tampered_token_is_rejected() {
        let keys = keys(1);
        let token = keys.issue(&fulano(), NOW).unwrap();

        // Troca o payload por outro (com `sub` diferente) mantendo a assinatura original.
        let other = keys
            .issue(
                &User {
                    id: 8,
                    username: "ciclano".into(),
                },
                NOW,
            )
            .unwrap();
        let mut parts: Vec<&str> = token.split('.').collect();
        let other_payload = other.split('.').nth(1).unwrap();
        parts[1] = other_payload;
        let tampered = parts.join(".");

        assert!(matches!(
            keys.verify(&tampered, NOW),
            Err(JwtError::Invalid(_))
        ));
        assert!(keys.verify("nao.e.jwt", NOW).is_err());
    }
}
