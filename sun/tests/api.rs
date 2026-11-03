//! Testes de integração do `sun`: HTTP direto no router, WebSocket numa porta efêmera.

mod common;

use anyhow::Context;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{TIMEOUT, TestServer, recv, send};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use orbit_shared::{ClientMessage, ServerMessage};
use tokio_tungstenite::tungstenite::Message;
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok() -> anyhow::Result<()> {
    let req = Request::get("/health").body(Body::empty())?;
    let res = sun::app().oneshot(req).await?;

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await?.to_bytes();
    assert_eq!(&body[..], b"ok");
    Ok(())
}

#[tokio::test]
async fn ws_ping_gets_pong() -> anyhow::Result<()> {
    let server = TestServer::start().await?;
    let mut ws = server.connect_ws().await?;

    send(&mut ws, &ClientMessage::Ping).await?;
    assert_eq!(recv(&mut ws).await?, ServerMessage::Pong);
    Ok(())
}

#[tokio::test]
async fn ws_invalid_message_keeps_connection() -> anyhow::Result<()> {
    let server = TestServer::start().await?;
    let mut ws = server.connect_ws().await?;

    ws.send(Message::text("isto não é json")).await?;
    ws.send(Message::text(r#"{"type":"nao_existe"}"#)).await?;

    // A conexão continua viva: o Ping seguinte ainda é respondido.
    send(&mut ws, &ClientMessage::Ping).await?;
    assert_eq!(recv(&mut ws).await?, ServerMessage::Pong);
    Ok(())
}

#[tokio::test]
async fn ws_oversized_message_closes_connection() -> anyhow::Result<()> {
    let server = TestServer::start().await?;
    let mut ws = server.connect_ws().await?;

    let huge = "a".repeat(sun::MAX_MESSAGE_SIZE + 1);
    // O envio pode falhar se o servidor fechar no meio; o que importa é o fechamento abaixo.
    if let Err(err) = ws.send(Message::text(huge)).await {
        eprintln!("envio interrompido (esperado): {err}");
    }

    // O servidor encerra a conexão: fim do stream, erro ou frame de Close, nunca uma resposta.
    loop {
        let next = tokio::time::timeout(TIMEOUT, ws.next())
            .await
            .context("o servidor não fechou a conexão")?;
        match next {
            None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
            Some(Ok(other)) => anyhow::bail!("esperava fechamento, veio {other:?}"),
        }
    }
    Ok(())
}
