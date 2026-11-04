//! Salas de voz: criação e listagem. Seguem as mesmas regras de nome dos canais de texto.

use orbit_shared::VoiceChannel;
use sqlx::SqlitePool;

use crate::chat::channels::{ChannelError, normalize_name};

fn to_channel(id: i64, name: String) -> Result<VoiceChannel, ChannelError> {
    let id = u32::try_from(id).map_err(|_| ChannelError::IdOutOfRange(id))?;
    Ok(VoiceChannel { id, name })
}

/// Todas as salas, na ordem de criação.
pub async fn list_channels(db: &SqlitePool) -> Result<Vec<VoiceChannel>, ChannelError> {
    let rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, name FROM voice_channels ORDER BY id")
            .fetch_all(db)
            .await
            .map_err(ChannelError::Db)?;
    rows.into_iter()
        .map(|(id, name)| to_channel(id, name))
        .collect()
}

pub async fn create_channel(db: &SqlitePool, name: &str) -> Result<VoiceChannel, ChannelError> {
    let name = normalize_name(name).ok_or(ChannelError::InvalidName)?;
    let id: i64 = sqlx::query_scalar("INSERT INTO voice_channels (name) VALUES (?) RETURNING id")
        .bind(&name)
        .fetch_one(db)
        .await
        .map_err(|err| match err.as_database_error() {
            Some(db_err) if db_err.is_unique_violation() => ChannelError::NameTaken,
            _ => ChannelError::Db(err),
        })?;
    to_channel(id, name)
}

pub async fn channel_exists(db: &SqlitePool, channel_id: u32) -> Result<bool, ChannelError> {
    let found: Option<i64> = sqlx::query_scalar("SELECT id FROM voice_channels WHERE id = ?")
        .bind(i64::from(channel_id))
        .fetch_optional(db)
        .await
        .map_err(ChannelError::Db)?;
    Ok(found.is_some())
}
