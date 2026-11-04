//! Testes da presença (spec 008), com conexões WebSocket reais.

mod common;

use std::time::Duration;

use common::{TestServer, Ws, recv_any, send, state};
use futures_util::StreamExt;
use orbit_shared::{ClientMessage, ServerMessage, User, UserPresence};

async fn server() -> anyhow::Result<TestServer> {
    TestServer::start().await
}

/// Lê o `presence_snapshot`, que deve ser a primeira mensagem depois do `auth_ok`.
async fn recv_snapshot(ws: &mut Ws) -> anyhow::Result<Vec<UserPresence>> {
    match recv_any(ws).await? {
        ServerMessage::PresenceSnapshot { users } => Ok(users),
        other => anyhow::bail!("esperava presence_snapshot, veio {other:?}"),
    }
}

/// Conecta como `user` e consome o que toda entrada recebe: o retrato e o próprio anúncio.
async fn join(server: &TestServer, user: &User) -> anyhow::Result<Ws> {
    let mut ws = server.connect_ws_as(user).await?;
    recv_snapshot(&mut ws).await?;
    assert_eq!(
        recv_any(&mut ws).await?,
        ServerMessage::UserOnline { user: user.clone() }
    );
    Ok(ws)
}

/// Garante que nada chegou: `ping` é respondido só depois do que já estava na fila.
async fn assert_nothing_pending(ws: &mut Ws) -> anyhow::Result<()> {
    send(ws, &ClientMessage::Ping).await?;
    assert_eq!(recv_any(ws).await?, ServerMessage::Pong);
    Ok(())
}

#[tokio::test]
async fn snapshot_lists_all_accounts() -> anyhow::Result<()> {
    let server = server().await?;
    let fulano = server.register("fulano").await?;
    let ciclano = server.register("ciclano").await?;
    let beltrano = server.register("beltrano").await?;
    let _ciclano_ws = join(&server, &ciclano).await?;

    let mut ws = server.connect_ws_as(&fulano).await?;

    let users = recv_snapshot(&mut ws).await?;
    assert_eq!(
        users,
        vec![
            UserPresence {
                user: fulano,
                online: true
            },
            UserPresence {
                user: ciclano,
                online: true
            },
            UserPresence {
                user: beltrano,
                online: false
            },
        ]
    );
    Ok(())
}

#[tokio::test]
async fn connect_announces_online() -> anyhow::Result<()> {
    let server = server().await?;
    let fulano = server.register("fulano").await?;
    let ciclano = server.register("ciclano").await?;
    let mut fulano_ws = join(&server, &fulano).await?;

    let _ciclano_ws = server.connect_ws_as(&ciclano).await?;

    assert_eq!(
        recv_any(&mut fulano_ws).await?,
        ServerMessage::UserOnline { user: ciclano }
    );
    Ok(())
}

#[tokio::test]
async fn disconnect_announces_offline() -> anyhow::Result<()> {
    let server = server().await?;
    let fulano = server.register("fulano").await?;
    let ciclano = server.register("ciclano").await?;
    let mut fulano_ws = join(&server, &fulano).await?;
    let mut ciclano_ws = join(&server, &ciclano).await?;
    recv_any(&mut fulano_ws).await?; // user_online de ciclano

    ciclano_ws.close(None).await?;

    assert_eq!(
        recv_any(&mut fulano_ws).await?,
        ServerMessage::UserOffline { user: ciclano }
    );
    Ok(())
}

#[tokio::test]
async fn second_connection_is_silent() -> anyhow::Result<()> {
    let server = server().await?;
    let fulano = server.register("fulano").await?;
    let ciclano = server.register("ciclano").await?;
    let mut fulano_ws = join(&server, &fulano).await?;
    let mut first = join(&server, &ciclano).await?;
    recv_any(&mut fulano_ws).await?; // user_online de ciclano

    let mut second = server.connect_ws_as(&ciclano).await?;
    recv_snapshot(&mut second).await?;
    assert_nothing_pending(&mut fulano_ws).await?;

    first.close(None).await?;
    // Quando o `close` volta, o servidor pode ainda não ter visto o fechamento:
    // o `ping` da outra conexão não prova nada, então espera a primeira terminar.
    while first.next().await.is_some() {}
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_nothing_pending(&mut fulano_ws).await?;

    second.close(None).await?;
    assert_eq!(
        recv_any(&mut fulano_ws).await?,
        ServerMessage::UserOffline { user: ciclano }
    );
    Ok(())
}

#[tokio::test]
async fn failed_auth_is_not_announced() -> anyhow::Result<()> {
    let server = server().await?;
    let fulano = server.register("fulano").await?;
    let mut fulano_ws = join(&server, &fulano).await?;

    let mut intruder = server.connect_ws().await?;
    let token = "a.b.c".to_owned();
    send(&mut intruder, &ClientMessage::Auth { token }).await?;
    assert_eq!(recv_any(&mut intruder).await?, ServerMessage::AuthError);

    assert_nothing_pending(&mut fulano_ws).await
}

/// Servidor com ping e prazo curtos.
async fn impatient_server() -> anyhow::Result<TestServer> {
    let mut state = state().await?;
    state.ws_ping_interval = Duration::from_millis(50);
    state.ws_idle_timeout = Duration::from_millis(300);
    TestServer::start_with(state).await
}

#[tokio::test]
async fn dead_client_goes_offline() -> anyhow::Result<()> {
    let server = impatient_server().await?;
    let fulano = server.register("fulano").await?;
    let ciclano = server.register("ciclano").await?;
    let mut fulano_ws = join(&server, &fulano).await?;

    // Conecta e para de ler: não responde aos pings, como um PC que perdeu a rede.
    let ciclano_ws = server.connect_ws_as(&ciclano).await?;
    assert_eq!(
        recv_any(&mut fulano_ws).await?,
        ServerMessage::UserOnline {
            user: ciclano.clone()
        }
    );

    // `recv_any` responde aos pings de quem está vivo.
    assert_eq!(
        recv_any(&mut fulano_ws).await?,
        ServerMessage::UserOffline { user: ciclano }
    );
    drop(ciclano_ws);
    Ok(())
}

#[tokio::test]
async fn responsive_client_stays_online() -> anyhow::Result<()> {
    let server = impatient_server().await?;
    let fulano = server.register("fulano").await?;
    let mut ws = join(&server, &fulano).await?;

    // Passa do prazo só lendo: a biblioteca responde aos pings ao ler.
    let idle = tokio::time::timeout(Duration::from_millis(900), recv_any(&mut ws)).await;
    assert!(idle.is_err(), "não deveria chegar nada: {idle:?}");

    assert_nothing_pending(&mut ws).await
}
