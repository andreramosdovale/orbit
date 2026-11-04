//! Voz: salas e emissão de tokens do LiveKit. O áudio não passa pelo `sun`.

mod channels;
mod token;

use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use orbit_shared::{CreateVoiceChannelRequest, ServerMessage, VoiceChannel, VoiceJoin};

use crate::AppState;
use crate::auth::{self, AuthUser};
use crate::error::ApiError;

pub use token::{LiveKitConfig, LiveKitConfigError};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/voice-channels", get(list_handler).post(create_handler))
        .route("/voice-channels/{id}/token", post(token_handler))
}

#[tracing::instrument(skip_all)]
async fn list_handler(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<VoiceChannel>>, ApiError> {
    let channels = channels::list_channels(&state.db).await?;
    Ok(Json(channels))
}

#[tracing::instrument(skip_all)]
async fn create_handler(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    payload: Result<Json<CreateVoiceChannelRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<VoiceChannel>), ApiError> {
    let Json(req) = payload.map_err(|rejection| ApiError::InvalidRequest(rejection.body_text()))?;
    let channel = channels::create_channel(&state.db, &req.name)
        .await
        .inspect_err(|err| {
            tracing::debug!(error = %err, "criação de sala de voz recusada");
        })?;
    tracing::info!(
        channel_id = channel.id,
        name = %channel.name,
        user_id = user.id,
        "sala de voz criada"
    );
    state.publish(ServerMessage::VoiceChannelCreated {
        channel: channel.clone(),
    });
    Ok((StatusCode::CREATED, Json(channel)))
}

#[tracing::instrument(skip_all)]
async fn token_handler(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    channel_id: Result<Path<u32>, PathRejection>,
) -> Result<Json<VoiceJoin>, ApiError> {
    let Path(channel_id) =
        channel_id.map_err(|rejection| ApiError::InvalidRequest(rejection.body_text()))?;
    let Some(livekit) = &state.livekit else {
        return Err(ApiError::VoiceUnavailable);
    };
    if !channels::channel_exists(&state.db, channel_id).await? {
        return Err(ApiError::UnknownChannel);
    }
    let token = livekit
        .issue(&user, channel_id, auth::now())
        .map_err(ApiError::internal)?;
    tracing::debug!(channel_id, user_id = user.id, "token de voz emitido");
    Ok(Json(VoiceJoin {
        token,
        livekit_port: livekit.port(),
    }))
}
