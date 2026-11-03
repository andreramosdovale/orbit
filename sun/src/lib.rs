//! Servidor do Orbit. O binário (`main.rs`) só lê a config e chama [`serve`];
//! toda a lógica fica aqui para ser testável sem abrir porta.

mod auth;
pub mod db;
mod error;
mod ws;

use std::future::Future;
use std::net::SocketAddr;

use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method, header};
use axum::{Router, routing::get};
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

pub use ws::MAX_MESSAGE_SIZE;

/// Tamanho máximo do corpo de uma requisição HTTP. A API só recebe JSON pequeno.
pub const MAX_BODY_SIZE: usize = 16 * 1024;

/// Origens do webview do `moon` liberadas no CORS: build (`tauri.localhost`) e dev (Vite).
pub const ALLOWED_ORIGINS: [&str; 3] = [
    "http://tauri.localhost",
    "tauri://localhost",
    "http://localhost:1420",
];

/// Estado compartilhado por todas as rotas.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
}

/// Monta o router com todas as rotas do servidor.
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ws", get(ws::upgrade))
        .merge(auth::routes())
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
        .layer(cors())
        .with_state(state)
}

fn cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(ALLOWED_ORIGINS.map(HeaderValue::from_static))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE])
}

/// Serve o [`app`] no `listener` até `shutdown` completar, esperando as
/// conexões em andamento terminarem (encerramento gracioso).
pub async fn serve<F>(listener: TcpListener, state: AppState, shutdown: F) -> std::io::Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    let service = app(state).into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, service)
        .with_graceful_shutdown(shutdown)
        .await
}
