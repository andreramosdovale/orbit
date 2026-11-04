//! Testes do histórico paginado (spec 007).

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{JsonResponse, post_json, send_request, state};
use orbit_shared::{ApiErrorBody, ApiErrorCode, MessagePage, RegisterRequest, User};
use sun::{AppState, MESSAGE_PAGE_SIZE};

/// O canal `geral`, criado pela migration.
const GENERAL: u32 = 1;
const PAGE: usize = MESSAGE_PAGE_SIZE as usize; // u8 → usize nunca trunca.

async fn register(state: &AppState) -> anyhow::Result<User> {
    let req = RegisterRequest {
        username: "fulano".into(),
        password: "senha-boa-123".into(),
    };
    post_json(state, "/auth/register", &req).await?.json()
}

async fn create_channel(state: &AppState, name: &str) -> anyhow::Result<u32> {
    let id: i64 = sqlx::query_scalar("INSERT INTO channels (name) VALUES (?) RETURNING id")
        .bind(name)
        .fetch_one(&state.db)
        .await?;
    Ok(u32::try_from(id)?)
}

/// Grava `count` mensagens direto no banco, de `msg 0` em diante. Devolve os ids.
async fn seed(
    state: &AppState,
    author: &User,
    channel_id: u32,
    count: usize,
) -> anyhow::Result<Vec<u32>> {
    let mut ids = Vec::with_capacity(count);
    for i in 0..count {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (channel_id, author_id, content) VALUES (?, ?, ?) RETURNING id",
        )
        .bind(i64::from(channel_id))
        .bind(i64::from(author.id))
        .bind(format!("msg {i}"))
        .fetch_one(&state.db)
        .await?;
        ids.push(u32::try_from(id)?);
    }
    Ok(ids)
}

async fn get(state: &AppState, path: &str, author: &User) -> anyhow::Result<JsonResponse> {
    let token = state.jwt.issue(author, sun::now())?;
    let req = Request::get(path)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())?;
    send_request(state, req).await
}

async fn page(
    state: &AppState,
    author: &User,
    channel_id: u32,
    before: Option<u32>,
) -> anyhow::Result<MessagePage> {
    let query = before.map_or(String::new(), |id| format!("?before={id}"));
    let path = format!("/channels/{channel_id}/messages{query}");
    let res = get(state, &path, author).await?;
    assert_eq!(res.status, StatusCode::OK);
    res.json()
}

fn ids(page: &MessagePage) -> Vec<u32> {
    page.messages.iter().map(|m| m.id).collect()
}

fn assert_error(res: &JsonResponse, status: StatusCode, code: ApiErrorCode) -> anyhow::Result<()> {
    assert_eq!(res.status, status);
    assert_eq!(res.json::<ApiErrorBody>()?.code, code);
    Ok(())
}

#[tokio::test]
async fn latest_page() -> anyhow::Result<()> {
    let state = state().await?;
    let author = register(&state).await?;
    let all = seed(&state, &author, GENERAL, PAGE + 1).await?;

    let first = page(&state, &author, GENERAL, None).await?;

    assert!(first.has_more);
    assert_eq!(ids(&first), all[1..]);
    let last = first
        .messages
        .last()
        .expect("a página não deveria vir vazia");
    assert_eq!(last.channel_id, GENERAL);
    assert_eq!(last.author, author);
    assert_eq!(last.content, format!("msg {PAGE}"));
    assert!(!last.created_at.is_empty());
    Ok(())
}

#[tokio::test]
async fn exact_page_has_no_more() -> anyhow::Result<()> {
    let state = state().await?;
    let author = register(&state).await?;
    seed(&state, &author, GENERAL, PAGE).await?;

    let first = page(&state, &author, GENERAL, None).await?;

    assert_eq!(first.messages.len(), PAGE);
    assert!(!first.has_more);
    Ok(())
}

#[tokio::test]
async fn before_walks_to_the_start() -> anyhow::Result<()> {
    let state = state().await?;
    let author = register(&state).await?;
    let all = seed(&state, &author, GENERAL, 120).await?;

    let first = page(&state, &author, GENERAL, None).await?;
    let second = page(&state, &author, GENERAL, ids(&first).first().copied()).await?;
    let third = page(&state, &author, GENERAL, ids(&second).first().copied()).await?;

    assert_eq!(first.messages.len(), 50);
    assert_eq!(second.messages.len(), 50);
    assert_eq!(third.messages.len(), 20);
    assert!(first.has_more && second.has_more && !third.has_more);
    let walked: Vec<u32> = [ids(&third), ids(&second), ids(&first)].concat();
    assert_eq!(walked, all);
    Ok(())
}

#[tokio::test]
async fn new_message_does_not_shift_pages() -> anyhow::Result<()> {
    let state = state().await?;
    let author = register(&state).await?;
    let all = seed(&state, &author, GENERAL, 60).await?;

    let first = page(&state, &author, GENERAL, None).await?;
    seed(&state, &author, GENERAL, 5).await?;
    let second = page(&state, &author, GENERAL, ids(&first).first().copied()).await?;

    assert_eq!(ids(&second), all[..10]);
    assert!(!second.has_more);
    Ok(())
}

#[tokio::test]
async fn only_requested_channel() -> anyhow::Result<()> {
    let state = state().await?;
    let author = register(&state).await?;
    let games = create_channel(&state, "jogos").await?;
    let general = seed(&state, &author, GENERAL, 2).await?;
    seed(&state, &author, games, PAGE).await?;

    let first = page(&state, &author, GENERAL, None).await?;

    assert_eq!(ids(&first), general);
    assert!(!first.has_more);
    Ok(())
}

#[tokio::test]
async fn empty_channel() -> anyhow::Result<()> {
    let state = state().await?;
    let author = register(&state).await?;

    let first = page(&state, &author, GENERAL, None).await?;

    assert!(first.messages.is_empty());
    assert!(!first.has_more);
    Ok(())
}

#[tokio::test]
async fn unknown_channel() -> anyhow::Result<()> {
    let state = state().await?;
    let author = register(&state).await?;

    let res = get(&state, "/channels/999/messages", &author).await?;

    assert_error(&res, StatusCode::NOT_FOUND, ApiErrorCode::UnknownChannel)
}

#[tokio::test]
async fn invalid_params() -> anyhow::Result<()> {
    let state = state().await?;
    let author = register(&state).await?;

    for path in [
        "/channels/1/messages?before=abc",
        "/channels/1/messages?before=-1",
        "/channels/1/messages?before=4294967296",
        "/channels/abc/messages",
        "/channels/4294967296/messages",
    ] {
        let res = get(&state, path, &author).await?;
        assert_error(&res, StatusCode::BAD_REQUEST, ApiErrorCode::InvalidRequest)?;
    }
    Ok(())
}

#[tokio::test]
async fn requires_auth() -> anyhow::Result<()> {
    let state = state().await?;

    // Parâmetro inválido sem token também é 401: a autenticação vem primeiro.
    for path in ["/channels/1/messages", "/channels/abc/messages?before=x"] {
        let req = Request::get(path).body(Body::empty())?;
        let res = send_request(&state, req).await?;
        assert_error(&res, StatusCode::UNAUTHORIZED, ApiErrorCode::Unauthorized)?;
    }

    let req = Request::get("/channels/1/messages")
        .header(header::AUTHORIZATION, "Bearer a.b.c")
        .body(Body::empty())?;
    let res = send_request(&state, req).await?;
    assert_error(&res, StatusCode::UNAUTHORIZED, ApiErrorCode::Unauthorized)
}
