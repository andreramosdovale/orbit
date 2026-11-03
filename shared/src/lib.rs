//! Protocolo do Orbit: tipos trafegados entre `sun` e `moon`.
//!
//! Toda mudança aqui é uma mudança de protocolo: atualize `docs/protocol.md`
//! e os dois lados (`sun` e `moon`) no mesmo commit.

use serde::{Deserialize, Serialize};

/// Mensagens enviadas pelo cliente via WebSocket (`/ws`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Ping,
}

/// Mensagens enviadas pelo servidor via WebSocket (`/ws`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Pong,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Fixam o formato no fio: se um destes quebrar, o protocolo mudou.
    #[test]
    fn client_message_wire_format() {
        assert_eq!(
            serde_json::to_value(ClientMessage::Ping).unwrap(),
            json!({ "type": "ping" })
        );
    }

    #[test]
    fn server_message_wire_format() {
        assert_eq!(
            serde_json::to_value(ServerMessage::Pong).unwrap(),
            json!({ "type": "pong" })
        );
    }
}
