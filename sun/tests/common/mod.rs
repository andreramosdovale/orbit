//! Helpers dos testes de integração do `sun`.

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, bail};
use futures_util::{SinkExt, StreamExt};
use orbit_shared::{ClientMessage, ServerMessage};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

/// Tempo máximo esperando qualquer resposta do servidor. Evita teste travado na CI.
pub const TIMEOUT: Duration = Duration::from_secs(5);

pub type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Servidor real numa porta efêmera. É abortado quando sai de escopo.
pub struct TestServer {
    pub addr: SocketAddr,
    handle: JoinHandle<std::io::Result<()>>,
}

impl TestServer {
    pub async fn start() -> anyhow::Result<Self> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .context("abrindo porta efêmera")?;
        let addr = listener.local_addr().context("lendo endereço local")?;
        let handle = tokio::spawn(sun::serve(listener, std::future::pending()));
        Ok(Self { addr, handle })
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
