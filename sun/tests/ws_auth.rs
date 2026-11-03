//! Testes da autenticação do WebSocket (spec 003, ADR 0009).

mod common;

use std::time::Duration;

use common::{TestServer, Ws, post_json, recv, recv_close_code, send};
use futures_util::SinkExt;
use orbit_shared::{
    ClientMessage, LoginRequest, LoginResponse, RegisterRequest, ServerMessage, User,
};
use tokio_tungstenite::tungstenite::Message;

const PASSWORD: &str = "senha-boa-123";
/// Código de fechamento "policy violation".
const CLOSE_POLICY: u16 = 1008;

fn auth(token: impl Into<String>) -> ClientMessage {
    ClientMessage::Auth {
        token: token.into(),
    }
}

/// O servidor recusa: `auth_error`, depois fechamento 1008.
async fn assert_rejected(ws: &mut Ws) -> anyhow::Result<()> {
    assert_eq!(recv(ws).await?, ServerMessage::AuthError);
    assert_eq!(recv_close_code(ws).await?, CLOSE_POLICY);
    Ok(())
}

#[tokio::test]
async fn login_token_authenticates() -> anyhow::Result<()> {
    let server = TestServer::start().await?;
    let register = RegisterRequest {
        username: "fulano".into(),
        password: PASSWORD.into(),
    };
    post_json(&server.state, "/auth/register", &register).await?;
    let login = LoginRequest {
        username: "fulano".into(),
        password: PASSWORD.into(),
    };
    let res: LoginResponse = post_json(&server.state, "/auth/login", &login)
        .await?
        .json()?;

    let mut ws = server.connect_ws().await?;
    send(&mut ws, &auth(res.token)).await?;

    assert_eq!(
        recv(&mut ws).await?,
        ServerMessage::AuthOk { user: res.user }
    );
    send(&mut ws, &ClientMessage::Ping).await?;
    assert_eq!(recv(&mut ws).await?, ServerMessage::Pong);
    Ok(())
}

#[tokio::test]
async fn invalid_token_is_rejected() -> anyhow::Result<()> {
    let server = TestServer::start().await?;
    let mut ws = server.connect_ws().await?;

    send(&mut ws, &auth("nao.e.jwt")).await?;

    assert_rejected(&mut ws).await
}

#[tokio::test]
async fn expired_token_is_rejected() -> anyhow::Result<()> {
    let server = TestServer::start().await?;
    let user = User {
        id: 1,
        username: "fulano".into(),
    };
    let long_ago = sun::now() - 31 * 24 * 60 * 60;
    let token = server.state.jwt.issue(&user, long_ago)?;
    let mut ws = server.connect_ws().await?;

    send(&mut ws, &auth(token)).await?;

    assert_rejected(&mut ws).await
}

#[tokio::test]
async fn ping_before_auth_is_rejected() -> anyhow::Result<()> {
    let server = TestServer::start().await?;
    let mut ws = server.connect_ws().await?;

    send(&mut ws, &ClientMessage::Ping).await?;

    assert_rejected(&mut ws).await
}

#[tokio::test]
async fn malformed_message_before_auth_is_rejected() -> anyhow::Result<()> {
    let server = TestServer::start().await?;
    let mut ws = server.connect_ws().await?;

    ws.send(Message::text("isto não é json")).await?;

    assert_rejected(&mut ws).await
}

#[tokio::test]
async fn binary_message_before_auth_is_rejected() -> anyhow::Result<()> {
    let server = TestServer::start().await?;
    let mut ws = server.connect_ws().await?;

    ws.send(Message::binary(vec![1, 2, 3])).await?;

    assert_rejected(&mut ws).await
}

#[tokio::test]
async fn auth_timeout_closes_connection() -> anyhow::Result<()> {
    let mut state = common::state().await?;
    state.ws_auth_timeout = Duration::from_millis(100);
    let server = TestServer::start_with(state).await?;

    let mut ws = server.connect_ws().await?;

    assert_rejected(&mut ws).await
}

#[tokio::test]
async fn second_auth_is_ignored() -> anyhow::Result<()> {
    let server = TestServer::start().await?;
    let mut ws = server.connect_ws_authed().await?;

    send(&mut ws, &auth("nao.e.jwt")).await?;
    send(&mut ws, &ClientMessage::Ping).await?;

    // A primeira resposta já é o pong: o `auth` não gerou resposta nem derrubou a conexão.
    assert_eq!(recv(&mut ws).await?, ServerMessage::Pong);
    Ok(())
}
