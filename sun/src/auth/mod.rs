//! Contas de usuário: cadastro, login e JWT.

mod jwt;
mod login;
mod register;

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use orbit_shared::{LoginRequest, LoginResponse, RegisterRequest, User};

use crate::AppState;
use crate::error::ApiError;

pub use jwt::{Claims, JwtError, JwtKeys, SecretError, now};
use login::LoginError;
use register::RegisterError;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register_handler))
        .route("/auth/login", post(login_handler))
}

#[tracing::instrument(skip_all)]
async fn register_handler(
    State(state): State<AppState>,
    payload: Result<Json<RegisterRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<User>), ApiError> {
    tracing::debug!("POST /auth/register recebido");
    let Json(req) = payload.map_err(|rejection| {
        tracing::debug!(motivo = %rejection.body_text(), "corpo da requisição rejeitado");
        ApiError::InvalidRequest(rejection.body_text())
    })?;
    tracing::debug!(username = %req.username, "JSON lido; iniciando cadastro");
    let user = register::register_user(&state.db, req)
        .await
        .inspect_err(|err| {
            tracing::debug!(error = %err, "cadastro recusado");
        })?;
    tracing::info!(user_id = user.id, username = %user.username, "usuário cadastrado");
    Ok((StatusCode::CREATED, Json(user)))
}

#[tracing::instrument(skip_all)]
async fn login_handler(
    State(state): State<AppState>,
    payload: Result<Json<LoginRequest>, JsonRejection>,
) -> Result<Json<LoginResponse>, ApiError> {
    tracing::debug!("POST /auth/login recebido");
    let Json(req) = payload.map_err(|rejection| {
        tracing::debug!(motivo = %rejection.body_text(), "corpo da requisição rejeitado");
        ApiError::InvalidRequest(rejection.body_text())
    })?;
    let res = login::login(&state.db, &state.jwt, req)
        .await
        .inspect_err(|err| {
            tracing::debug!(error = %err, "login recusado");
        })?;
    tracing::info!(user_id = res.user.id, username = %res.user.username, "usuário logou");
    Ok(Json(res))
}

impl From<LoginError> for ApiError {
    fn from(err: LoginError) -> Self {
        match err {
            LoginError::InvalidCredentials => Self::InvalidCredentials,
            other @ (LoginError::HashTask(_)
            | LoginError::CorruptHash(_)
            | LoginError::Db(_)
            | LoginError::IdOutOfRange(_)
            | LoginError::Token(_)) => Self::internal(other),
        }
    }
}

impl From<RegisterError> for ApiError {
    fn from(err: RegisterError) -> Self {
        match err {
            RegisterError::InvalidUsername => Self::InvalidUsername,
            RegisterError::InvalidPassword => Self::InvalidPassword,
            RegisterError::UsernameTaken => Self::UsernameTaken,
            other @ (RegisterError::Hash(_)
            | RegisterError::HashTask(_)
            | RegisterError::Db(_)
            | RegisterError::IdOutOfRange(_)) => Self::internal(other),
        }
    }
}
