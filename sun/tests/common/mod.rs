//! Helpers dos testes de integração do `sun`.
//!
//! Cada arquivo em `tests/` compila este módulo separadamente e usa só parte dele.
#![allow(dead_code)]

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, bail};
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use orbit_shared::{ClientMessage, ServerMessage, User};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sun::{AppState, JwtKeys};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tower::ServiceExt;

/// Tempo máximo esperando qualquer resposta do servidor. Evita teste travado na CI.
pub const TIMEOUT: Duration = Duration::from_secs(5);

pub type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Estado com um SQLite em memória, já migrado. Cada chamada é um banco novo.
pub async fn state() -> anyhow::Result<AppState> {
    let db = sun::db::connect_in_memory()
        .await
        .context("abrindo SQLite em memória")?;
    let jwt = JwtKeys::load_or_create(&db)
        .await
        .context("carregando o segredo do JWT")?;
    Ok(AppState {
        db,
        jwt,
        ws_auth_timeout: sun::WS_AUTH_TIMEOUT,
    })
}

/// Resposta HTTP já lida: status + corpo.
pub struct JsonResponse {
    pub status: StatusCode,
    pub body: axum::body::Bytes,
}

impl JsonResponse {
    pub fn json<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
        serde_json::from_slice(&self.body).with_context(|| {
            format!(
                "corpo não é o JSON esperado: {}",
                String::from_utf8_lossy(&self.body)
            )
        })
    }
}

/// `POST path` com corpo JSON direto no router (sem abrir porta).
pub async fn post_json(
    state: &AppState,
    path: &str,
    body: &impl Serialize,
) -> anyhow::Result<JsonResponse> {
    let json = serde_json::to_vec(body).context("serializando corpo")?;
    let req = Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json))?;
    send_request(state, req).await
}

/// Envia uma requisição qualquer direto no router e lê o corpo inteiro.
pub async fn send_request(state: &AppState, req: Request<Body>) -> anyhow::Result<JsonResponse> {
    let res = sun::app(state.clone())
        .oneshot(req)
        .await
        .context("executando requisição")?;
    let status = res.status();
    let body = res
        .into_body()
        .collect()
        .await
        .context("lendo corpo")?
        .to_bytes();
    Ok(JsonResponse { status, body })
}

/// Servidor real numa porta efêmera. É abortado quando sai de escopo.
pub struct TestServer {
    pub addr: SocketAddr,
    /// O mesmo estado do servidor: mesmo banco e mesmas chaves do JWT.
    pub state: AppState,
    handle: JoinHandle<std::io::Result<()>>,
}

impl TestServer {
    pub async fn start() -> anyhow::Result<Self> {
        Self::start_with(state().await?).await
    }

    pub async fn start_with(state: AppState) -> anyhow::Result<Self> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .context("abrindo porta efêmera")?;
        let addr = listener.local_addr().context("lendo endereço local")?;
        let handle = tokio::spawn(sun::serve(listener, state.clone(), std::future::pending()));
        Ok(Self {
            addr,
            state,
            handle,
        })
    }

    pub async fn connect_ws(&self) -> anyhow::Result<Ws> {
        if self.handle.is_finished() {
            bail!("o servidor de teste encerrou antes da conexão");
        }
        let url = format!("ws://{}/ws", self.addr);
        let (ws, _) = tokio::time::timeout(TIMEOUT, tokio_tungstenite::connect_async(&url))
            .await
            .context("timeout conectando no WebSocket")?
            .with_context(|| format!("conectando em {url}"))?;
        Ok(ws)
    }

    /// Conexão já autenticada como um usuário de teste.
    pub async fn connect_ws_authed(&self) -> anyhow::Result<Ws> {
        let user = User {
            id: 1,
            username: "fulano".into(),
        };
        let token = self
            .state
            .jwt
            .issue(&user, sun::now())
            .context("emitindo token de teste")?;
        let mut ws = self.connect_ws().await?;
        send(&mut ws, &ClientMessage::Auth { token }).await?;
        match recv(&mut ws).await? {
            ServerMessage::AuthOk { .. } => Ok(ws),
            other => bail!("esperava auth_ok, veio {other:?}"),
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub async fn send(ws: &mut Ws, msg: &ClientMessage) -> anyhow::Result<()> {
    let json = serde_json::to_string(msg).context("serializando ClientMessage")?;
    ws.send(Message::text(json))
        .await
        .context("enviando mensagem")
}

/// Próxima mensagem de texto do servidor, já desserializada.
pub async fn recv(ws: &mut Ws) -> anyhow::Result<ServerMessage> {
    let msg = tokio::time::timeout(TIMEOUT, ws.next())
        .await
        .context("timeout esperando resposta do servidor")?
        .context("conexão fechada sem resposta")?
        .context("erro lendo do WebSocket")?;
    let text = msg
        .to_text()
        .with_context(|| format!("esperava texto, veio {msg:?}"))?;
    serde_json::from_str(text).with_context(|| format!("resposta inválida: {text}"))
}

/// Espera o servidor fechar a conexão e devolve o código do frame de Close.
pub async fn recv_close_code(ws: &mut Ws) -> anyhow::Result<u16> {
    loop {
        let msg = tokio::time::timeout(TIMEOUT, ws.next())
            .await
            .context("timeout esperando o fechamento")?
            .context("conexão acabou sem frame de Close")?
            .context("erro lendo do WebSocket")?;
        match msg {
            Message::Close(Some(frame)) => return Ok(u16::from(frame.code)),
            Message::Ping(_) | Message::Pong(_) => continue,
            other => bail!("esperava Close com código, veio {other:?}"),
        }
    }
}
