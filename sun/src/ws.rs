//! Conexão WebSocket com o `moon`. Mensagens seguem `orbit_shared`.
//!
//! Toda conexão começa pela autenticação (ADR 0009): só quem manda um JWT válido
//! como primeira mensagem chega ao loop de mensagens, que também repassa ao
//! cliente o que é publicado no `hub`. Enquanto ela durar, a pessoa conta como
//! online, e um ping periódico descobre as conexões que morreram sem avisar.

use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code};
use axum::extract::{ConnectInfo, State};
use axum::response::Response;
use orbit_shared::{ClientMessage, MessageRejectReason, ServerMessage, User};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use tokio::time::{Instant, MissedTickBehavior};
use tracing::Instrument;

use crate::AppState;
use crate::auth::{self, JwtKeys};
use crate::chat::messages;
use crate::presence;

/// Tamanho máximo de uma mensagem do cliente. O protocolo só tem mensagens
/// pequenas (JSON de controle e chat); qualquer coisa maior é abuso ou bug.
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024;

/// Prazo para o cliente se autenticar depois de abrir a conexão.
pub const WS_AUTH_TIMEOUT: Duration = Duration::from_secs(10);

/// De quanto em quanto tempo o servidor envia um ping. O webview responde sozinho.
pub const WS_PING_INTERVAL: Duration = Duration::from_secs(20);

/// Sem receber nada do cliente por esse tempo, a conexão é dada como morta.
pub const WS_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// Prazo de cada envio ao cliente. Um cliente travado não pode segurar a conexão.
const SEND_TIMEOUT: Duration = Duration::from_secs(10);

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

    if let Some((user, hub)) = authenticate(&mut socket, &state).await {
        tracing::info!(user_id = user.id, username = %user.username, "conexão WebSocket autenticada");
        let span = tracing::info_span!("user", user_id = user.id);
        session(socket, &state, &user, hub).instrument(span).await;
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
async fn authenticate(
    socket: &mut WebSocket,
    state: &AppState,
) -> Option<(User, broadcast::Receiver<ServerMessage>)> {
    // O prazo cobre a etapa inteira: frames de controle não o renovam.
    let outcome = tokio::time::timeout(state.ws_auth_timeout, wait_for_auth(socket, &state.jwt));
    let failure = match outcome.await {
        Ok(Ok(user)) => {
            // Assina antes de responder: nada publicado depois do `auth_ok` se perde.
            let hub = state.hub.subscribe();
            let reply = ServerMessage::AuthOk { user: user.clone() };
            return match send_message(socket, &reply).await {
                Ok(()) => Some((user, hub)),
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
            Ok(ClientMessage::Ping | ClientMessage::SendMessage { .. }) => {
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
    close(socket, close_code::POLICY, "autenticação recusada").await
}

/// Da entrada na presença até o fim da conexão.
async fn session(
    mut socket: WebSocket,
    state: &AppState,
    user: &User,
    hub: broadcast::Receiver<ServerMessage>,
) {
    // Destruído em qualquer saída daqui, o que anuncia a saída da pessoa.
    let _online = state.presence.connect(user);

    let users = match presence::snapshot(&state.db, &state.presence).await {
        Ok(users) => users,
        Err(err) => {
            tracing::error!(error = ?err, "falha ao ler a presença");
            if let Err(err) = close(&mut socket, close_code::ERROR, "erro interno").await {
                tracing::debug!(error = %err, "erro ao fechar a conexão");
            }
            return;
        }
    };
    let snapshot = ServerMessage::PresenceSnapshot { users };
    if let Err(err) = send_message(&mut socket, &snapshot).await {
        tracing::debug!(error = %err, "erro ao enviar; encerrando conexão");
        return;
    }

    message_loop(socket, state, user, hub).await;
}

async fn message_loop(
    mut socket: WebSocket,
    state: &AppState,
    user: &User,
    mut hub: broadcast::Receiver<ServerMessage>,
) {
    let mut last_seen = Instant::now();
    let mut ping =
        tokio::time::interval_at(last_seen + state.ws_ping_interval, state.ws_ping_interval);
    ping.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        // As duas esperas podem ser canceladas sem perder mensagem. O tratamento
        // roda depois do `select!`, então uma resposta nunca é interrompida no meio.
        let outgoing = tokio::select! {
            incoming = socket.recv() => {
                let msg = match incoming {
                    Some(Ok(msg)) => msg,
                    Some(Err(err)) => {
                        tracing::debug!(error = %err, "erro ao receber; encerrando conexão");
                        break;
                    }
                    None => break,
                };
                last_seen = Instant::now();
                // Ping/Pong/Close de controle são tratados pelo axum; binário não faz parte do protocolo.
                let Message::Text(text) = msg else { continue };
                match handle_client_message(&text, state, user).await {
                    Some(reply) => reply,
                    None => continue,
                }
            }
            published = hub.recv() => match published {
                Ok(msg) => msg,
                Err(RecvError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "cliente atrasado; encerrando conexão");
                    let closed = close(&mut socket, close_code::AGAIN, "conexão atrasada").await;
                    if let Err(err) = closed {
                        tracing::debug!(error = %err, "erro ao fechar a conexão atrasada");
                    }
                    break;
                }
                // Só acontece se o `AppState` inteiro sumir: não há mais o que repassar.
                Err(RecvError::Closed) => break,
            },
            _ = ping.tick() => {
                if let Err(err) = send_frame(&mut socket, Message::Ping(Default::default())).await {
                    tracing::debug!(error = %err, "erro ao enviar ping; encerrando conexão");
                    break;
                }
                continue;
            }
            () = tokio::time::sleep_until(last_seen + state.ws_idle_timeout) => {
                tracing::debug!("cliente não responde; encerrando conexão");
                if let Err(err) = close(&mut socket, close_code::AWAY, "sem resposta").await {
                    tracing::debug!(error = %err, "erro ao fechar a conexão morta");
                }
                break;
            }
        };

        match send_message(&mut socket, &outgoing).await {
            Ok(()) => {}
            Err(SendError::Serialize(err)) => {
                tracing::error!(error = %err, "falha ao serializar resposta");
            }
            Err(err @ (SendError::Socket(_) | SendError::Timeout)) => {
                tracing::debug!(error = %err, "erro ao enviar; encerrando conexão");
                break;
            }
        }
    }
}

/// Trata uma mensagem do cliente autenticado. Devolve o que responder só a ele.
async fn handle_client_message(text: &str, state: &AppState, user: &User) -> Option<ServerMessage> {
    match serde_json::from_str::<ClientMessage>(text) {
        Ok(ClientMessage::Ping) => Some(ServerMessage::Pong),
        Ok(ClientMessage::Auth { .. }) => {
            tracing::warn!("auth repetido em conexão já autenticada; ignorado");
            None
        }
        Ok(ClientMessage::SendMessage {
            channel_id,
            content,
        }) => match messages::send_message(&state.db, user, channel_id, &content).await {
            Ok(message) => {
                tracing::debug!(message_id = message.id, channel_id, "mensagem gravada");
                state.publish(ServerMessage::NewMessage { message });
                None
            }
            Err(err) => {
                let reason = err.reject_reason();
                if reason == MessageRejectReason::Internal {
                    tracing::error!(error = ?err, channel_id, "falha ao gravar mensagem");
                } else {
                    tracing::debug!(error = %err, channel_id, "mensagem recusada");
                }
                Some(ServerMessage::MessageRejected { reason })
            }
        },
        Err(err) => {
            // Só a categoria: o texto do erro do serde pode repetir trechos
            // da mensagem, e ela pode conter um token.
            tracing::warn!(categoria = ?err.classify(), "mensagem inválida do cliente");
            None
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum SendError {
    #[error("falha ao serializar a mensagem")]
    Serialize(#[source] serde_json::Error),
    #[error("erro no socket")]
    Socket(#[source] axum::Error),
    #[error("o cliente não recebeu dentro do prazo")]
    Timeout,
}

async fn send_message(socket: &mut WebSocket, msg: &ServerMessage) -> Result<(), SendError> {
    let json = serde_json::to_string(msg).map_err(SendError::Serialize)?;
    send_frame(socket, Message::Text(json.into())).await
}

/// Envia o frame de fechamento com `code`.
async fn close(socket: &mut WebSocket, code: u16, reason: &'static str) -> Result<(), SendError> {
    let frame = CloseFrame {
        code,
        reason: reason.into(),
    };
    send_frame(socket, Message::Close(Some(frame))).await
}

async fn send_frame(socket: &mut WebSocket, frame: Message) -> Result<(), SendError> {
    match tokio::time::timeout(SEND_TIMEOUT, socket.send(frame)).await {
        Ok(result) => result.map_err(SendError::Socket),
        Err(_elapsed) => Err(SendError::Timeout),
    }
}
