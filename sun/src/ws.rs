//! Conexão WebSocket com o `moon`. Mensagens seguem `orbit_shared`.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use orbit_shared::{ClientMessage, ServerMessage};

pub async fn upgrade(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle)
}

async fn handle(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        let Message::Text(text) = msg else { continue };

        let reply = match serde_json::from_str::<ClientMessage>(&text) {
            Ok(ClientMessage::Ping) => ServerMessage::Pong,
            Err(err) => {
                tracing::warn!(%err, "mensagem inválida do cliente");
                continue;
            }
        };

        let Ok(json) = serde_json::to_string(&reply) else {
            continue;
        };
        if socket.send(Message::Text(json.into())).await.is_err() {
            break;
        }
    }
}
