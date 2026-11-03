//! Protocolo do Orbit: tipos trafegados entre `sun` e `moon`.

use serde::{Deserialize, Serialize};

/// Mensagens enviadas pelo cliente via WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Ping,
}

/// Mensagens enviadas pelo servidor via WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Pong,
}
