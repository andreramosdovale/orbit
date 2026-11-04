//! Servidor do Orbit. O binário (`main.rs`) só lê a config e chama [`serve`];
//! toda a lógica fica aqui para ser testável sem abrir porta.

mod auth;
mod chat;
pub mod db;
mod error;
mod presence;
mod voice;
mod ws;

use std::future::Future;
use std::net::SocketAddr;
use std::ops::RangeInclusive;
use std::time::Duration;

use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method, header};
use axum::{Router, routing::get};
use orbit_shared::ServerMessage;
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower_http::cors::{AllowOrigin, CorsLayer};

use presence::Presence;

pub use auth::{Claims, JwtError, JwtKeys, SecretError, now};
pub use voice::{LiveKitConfig, LiveKitConfigError};
pub use ws::{MAX_MESSAGE_SIZE, WS_AUTH_TIMEOUT, WS_IDLE_TIMEOUT, WS_PING_INTERVAL};

/// Quantas mensagens o `hub` guarda para quem está atrasado. Quem passa disso é desconectado.
pub const HUB_CAPACITY: usize = 256;

/// Quantas mensagens vêm em cada página do histórico.
pub const MESSAGE_PAGE_SIZE: u8 = 50;

/// Tamanho máximo do corpo de uma requisição HTTP. A API só recebe JSON pequeno.
pub const MAX_BODY_SIZE: usize = 16 * 1024;

/// Origens do webview do `moon` no build. Em dev, ver [`DEV_PORTS`].
pub const ALLOWED_ORIGINS: [&str; 2] = ["http://tauri.localhost", "tauri://localhost"];

/// Portas de `http://localhost` liberadas no CORS: as que o Vite do `moon` usa em dev
/// (`moon/scripts/dev.mjs`), uma por instância aberta.
pub const DEV_PORTS: RangeInclusive<u16> = 1420..=1439;

/// Estado compartilhado por todas as rotas.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    /// Chaves do JWT, carregadas do banco quando o servidor sobe (ADR 0008).
    pub jwt: JwtKeys,
    /// Prazo para autenticar no `/ws`. Em produção, [`WS_AUTH_TIMEOUT`]; os testes usam um valor curto.
    pub ws_auth_timeout: Duration,
    /// Publica para todas as conexões WebSocket autenticadas.
    pub hub: broadcast::Sender<ServerMessage>,
    /// De quanto em quanto tempo o `/ws` envia um ping. Em produção, [`WS_PING_INTERVAL`].
    pub ws_ping_interval: Duration,
    /// Sem receber nada por esse tempo, o `/ws` encerra a conexão. Em produção, [`WS_IDLE_TIMEOUT`].
    pub ws_idle_timeout: Duration,
    /// Quem está conectado agora.
    presence: Presence,
    /// Chaves do LiveKit. Sem elas, a voz fica indisponível e o resto funciona.
    pub livekit: Option<LiveKitConfig>,
}

impl AppState {
    pub fn new(db: SqlitePool, jwt: JwtKeys) -> Self {
        let (hub, _) = broadcast::channel(HUB_CAPACITY);
        Self {
            db,
            jwt,
            ws_auth_timeout: WS_AUTH_TIMEOUT,
            ws_ping_interval: WS_PING_INTERVAL,
            ws_idle_timeout: WS_IDLE_TIMEOUT,
            presence: Presence::new(hub.clone()),
            livekit: None,
            hub,
        }
    }

    /// Envia `msg` a todas as conexões autenticadas.
    pub fn publish(&self, msg: ServerMessage) {
        // Só falha sem nenhuma conexão aberta, e aí não há a quem entregar.
        if self.hub.send(msg).is_err() {
            tracing::debug!("nenhuma conexão para receber a publicação");
        }
    }
}

/// Monta o router com todas as rotas do servidor.
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ws", get(ws::upgrade))
        .merge(auth::routes())
        .merge(chat::routes())
        .merge(voice::routes())
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
        .layer(cors())
        .with_state(state)
}

fn is_allowed_origin(origin: &HeaderValue) -> bool {
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    ALLOWED_ORIGINS.contains(&origin)
        || origin
            .strip_prefix("http://localhost:")
            .and_then(|port| port.parse().ok())
            .is_some_and(|port: u16| DEV_PORTS.contains(&port))
}

fn cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            is_allowed_origin(origin)
        }))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
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
