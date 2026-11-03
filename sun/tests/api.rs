//! Testes de integração do `sun`: HTTP direto no router, WebSocket numa porta efêmera.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use orbit_shared::{ClientMessage, ServerMessage};
use tokio_tungstenite::tungstenite::Message;
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok() {
    let res = sun::app()
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"ok");
}

#[tokio::test]
async fn ws_ping_gets_pong() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, sun::app()).await });

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
        .await
        .unwrap();

    let ping = serde_json::to_string(&ClientMessage::Ping).unwrap();
    ws.send(Message::text(ping)).await.unwrap();

    let reply = ws.next().await.unwrap().unwrap();
    let reply: ServerMessage = serde_json::from_str(reply.to_text().unwrap()).unwrap();
    assert_eq!(reply, ServerMessage::Pong);
}
