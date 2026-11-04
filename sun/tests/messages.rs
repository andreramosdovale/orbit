//! Testes das mensagens em tempo real (spec 006).

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{TestServer, Ws, recv, recv_close_code, send, send_request};
use orbit_shared::{
    ChatMessage, ClientMessage, CreateChannelRequest, MessageRejectReason, ServerMessage,
    TextChannel, User,
};

/// O canal `geral`, criado pela migration.
const GENERAL: u32 = 1;

fn say(channel_id: u32, content: &str) -> ClientMessage {
    ClientMessage::SendMessage {
        channel_id,
        content: content.into(),
    }
}

async fn recv_message(ws: &mut Ws) -> anyhow::Result<ChatMessage> {
    match recv(ws).await? {
        ServerMessage::NewMessage { message } => Ok(message),
        other => anyhow::bail!("esperava new_message, veio {other:?}"),
    }
}

async fn count_messages(server: &TestServer) -> anyhow::Result<i64> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM messages")
        .fetch_one(&server.state.db)
        .await?)
}

/// Servidor com duas contas de verdade, já conectadas.
struct Chat {
    server: TestServer,
    fulano: User,
    fulano_ws: Ws,
    ciclano_ws: Ws,
}

async fn chat() -> anyhow::Result<Chat> {
    let server = TestServer::start().await?;
    let fulano = server.register("fulano").await?;
    let ciclano = server.register("ciclano").await?;
    let fulano_ws = server.connect_ws_as(&fulano).await?;
    let ciclano_ws = server.connect_ws_as(&ciclano).await?;
    Ok(Chat {
        server,
        fulano,
        fulano_ws,
        ciclano_ws,
    })
}

#[tokio::test]
async fn message_reaches_all_connections() -> anyhow::Result<()> {
    let mut chat = chat().await?;

    send(&mut chat.fulano_ws, &say(GENERAL, "oi")).await?;

    let received = recv_message(&mut chat.ciclano_ws).await?;
    assert_eq!(received.channel_id, GENERAL);
    assert_eq!(received.content, "oi");
    assert_eq!(received.author, chat.fulano);
    assert!(!received.created_at.is_empty());
    // Quem enviou recebe exatamente a mesma mensagem.
    assert_eq!(recv_message(&mut chat.fulano_ws).await?, received);
    Ok(())
}

#[tokio::test]
async fn message_is_persisted() -> anyhow::Result<()> {
    let mut chat = chat().await?;

    send(&mut chat.fulano_ws, &say(GENERAL, "guarda isto")).await?;
    let received = recv_message(&mut chat.ciclano_ws).await?;

    let row: (i64, i64, String, String) = sqlx::query_as(
        "SELECT channel_id, author_id, content, created_at FROM messages WHERE id = ?",
    )
    .bind(i64::from(received.id))
    .fetch_one(&chat.server.state.db)
    .await?;
    assert_eq!(row.0, i64::from(GENERAL));
    assert_eq!(row.1, i64::from(chat.fulano.id));
    assert_eq!(row.2, "guarda isto");
    assert_eq!(row.3, received.created_at);
    Ok(())
}

#[tokio::test]
async fn messages_keep_order() -> anyhow::Result<()> {
    let mut chat = chat().await?;
    let sent: Vec<String> = (0..20).map(|n| format!("mensagem {n}")).collect();

    for content in &sent {
        send(&mut chat.fulano_ws, &say(GENERAL, content)).await?;
    }

    let mut last_id = 0;
    for content in &sent {
        let received = recv_message(&mut chat.ciclano_ws).await?;
        assert_eq!(&received.content, content);
        assert!(received.id > last_id, "ids devem crescer");
        last_id = received.id;
    }
    Ok(())
}

#[tokio::test]
async fn content_is_trimmed() -> anyhow::Result<()> {
    let mut chat = chat().await?;

    send(&mut chat.fulano_ws, &say(GENERAL, "  oi  ")).await?;

    assert_eq!(recv_message(&mut chat.ciclano_ws).await?.content, "oi");
    Ok(())
}

#[tokio::test]
async fn invalid_content_is_rejected() -> anyhow::Result<()> {
    let mut chat = chat().await?;
    let too_long = "a".repeat(2001);

    for content in ["", "   ", too_long.as_str()] {
        send(&mut chat.fulano_ws, &say(GENERAL, content)).await?;
        assert_eq!(
            recv(&mut chat.fulano_ws).await?,
            ServerMessage::MessageRejected {
                reason: MessageRejectReason::InvalidContent
            }
        );
    }
    assert_eq!(count_messages(&chat.server).await?, 0);

    // O limite exato passa, e é a primeira coisa que o outro cliente recebe:
    // nenhuma das recusas chegou até ele.
    let longest = "a".repeat(2000);
    send(&mut chat.fulano_ws, &say(GENERAL, &longest)).await?;
    assert_eq!(recv_message(&mut chat.ciclano_ws).await?.content, longest);
    Ok(())
}

#[tokio::test]
async fn unknown_channel_is_rejected() -> anyhow::Result<()> {
    let mut chat = chat().await?;

    send(&mut chat.fulano_ws, &say(999, "oi")).await?;

    assert_eq!(
        recv(&mut chat.fulano_ws).await?,
        ServerMessage::MessageRejected {
            reason: MessageRejectReason::UnknownChannel
        }
    );
    assert_eq!(count_messages(&chat.server).await?, 0);
    // A conexão continua.
    send(&mut chat.fulano_ws, &ClientMessage::Ping).await?;
    assert_eq!(recv(&mut chat.fulano_ws).await?, ServerMessage::Pong);
    Ok(())
}

#[tokio::test]
async fn send_before_auth_is_rejected() -> anyhow::Result<()> {
    let server = TestServer::start().await?;
    let mut ws = server.connect_ws().await?;

    send(&mut ws, &say(GENERAL, "oi")).await?;

    assert_eq!(recv(&mut ws).await?, ServerMessage::AuthError);
    assert_eq!(recv_close_code(&mut ws).await?, 1008);
    assert_eq!(count_messages(&server).await?, 0);
    Ok(())
}

#[tokio::test]
async fn channel_created_reaches_connections() -> anyhow::Result<()> {
    let mut chat = chat().await?;
    let token = chat.server.state.jwt.issue(&chat.fulano, sun::now())?;
    let body = serde_json::to_vec(&CreateChannelRequest {
        name: "jogos".into(),
    })?;
    let req = Request::post("/channels")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body))?;

    let res = send_request(&chat.server.state, req).await?;

    assert_eq!(res.status, StatusCode::CREATED);
    let created: TextChannel = res.json()?;
    for ws in [&mut chat.fulano_ws, &mut chat.ciclano_ws] {
        assert_eq!(
            recv(ws).await?,
            ServerMessage::ChannelCreated {
                channel: created.clone()
            }
        );
    }
    Ok(())
}

#[tokio::test]
async fn lagging_client_is_disconnected() -> anyhow::Result<()> {
    let mut chat = chat().await?;
    let channel = TextChannel {
        id: 99,
        name: "enxurrada".into(),
    };

    // O runtime do teste tem uma thread só: sem nenhum `.await`, o servidor não
    // roda durante o laço, e a conexão fica atrás de tudo o que foi publicado.
    for _ in 0..=sun::HUB_CAPACITY {
        chat.server.state.publish(ServerMessage::ChannelCreated {
            channel: channel.clone(),
        });
    }

    assert_eq!(recv_close_code(&mut chat.ciclano_ws).await?, 1013);
    Ok(())
}
