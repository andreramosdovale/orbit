//! Testes das salas de voz e dos tokens do LiveKit (spec 010).

mod common;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{JsonResponse, TestServer, recv, send_request, state};
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use orbit_shared::{
    ApiErrorBody, ApiErrorCode, CreateVoiceChannelRequest, ServerMessage, User, VoiceChannel,
    VoiceJoin,
};
use serde_json::Value;
use sun::{AppState, LiveKitConfig};

const API_KEY: &str = "chave-de-teste";
const API_SECRET: &str = "segredo-de-teste-bem-longo-para-o-livekit";
const GERAL: u32 = 1;

fn fulano() -> User {
    User {
        id: 7,
        username: "fulano".into(),
    }
}

async fn state_with_keys() -> anyhow::Result<AppState> {
    let mut state = state().await?;
    state.livekit = LiveKitConfig::from_values(
        Some(API_KEY.into()),
        Some(API_SECRET.into()),
        Some("7999".into()),
    )?;
    Ok(state)
}

async fn call(
    state: &AppState,
    method: Method,
    path: &str,
    body: Option<&impl serde::Serialize>,
) -> anyhow::Result<JsonResponse> {
    let token = state.jwt.issue(&fulano(), sun::now())?;
    let builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json");
    let body = match body {
        Some(value) => Body::from(serde_json::to_vec(value)?),
        None => Body::empty(),
    };
    send_request(state, builder.body(body)?).await
}

async fn get(state: &AppState, path: &str) -> anyhow::Result<JsonResponse> {
    call(state, Method::GET, path, None::<&()>).await
}

async fn post_token(state: &AppState, channel_id: &str) -> anyhow::Result<JsonResponse> {
    let path = format!("/voice-channels/{channel_id}/token");
    call(state, Method::POST, &path, None::<&()>).await
}

async fn create(state: &AppState, name: &str) -> anyhow::Result<JsonResponse> {
    let req = CreateVoiceChannelRequest { name: name.into() };
    call(state, Method::POST, "/voice-channels", Some(&req)).await
}

fn assert_error(res: &JsonResponse, status: StatusCode, code: ApiErrorCode) -> anyhow::Result<()> {
    assert_eq!(res.status, status);
    assert_eq!(res.json::<ApiErrorBody>()?.code, code);
    Ok(())
}

#[tokio::test]
async fn lists_default_channel() -> anyhow::Result<()> {
    let state = state().await?;

    let res = get(&state, "/voice-channels").await?;

    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(
        res.json::<Vec<VoiceChannel>>()?,
        vec![VoiceChannel {
            id: GERAL,
            name: "geral".into()
        }]
    );
    Ok(())
}

#[tokio::test]
async fn create_channel() -> anyhow::Result<()> {
    let server = TestServer::start().await?;
    let mut ws = server.connect_ws_authed().await?;

    let res = create(&server.state, "  Música ").await?;

    assert_eq!(res.status, StatusCode::CREATED);
    let created: VoiceChannel = res.json()?;
    assert_eq!(created.name, "música");
    assert_eq!(
        recv(&mut ws).await?,
        ServerMessage::VoiceChannelCreated {
            channel: created.clone()
        }
    );

    let names = get(&server.state, "/voice-channels")
        .await?
        .json::<Vec<VoiceChannel>>()?;
    assert_eq!(names.last(), Some(&created));

    assert_error(
        &create(&server.state, "música").await?,
        StatusCode::CONFLICT,
        ApiErrorCode::ChannelNameTaken,
    )?;
    assert_error(
        &create(&server.state, "dois nomes").await?,
        StatusCode::BAD_REQUEST,
        ApiErrorCode::InvalidChannelName,
    )
}

#[tokio::test]
async fn names_do_not_conflict_with_text_channels() -> anyhow::Result<()> {
    let state = state().await?;
    let text: Vec<(String,)> = sqlx::query_as("SELECT name FROM channels")
        .fetch_all(&state.db)
        .await?;
    assert!(text.iter().any(|(name,)| name == "geral"));

    let res = create(&state, "jogos").await?;

    assert_eq!(res.status, StatusCode::CREATED);
    let text_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channels WHERE name = 'jogos'")
        .fetch_one(&state.db)
        .await?;
    assert_eq!(text_count, 0);
    Ok(())
}

#[tokio::test]
async fn requires_auth() -> anyhow::Result<()> {
    let state = state_with_keys().await?;

    for (method, path) in [
        (Method::GET, "/voice-channels"),
        (Method::POST, "/voice-channels"),
        (Method::POST, "/voice-channels/1/token"),
        (Method::POST, "/voice-channels/abc/token"),
    ] {
        let req = Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())?;
        let res = send_request(&state, req).await?;
        assert_error(&res, StatusCode::UNAUTHORIZED, ApiErrorCode::Unauthorized)?;
    }
    Ok(())
}

#[tokio::test]
async fn token_grants() -> anyhow::Result<()> {
    let state = state_with_keys().await?;
    let before = sun::now();

    let res = post_token(&state, "1").await?;

    assert_eq!(res.status, StatusCode::OK);
    let join: VoiceJoin = res.json()?;
    assert_eq!(join.livekit_port, 7999);

    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[API_KEY]);
    let claims = jsonwebtoken::decode::<Value>(
        &join.token,
        &DecodingKey::from_secret(API_SECRET.as_bytes()),
        &validation,
    )?
    .claims;

    assert_eq!(claims["sub"], "7");
    assert_eq!(claims["name"], "fulano");
    let video = &claims["video"];
    assert_eq!(video["room"], "voice-1");
    assert_eq!(video["roomJoin"], true);
    assert_eq!(video["canSubscribe"], true);
    assert_eq!(video["canPublish"], true);
    assert_eq!(video["canPublishData"], false);
    assert_eq!(
        video["canPublishSources"],
        serde_json::json!(["microphone"])
    );

    let exp = claims["exp"].as_u64().expect("exp deveria ser um número");
    let nbf = claims["nbf"].as_u64().expect("nbf deveria ser um número");
    assert_eq!(exp - nbf, 600, "o token dura 10 minutos");
    assert!(nbf >= before && nbf <= sun::now());
    Ok(())
}

#[tokio::test]
async fn token_for_unknown_channel() -> anyhow::Result<()> {
    let state = state_with_keys().await?;

    assert_error(
        &post_token(&state, "999").await?,
        StatusCode::NOT_FOUND,
        ApiErrorCode::UnknownChannel,
    )?;
    assert_error(
        &post_token(&state, "abc").await?,
        StatusCode::BAD_REQUEST,
        ApiErrorCode::InvalidRequest,
    )
}

#[tokio::test]
async fn token_without_keys() -> anyhow::Result<()> {
    let state = state().await?;

    assert_error(
        &post_token(&state, "1").await?,
        StatusCode::SERVICE_UNAVAILABLE,
        ApiErrorCode::VoiceUnavailable,
    )?;

    // O resto do servidor não depende da voz.
    assert_eq!(get(&state, "/channels").await?.status, StatusCode::OK);
    assert_eq!(get(&state, "/voice-channels").await?.status, StatusCode::OK);
    Ok(())
}

#[tokio::test]
async fn secret_never_reaches_a_response() -> anyhow::Result<()> {
    let state = state_with_keys().await?;

    let res = post_token(&state, "1").await?;

    let body = String::from_utf8(res.body.to_vec())?;
    assert!(!body.contains(API_SECRET));
    Ok(())
}
