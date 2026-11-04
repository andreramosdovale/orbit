//! Chat: canais de texto.

mod channels;

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use orbit_shared::{CreateChannelRequest, TextChannel};

use crate::AppState;
use crate::auth::AuthUser;
use crate::error::ApiError;

use channels::ChannelError;

pub fn routes() -> Router<AppState> {
    Router::new().route("/channels", get(list_handler).post(create_handler))
}

#[tracing::instrument(skip_all)]
async fn list_handler(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<TextChannel>>, ApiError> {
    let channels = channels::list_channels(&state.db).await?;
    Ok(Json(channels))
}

#[tracing::instrument(skip_all)]
async fn create_handler(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    payload: Result<Json<CreateChannelRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<TextChannel>), ApiError> {
    let Json(req) = payload.map_err(|rejection| {
        tracing::debug!(motivo = %rejection.body_text(), "corpo da requisição rejeitado");
        ApiError::InvalidRequest(rejection.body_text())
    })?;
    let channel = channels::create_channel(&state.db, &req.name)
        .await
        .inspect_err(|err| {
            tracing::debug!(error = %err, "criação de canal recusada");
        })?;
    tracing::info!(
        channel_id = channel.id,
        name = %channel.name,
        user_id = user.id,
        "canal criado"
    );
    Ok((StatusCode::CREATED, Json(channel)))
}

impl From<ChannelError> for ApiError {
    fn from(err: ChannelError) -> Self {
        match err {
            ChannelError::InvalidName => Self::InvalidChannelName,
            ChannelError::NameTaken => Self::ChannelNameTaken,
            other @ (ChannelError::Db(_) | ChannelError::IdOutOfRange(_)) => Self::internal(other),
        }
    }
}
