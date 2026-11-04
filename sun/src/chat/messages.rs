//! Mensagens de texto: validação, gravação e histórico. A entrega fica com o `ws.rs`.

use orbit_shared::{ChatMessage, MessagePage, MessageRejectReason, User};
use sqlx::SqlitePool;

use crate::MESSAGE_PAGE_SIZE;

/// Tamanho máximo do conteúdo, em caracteres.
const MAX_CONTENT_CHARS: usize = 2000;

#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error("conteúdo inválido")]
    InvalidContent,
    #[error("canal inexistente")]
    UnknownChannel,
    #[error("erro no banco")]
    Db(#[source] sqlx::Error),
    #[error("id de mensagem fora do intervalo: {0}")]
    IdOutOfRange(i64),
}

impl MessageError {
    /// O que o cliente fica sabendo. A causa de um erro interno só vai para o log.
    pub fn reject_reason(&self) -> MessageRejectReason {
        match self {
            Self::InvalidContent => MessageRejectReason::InvalidContent,
            Self::UnknownChannel => MessageRejectReason::UnknownChannel,
            Self::Db(_) | Self::IdOutOfRange(_) => MessageRejectReason::Internal,
        }
    }
}

/// Remove espaços nas pontas. `None` se o resultado não tiver de 1 a 2000 caracteres.
fn normalize_content(content: &str) -> Option<&str> {
    let content = content.trim();
    let valid = !content.is_empty() && content.chars().count() <= MAX_CONTENT_CHARS;
    valid.then_some(content)
}

async fn ensure_channel_exists(db: &SqlitePool, channel_id: u32) -> Result<(), MessageError> {
    let channel: Option<i64> = sqlx::query_scalar("SELECT id FROM channels WHERE id = ?")
        .bind(i64::from(channel_id))
        .fetch_optional(db)
        .await
        .map_err(MessageError::Db)?;
    channel.map(|_| ()).ok_or(MessageError::UnknownChannel)
}

fn to_u32(id: i64) -> Result<u32, MessageError> {
    u32::try_from(id).map_err(|_| MessageError::IdOutOfRange(id))
}

/// As [`MESSAGE_PAGE_SIZE`] mensagens mais recentes do canal com `id` menor que `before`,
/// ou simplesmente as mais recentes, sem `before`.
pub async fn message_page(
    db: &SqlitePool,
    channel_id: u32,
    before: Option<u32>,
) -> Result<MessagePage, MessageError> {
    ensure_channel_exists(db, channel_id).await?;

    // Uma linha a mais só para saber se existem mensagens mais antigas.
    let mut rows: Vec<(i64, String, String, i64, String)> = sqlx::query_as(
        "SELECT m.id, m.content, m.created_at, u.id, u.username \
         FROM messages m JOIN users u ON u.id = m.author_id \
         WHERE m.channel_id = ? AND m.id < ? \
         ORDER BY m.id DESC LIMIT ?",
    )
    .bind(i64::from(channel_id))
    .bind(before.map_or(i64::MAX, i64::from))
    .bind(i64::from(MESSAGE_PAGE_SIZE) + 1)
    .fetch_all(db)
    .await
    .map_err(MessageError::Db)?;

    let page_len = usize::from(MESSAGE_PAGE_SIZE);
    let has_more = rows.len() > page_len;
    rows.truncate(page_len);
    rows.reverse();

    let messages = rows
        .into_iter()
        .map(|(id, content, created_at, author_id, username)| {
            Ok(ChatMessage {
                id: to_u32(id)?,
                channel_id,
                author: User {
                    id: to_u32(author_id)?,
                    username,
                },
                content,
                created_at,
            })
        })
        .collect::<Result<_, MessageError>>()?;

    Ok(MessagePage { messages, has_more })
}

/// Valida e grava uma mensagem de `author`, devolvendo-a pronta para entregar.
pub async fn send_message(
    db: &SqlitePool,
    author: &User,
    channel_id: u32,
    content: &str,
) -> Result<ChatMessage, MessageError> {
    let content = normalize_content(content).ok_or(MessageError::InvalidContent)?;

    ensure_channel_exists(db, channel_id).await?;

    let (id, created_at): (i64, String) = sqlx::query_as(
        "INSERT INTO messages (channel_id, author_id, content) VALUES (?, ?, ?) \
         RETURNING id, created_at",
    )
    .bind(i64::from(channel_id))
    .bind(i64::from(author.id))
    .bind(content)
    .fetch_one(db)
    .await
    .map_err(MessageError::Db)?;

    Ok(ChatMessage {
        id: to_u32(id)?,
        channel_id,
        author: author.clone(),
        content: content.to_owned(),
        created_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_rules() {
        assert_eq!(normalize_content("  oi  "), Some("oi"));
        assert_eq!(normalize_content("a\nb"), Some("a\nb"));

        // O limite conta caracteres, não bytes.
        let longest = "á".repeat(MAX_CONTENT_CHARS);
        assert_eq!(normalize_content(&longest), Some(longest.as_str()));

        assert_eq!(normalize_content(""), None);
        assert_eq!(normalize_content(" \n "), None);
        assert_eq!(normalize_content(&"a".repeat(MAX_CONTENT_CHARS + 1)), None);
    }
}
