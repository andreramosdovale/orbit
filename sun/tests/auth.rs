//! Testes do auto-cadastro (`POST /auth/register`, spec 001) e do login
//! (`POST /auth/login`, spec 002). Ver `docs/specs/`.

mod common;

use anyhow::Context;
use argon2::Argon2;
use argon2::password_hash::PasswordVerifier;
use argon2::password_hash::phc::PasswordHash;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{post_json, state};
use orbit_shared::{
    ApiErrorBody, ApiErrorCode, LoginRequest, LoginResponse, RegisterRequest, User,
};
use sun::{AppState, JwtKeys};
use tower::ServiceExt;

const PASSWORD: &str = "senha-boa-123";

fn req(username: &str, password: &str) -> RegisterRequest {
    RegisterRequest {
        username: username.into(),
        password: password.into(),
    }
}

async fn register(
    state: &AppState,
    username: &str,
    password: &str,
) -> anyhow::Result<common::JsonResponse> {
    post_json(state, "/auth/register", &req(username, password)).await
}

/// Garante que a resposta é um erro com o status e o código esperados.
fn assert_error(
    res: &common::JsonResponse,
    status: StatusCode,
    code: ApiErrorCode,
) -> anyhow::Result<()> {
    assert_eq!(res.status, status);
    let body: ApiErrorBody = res.json()?;
    assert_eq!(body.code, code);
    assert!(!body.message.is_empty(), "mensagem de erro vazia");
    Ok(())
}

async fn count_users(state: &AppState) -> anyhow::Result<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .context("contando usuários")
}

#[tokio::test]
async fn register_creates_user() -> anyhow::Result<()> {
    let state = state().await?;

    let res = register(&state, "fulano", PASSWORD).await?;

    assert_eq!(res.status, StatusCode::CREATED);
    let user: User = res.json()?;
    assert_eq!(user.username, "fulano");

    let (id, username): (i64, String) = sqlx::query_as("SELECT id, username FROM users")
        .fetch_one(&state.db)
        .await?;
    assert_eq!(id, i64::from(user.id));
    assert_eq!(username, "fulano");
    Ok(())
}

#[tokio::test]
async fn register_normalizes_username() -> anyhow::Result<()> {
    let state = state().await?;

    let res = register(&state, "Fulano", PASSWORD).await?;

    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.json::<User>()?.username, "fulano");
    Ok(())
}

#[tokio::test]
async fn register_duplicate_returns_409() -> anyhow::Result<()> {
    let state = state().await?;
    assert_eq!(
        register(&state, "fulano", PASSWORD).await?.status,
        StatusCode::CREATED
    );

    for dup in ["fulano", "FULANO"] {
        let res = register(&state, dup, PASSWORD).await?;
        assert_error(&res, StatusCode::CONFLICT, ApiErrorCode::UsernameTaken)?;
    }
    assert_eq!(count_users(&state).await?, 1);
    Ok(())
}

#[tokio::test]
async fn register_invalid_username_returns_400() -> anyhow::Result<()> {
    let state = state().await?;
    let too_long = "a".repeat(33);

    for bad in ["ab", too_long.as_str(), "com espaço", "joão", "a-b"] {
        let res = register(&state, bad, PASSWORD).await?;
        assert_error(&res, StatusCode::BAD_REQUEST, ApiErrorCode::InvalidUsername)
            .with_context(|| format!("username {bad:?}"))?;
    }
    assert_eq!(count_users(&state).await?, 0);
    Ok(())
}

#[tokio::test]
async fn register_invalid_password_returns_400() -> anyhow::Result<()> {
    let state = state().await?;

    for bad in ["a".repeat(7), "a".repeat(129)] {
        let res = register(&state, "fulano", &bad).await?;
        assert_error(&res, StatusCode::BAD_REQUEST, ApiErrorCode::InvalidPassword)
            .with_context(|| format!("senha com {} caracteres", bad.len()))?;
    }
    assert_eq!(count_users(&state).await?, 0);
    Ok(())
}

#[tokio::test]
async fn register_malformed_body_returns_400() -> anyhow::Result<()> {
    let state = state().await?;

    let res = post_json(
        &state,
        "/auth/register",
        &serde_json::json!({ "username": "fulano" }),
    )
    .await?;

    assert_error(&res, StatusCode::BAD_REQUEST, ApiErrorCode::InvalidRequest)
}

#[tokio::test]
async fn register_stores_argon2_hash() -> anyhow::Result<()> {
    let state = state().await?;
    assert_eq!(
        register(&state, "fulano", PASSWORD).await?.status,
        StatusCode::CREATED
    );

    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE username = ?")
        .bind("fulano")
        .fetch_one(&state.db)
        .await?;

    assert!(hash.starts_with("$argon2id$"), "hash inesperado: {hash}");
    assert!(
        !hash.contains(PASSWORD),
        "o hash contém a senha em texto puro"
    );
    let parsed = PasswordHash::new(&hash).map_err(|err| anyhow::anyhow!("hash inválido: {err}"))?;
    assert!(
        Argon2::default()
            .verify_password(PASSWORD.as_bytes(), &parsed)
            .is_ok()
    );
    Ok(())
}

#[tokio::test]
async fn register_response_never_contains_password() -> anyhow::Result<()> {
    let state = state().await?;

    let res = register(&state, "fulano", PASSWORD).await?;

    let body = String::from_utf8_lossy(&res.body);
    assert!(!body.contains(PASSWORD));
    assert!(!body.contains("argon2"));
    Ok(())
}

#[tokio::test]
async fn register_cors_allows_app_origin() -> anyhow::Result<()> {
    let state = state().await?;

    for (origin, allowed) in [
        ("http://tauri.localhost", true),
        ("http://localhost:1420", true),
        ("http://evil.example", false),
    ] {
        let req = Request::builder()
            .method(Method::OPTIONS)
            .uri("/auth/register")
            .header(header::ORIGIN, origin)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
            .body(Body::empty())?;
        let res = sun::app(state.clone()).oneshot(req).await?;
        let allow = res.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN);
        if allowed {
            assert_eq!(
                allow.map(|v| v.as_bytes()),
                Some(origin.as_bytes()),
                "origem {origin}"
            );
        } else {
            assert!(allow.is_none(), "origem {origin} não deveria ser liberada");
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- login (spec 002)

async fn login(
    state: &AppState,
    username: &str,
    password: &str,
) -> anyhow::Result<common::JsonResponse> {
    let req = LoginRequest {
        username: username.into(),
        password: password.into(),
    };
    post_json(state, "/auth/login", &req).await
}

/// Estado com `fulano` / `PASSWORD` já cadastrado.
async fn state_with_fulano() -> anyhow::Result<(AppState, User)> {
    let state = state().await?;
    let res = register(&state, "fulano", PASSWORD).await?;
    assert_eq!(res.status, StatusCode::CREATED);
    let user = res.json()?;
    Ok((state, user))
}

#[tokio::test]
async fn login_returns_token_and_user() -> anyhow::Result<()> {
    let (state, user) = state_with_fulano().await?;

    let res = login(&state, "fulano", PASSWORD).await?;

    assert_eq!(res.status, StatusCode::OK);
    let body: LoginResponse = res.json()?;
    assert_eq!(body.user, user);
    assert!(!body.token.is_empty());

    let claims = state.jwt.verify(&body.token, sun::now())?;
    assert_eq!(claims.sub, user.id.to_string());
    assert_eq!(claims.username, "fulano");
    Ok(())
}

#[tokio::test]
async fn login_is_case_insensitive() -> anyhow::Result<()> {
    let (state, user) = state_with_fulano().await?;

    let res = login(&state, "FULANO", PASSWORD).await?;

    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json::<LoginResponse>()?.user, user);
    Ok(())
}

#[tokio::test]
async fn login_wrong_password_returns_401() -> anyhow::Result<()> {
    let (state, _) = state_with_fulano().await?;

    let res = login(&state, "fulano", "senha-errada-123").await?;

    assert_error(
        &res,
        StatusCode::UNAUTHORIZED,
        ApiErrorCode::InvalidCredentials,
    )
}

#[tokio::test]
async fn login_unknown_user_returns_401() -> anyhow::Result<()> {
    let (state, _) = state_with_fulano().await?;

    let unknown = login(&state, "ciclano", PASSWORD).await?;
    let wrong_password = login(&state, "fulano", "senha-errada-123").await?;

    assert_error(
        &unknown,
        StatusCode::UNAUTHORIZED,
        ApiErrorCode::InvalidCredentials,
    )?;
    assert_eq!(
        unknown.body, wrong_password.body,
        "usuário inexistente e senha errada devem dar a mesma resposta"
    );
    Ok(())
}

#[tokio::test]
async fn login_malformed_body_returns_400() -> anyhow::Result<()> {
    let state = state().await?;

    let res = post_json(
        &state,
        "/auth/login",
        &serde_json::json!({ "username": "fulano" }),
    )
    .await?;

    assert_error(&res, StatusCode::BAD_REQUEST, ApiErrorCode::InvalidRequest)
}

#[tokio::test]
async fn login_response_never_contains_password() -> anyhow::Result<()> {
    let (state, _) = state_with_fulano().await?;

    let res = login(&state, "fulano", PASSWORD).await?;

    assert_eq!(res.status, StatusCode::OK);
    let body = String::from_utf8_lossy(&res.body);
    assert!(!body.contains(PASSWORD));
    assert!(!body.contains("argon2"));
    Ok(())
}

#[tokio::test]
async fn jwt_secret_persists() -> anyhow::Result<()> {
    let path = common::TempDb::new()?;
    let user = User {
        id: 1,
        username: "fulano".into(),
    };

    let db = sun::db::connect(&path.0).await?;
    let first = JwtKeys::load_or_create(&db).await?;
    let token = first.issue(&user, sun::now())?;
    db.close().await;

    let db = sun::db::connect(&path.0).await?;
    let second = JwtKeys::load_or_create(&db).await?;
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM secrets")
        .fetch_one(&db)
        .await?;
    db.close().await;

    assert_eq!(rows, 1, "o segredo não deveria ser gerado de novo");
    let claims = second.verify(&token, sun::now())?;
    assert_eq!(claims.username, "fulano");
    Ok(())
}
