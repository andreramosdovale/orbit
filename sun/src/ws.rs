//! Conexão WebSocket com o `moon`. Mensagens seguem `orbit_shared`.
//!
//! Toda conexão começa pela autenticação (ADR 0009): só quem manda um JWT válido
//! como primeira mensagem chega ao loop de mensagens.

use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code};
use axum::extract::{ConnectInfo, State};
use axum::response::Response;
use orbit_shared::{ClientMessage, ServerMessage, User};
use tracing::Instrument;

use crate::AppState;
use crate::auth::{self, JwtKeys};

/// Tamanho máximo de uma mensagem do cliente. O protocolo só tem mensagens
/// pequenas (JSON de controle e chat); qualquer coisa maior é abuso ou bug.
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024;

/// Prazo para o cliente se autenticar depois de abrir a conexão.
pub const WS_AUTH_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
) -> Response {
    ws.max_message_size(MAX_MESSAGE_SIZE)
        .max_frame_size(MAX_MESSAGE_SIZE)
        .on_upgrade(move |socket| {
            handle(socket, state).instrument(tracing::info_span!("ws", %peer))
        })
}

async fn handle(mut socket: WebSocket, state: AppState) {
    tracing::debug!("conexão aberta");

    if let Some(user) = authenticate(&mut socket, &state).await {
        tracing::info!(user_id = user.id, username = %user.username, "conexão WebSocket autenticada");
        message_loop(socket)
            .instrument(tracing::info_span!("user", user_id = user.id))
            .await;
    }

    tracing::debug!("conexão encerrada");
}

/// Por que a etapa de autenticação não terminou com um usuário.
enum AuthFailure {
    /// O cliente saiu ou a conexão falhou: não há mais a quem responder.
    Disconnected,
    /// O cliente continua lá, mas não se autenticou: recebe `auth_error`.
    Rejected,
}

/// Espera a mensagem `auth` e responde. `None` significa que a conexão deve acabar.
async fn authenticate(socket: &mut WebSocket, state: &AppState) -> Option<User> {
    // O prazo cobre a etapa inteira: frames de controle não o renovam.
    let outcome = tokio::time::timeout(state.ws_auth_timeout, wait_for_auth(socket, &state.jwt));
    let failure = match outcome.await {
        Ok(Ok(user)) => {
            let reply = ServerMessage::AuthOk { user: user.clone() };
            return match send_message(socket, &reply).await {
                Ok(()) => Some(user),
                Err(err) => {
                    tracing::debug!(error = %err, "erro ao enviar; encerrando conexão");
                    None
                }
            };
        }
        Ok(Err(failure)) => failure,
        Err(_elapsed) => {
            tracing::debug!("prazo de autenticação esgotado");
            AuthFailure::Rejected
        }
    };

    match failure {
        AuthFailure::Disconnected => {}
        AuthFailure::Rejected => {
            if let Err(err) = reject(socket).await {
                tracing::debug!(error = %err, "erro ao recusar a autenticação");
            }
        }
    }
    None
}

/// Lê até decidir a autenticação. O motivo de uma recusa só vai para o log.
async fn wait_for_auth(socket: &mut WebSocket, jwt: &JwtKeys) -> Result<User, AuthFailure> {
    loop {
        let msg = match socket.recv().await {
            Some(Ok(msg)) => msg,
            Some(Err(err)) => {
                tracing::debug!(error = %err, "erro ao receber; encerrando conexão");
                return Err(AuthFailure::Disconnected);
            }
            None => return Err(AuthFailure::Disconnected),
        };

        let text = match msg {
            Message::Text(text) => text,
            Message::Ping(_) | Message::Pong(_) => continue,
            Message::Close(_) => return Err(AuthFailure::Disconnected),
            Message::Binary(_) => {
                tracing::debug!("frame binário antes de autenticar");
                return Err(AuthFailure::Rejected);
            }
        };

        let token = match serde_json::from_str::<ClientMessage>(&text) {
            Ok(ClientMessage::Auth { token }) => token,
            Ok(ClientMessage::Ping) => {
                tracing::debug!("mensagem antes de autenticar");
                return Err(AuthFailure::Rejected);
            }
            Err(err) => {
                tracing::debug!(categoria = ?err.classify(), "mensagem inválida antes de autenticar");
                return Err(AuthFailure::Rejected);
            }
        };

        let claims = jwt.verify(&token, auth::now()).map_err(|err| {
            tracing::debug!(error = %err, "token recusado");
            AuthFailure::Rejected
        })?;
        return claims.into_user().map_err(|err| {
            tracing::warn!(error = %err, "token assinado com `sub` que não é um id");
            AuthFailure::Rejected
        });
    }
}

/// Envia `auth_error` e fecha com o código 1008.
async fn reject(socket: &mut WebSocket) -> Result<(), SendError> {
    send_message(socket, &ServerMessage::AuthError).await?;
    socket
        .send(Message::Close(Some(CloseFrame {
            code: close_code::POLICY,
            reason: "autenticação recusada".into(),
        })))
        .await
        .map_err(SendError::Socket)
}

async fn message_loop(mut socket: WebSocket) {
    loop {
        let msg = match socket.recv().await {
            Some(Ok(msg)) => msg,
            Some(Err(err)) => {
                tracing::debug!(error = %err, "erro ao receber; encerrando conexão");
                break;
            }
            None => break,
        };

        // Ping/Pong/Close de controle são tratados pelo axum; binário não faz parte do protocolo.
        let Message::Text(text) = msg else { continue };

        let reply = match serde_json::from_str::<ClientMessage>(&text) {
            Ok(ClientMessage::Ping) => ServerMessage::Pong,
            Ok(ClientMessage::Auth { .. }) => {
                tracing::warn!("auth repetido em conexão já autenticada; ignorado");
                continue;
            }
            Err(err) => {
                // Só a categoria: o texto do erro do serde pode repetir trechos
                // da mensagem, e ela pode conter um token.
                tracing::warn!(categoria = ?err.classify(), "mensagem inválida do cliente");
                continue;
            }
        };

        match send_message(&mut socket, &reply).await {
            Ok(()) => {}
            Err(SendError::Serialize(err)) => {
                tracing::error!(error = %err, ?reply, "falha ao serializar resposta");
            }
            Err(SendError::Socket(err)) => {
                tracing::debug!(error = %err, "erro ao enviar; encerrando conexão");
                break;
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum SendError {
    #[error("falha ao serializar a mensagem")]
    Serialize(#[source] serde_json::Error),
    #[error("erro no socket")]
    Socket(#[source] axum::Error),
}

async fn send_message(socket: &mut WebSocket, msg: &ServerMessage) -> Result<(), SendError> {
    let json = serde_json::to_string(msg).map_err(SendError::Serialize)?;
    socket
        .send(Message::Text(json.into()))
        .await
        .map_err(SendError::Socket)
}
