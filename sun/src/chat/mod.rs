//! Chat: canais e mensagens de texto.

pub mod channels;
pub mod messages;

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use orbit_shared::{CreateChannelRequest, MessagePage, ServerMessage, TextChannel};
use serde::Deserialize;

use crate::AppState;
use crate::auth::AuthUser;
use crate::error::ApiError;

use channels::ChannelError;
use messages::MessageError;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/channels", get(list_handler).post(create_handler))
        .route("/channels/{id}/messages", get(history_handler))
}

#[derive(Deserialize)]
struct HistoryQuery {
    before: Option<u32>,
}

#[tracing::instrument(skip_all)]
async fn history_handler(
    _user: AuthUser,
    State(state): State<AppState>,
    channel_id: Result<Path<u32>, PathRejection>,
    query: Result<Query<HistoryQuery>, QueryRejection>,
) -> Result<Json<MessagePage>, ApiError> {
    let Path(channel_id) =
        channel_id.map_err(|rejection| ApiError::InvalidRequest(rejection.body_text()))?;
    let Query(query) =
        query.map_err(|rejection| ApiError::InvalidRequest(rejection.body_text()))?;
    let page = messages::message_page(&state.db, channel_id, query.before).await?;
    Ok(Json(page))
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
    state.publish(ServerMessage::ChannelCreated {
        channel: channel.clone(),
    });
    Ok((StatusCode::CREATED, Json(channel)))
}

impl From<MessageError> for ApiError {
    fn from(err: MessageError) -> Self {
        match err {
            MessageError::UnknownChannel => Self::UnknownChannel,
            other => Self::internal(other),
        }
    }
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
