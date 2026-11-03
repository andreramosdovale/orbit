//! Contas de usuário: cadastro (e, numa próxima spec, login).

mod register;

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use orbit_shared::{RegisterRequest, User};

use crate::AppState;
use crate::error::ApiError;

use register::RegisterError;

pub fn routes() -> Router<AppState> {
    Router::new().route("/auth/register", post(register_handler))
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
