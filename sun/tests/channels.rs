//! Testes dos canais de texto e da autenticação REST (spec 005).

mod common;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{JsonResponse, send_request, state};
use orbit_shared::{ApiErrorBody, ApiErrorCode, CreateChannelRequest, TextChannel, User};
use sun::{AppState, JwtKeys};
use tower::ServiceExt;

fn fulano() -> User {
    User {
        id: 1,
        username: "fulano".into(),
    }
}

fn token(state: &AppState) -> anyhow::Result<String> {
    Ok(state.jwt.issue(&fulano(), sun::now())?)
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

async fn list(state: &AppState, authorization: Option<&str>) -> anyhow::Result<JsonResponse> {
    let mut req = Request::get("/channels");
    if let Some(value) = authorization {
        req = req.header(header::AUTHORIZATION, value);
    }
    send_request(state, req.body(Body::empty())?).await
}

async fn create_raw(
    state: &AppState,
    authorization: Option<&str>,
    body: Vec<u8>,
) -> anyhow::Result<JsonResponse> {
    let mut req = Request::post("/channels").header(header::CONTENT_TYPE, "application/json");
    if let Some(value) = authorization {
        req = req.header(header::AUTHORIZATION, value);
    }
    send_request(state, req.body(Body::from(body))?).await
}

/// `POST /channels` com um token válido.
async fn create(state: &AppState, name: &str) -> anyhow::Result<JsonResponse> {
    let body = serde_json::to_vec(&CreateChannelRequest { name: name.into() })?;
    create_raw(state, Some(&bearer(&token(state)?)), body).await
}

async fn names(state: &AppState) -> anyhow::Result<Vec<String>> {
    let res = list(state, Some(&bearer(&token(state)?))).await?;
    assert_eq!(res.status, StatusCode::OK);
    let channels: Vec<TextChannel> = res.json()?;
    Ok(channels.into_iter().map(|c| c.name).collect())
}

fn assert_error(res: &JsonResponse, status: StatusCode, code: ApiErrorCode) -> anyhow::Result<()> {
    assert_eq!(res.status, status);
    let body: ApiErrorBody = res.json()?;
    assert_eq!(body.code, code);
    assert!(!body.message.is_empty(), "mensagem de erro vazia");
    Ok(())
}

#[tokio::test]
async fn list_requires_token() -> anyhow::Result<()> {
    let state = state().await?;
    let long_ago = sun::now() - 31 * 24 * 60 * 60;
    let expired = bearer(&state.jwt.issue(&fulano(), long_ago)?);
    // Assinado com outro segredo: a assinatura não pode bater.
    let other_keys = JwtKeys::load_or_create(&sun::db::connect_in_memory().await?).await?;
    let foreign = bearer(&other_keys.issue(&fulano(), sun::now())?);
    let raw_token = token(&state)?;

    for authorization in [
        None,
        Some("Bearer nao.e.jwt"),
        Some(expired.as_str()),
        Some(foreign.as_str()),
        // O token certo, mas sem o prefixo `Bearer `.
        Some(raw_token.as_str()),
    ] {
        let res = list(&state, authorization).await?;
        assert_error(&res, StatusCode::UNAUTHORIZED, ApiErrorCode::Unauthorized)?;
    }
    Ok(())
}

#[tokio::test]
async fn create_requires_token() -> anyhow::Result<()> {
    let state = state().await?;
    let body = serde_json::to_vec(&CreateChannelRequest {
        name: "novo".into(),
    })?;

    let res = create_raw(&state, None, body).await?;

    assert_error(&res, StatusCode::UNAUTHORIZED, ApiErrorCode::Unauthorized)?;
    assert_eq!(names(&state).await?, ["geral"]);
    Ok(())
}

#[tokio::test]
async fn new_server_has_general_channel() -> anyhow::Result<()> {
    let state = state().await?;

    assert_eq!(names(&state).await?, ["geral"]);
    Ok(())
}

#[tokio::test]
async fn create_returns_channel_and_lists_it() -> anyhow::Result<()> {
    let state = state().await?;

    let res = create(&state, "zebra").await?;
    assert_eq!(res.status, StatusCode::CREATED);
    let created: TextChannel = res.json()?;
    assert_eq!(created.name, "zebra");
    create(&state, "abelha").await?;

    // Ordem de criação, não alfabética.
    assert_eq!(names(&state).await?, ["geral", "zebra", "abelha"]);
    Ok(())
}

#[tokio::test]
async fn create_normalizes_name() -> anyhow::Result<()> {
    let state = state().await?;

    let res = create(&state, " Música ").await?;

    assert_eq!(res.status, StatusCode::CREATED);
    let created: TextChannel = res.json()?;
    assert_eq!(created.name, "música");
    Ok(())
}

#[tokio::test]
async fn create_duplicate_returns_409() -> anyhow::Result<()> {
    let state = state().await?;
    create(&state, "jogos").await?;

    for name in ["jogos", "JOGOS", "geral"] {
        let res = create(&state, name).await?;
        assert_error(&res, StatusCode::CONFLICT, ApiErrorCode::ChannelNameTaken)?;
    }
    assert_eq!(names(&state).await?, ["geral", "jogos"]);
    Ok(())
}

#[tokio::test]
async fn create_invalid_name_returns_400() -> anyhow::Result<()> {
    let state = state().await?;
    let too_long = "a".repeat(33);

    for name in ["", "   ", too_long.as_str(), "dois nomes", "sim!"] {
        let res = create(&state, name).await?;
        assert_error(
            &res,
            StatusCode::BAD_REQUEST,
            ApiErrorCode::InvalidChannelName,
        )?;
    }
    assert_eq!(names(&state).await?, ["geral"]);
    Ok(())
}

#[tokio::test]
async fn create_malformed_body_returns_400() -> anyhow::Result<()> {
    let state = state().await?;
    let authorization = bearer(&token(&state)?);

    let res = create_raw(&state, Some(&authorization), b"nao e json".to_vec()).await?;

    assert_error(&res, StatusCode::BAD_REQUEST, ApiErrorCode::InvalidRequest)
}

#[tokio::test]
async fn cors_allows_authorization_header() -> anyhow::Result<()> {
    let state = state().await?;
    let req = Request::builder()
        .method(Method::OPTIONS)
        .uri("/channels")
        .header(header::ORIGIN, "http://tauri.localhost")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
        .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "authorization")
        .body(Body::empty())?;

    let res = sun::app(state).oneshot(req).await?;

    let allowed = res
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_lowercase();
    assert!(
        allowed.contains("authorization"),
        "cabeçalhos liberados: {allowed:?}"
    );
    Ok(())
}

#[tokio::test]
async fn channels_persist() -> anyhow::Result<()> {
    let path = common::TempDb::new()?;

    let db = sun::db::connect(&path.0).await?;
    let first = common::state_with_db(db.clone()).await?;
    assert_eq!(create(&first, "jogos").await?.status, StatusCode::CREATED);
    db.close().await;

    let db = sun::db::connect(&path.0).await?;
    let second = common::state_with_db(db.clone()).await?;
    let found = names(&second).await?;
    db.close().await;

    assert_eq!(found, ["geral", "jogos"]);
    Ok(())
}
