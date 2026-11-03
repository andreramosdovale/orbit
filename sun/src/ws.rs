//! Conexão WebSocket com o `moon`. Mensagens seguem `orbit_shared`.

use std::net::SocketAddr;

use axum::extract::ConnectInfo;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use orbit_shared::{ClientMessage, ServerMessage};
use tracing::Instrument;

/// Tamanho máximo de uma mensagem do cliente. O protocolo só tem mensagens
/// pequenas (JSON de controle e chat); qualquer coisa maior é abuso ou bug.
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024;

pub async fn upgrade(ws: WebSocketUpgrade, ConnectInfo(peer): ConnectInfo<SocketAddr>) -> Response {
    ws.max_message_size(MAX_MESSAGE_SIZE)
        .max_frame_size(MAX_MESSAGE_SIZE)
        .on_upgrade(move |socket| handle(socket).instrument(tracing::info_span!("ws", %peer)))
}

async fn handle(mut socket: WebSocket) {
    tracing::debug!("conexão aberta");

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
            Err(err) => {
                tracing::warn!(error = %err, "mensagem inválida do cliente");
                continue;
            }
        };

        let json = match serde_json::to_string(&reply) {
            Ok(json) => json,
            Err(err) => {
                tracing::error!(error = %err, ?reply, "falha ao serializar resposta");
                continue;
            }
        };

        if let Err(err) = socket.send(Message::Text(json.into())).await {
            tracing::debug!(error = %err, "erro ao enviar; encerrando conexão");
            break;
        }
    }

    tracing::debug!("conexão encerrada");
}
