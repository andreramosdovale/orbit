//! Servidor do Orbit. O binário (`main.rs`) só lê a config e chama [`serve`];
//! toda a lógica fica aqui para ser testável sem abrir porta.

mod ws;

use std::future::Future;
use std::net::SocketAddr;

use axum::{Router, routing::get};
use tokio::net::TcpListener;

pub use ws::MAX_MESSAGE_SIZE;

/// Monta o router com todas as rotas do servidor.
pub fn app() -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ws", get(ws::upgrade))
}

/// Serve o [`app`] no `listener` até `shutdown` completar, esperando as
/// conexões em andamento terminarem (encerramento gracioso).
pub async fn serve<F>(listener: TcpListener, shutdown: F) -> std::io::Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    let service = app().into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, service)
        .with_graceful_shutdown(shutdown)
        .await
}
