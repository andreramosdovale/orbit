//! Servidor do Orbit. O binário (`main.rs`) só lê a config e chama [`app`];
//! toda a lógica fica aqui para ser testável sem abrir porta.

mod ws;

use axum::{Router, routing::get};

/// Monta o router com todas as rotas do servidor.
pub fn app() -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ws", get(ws::upgrade))
}
