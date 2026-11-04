//! Autenticação das rotas REST: uma rota fica protegida ao receber [`AuthUser`].

use axum::extract::FromRequestParts;
use axum::http::header;
use axum::http::request::Parts;
use orbit_shared::User;

use super::jwt::now;
use crate::AppState;
use crate::error::ApiError;

const BEARER_PREFIX: &str = "Bearer ";

/// O usuário dono do token em `Authorization: Bearer <token>`. Qualquer problema
/// com o token vira `401 unauthorized`; o motivo só vai para o log.
pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let Some(token) = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix(BEARER_PREFIX))
        else {
            tracing::debug!("requisição sem token Bearer");
            return Err(ApiError::Unauthorized);
        };

        let claims = state.jwt.verify(token, now()).map_err(|err| {
            tracing::debug!(error = %err, "token recusado");
            ApiError::Unauthorized
        })?;
        let user = claims.into_user().map_err(|err| {
            tracing::warn!(error = %err, "token assinado com `sub` que não é um id");
            ApiError::Unauthorized
        })?;
        Ok(Self(user))
    }
}
