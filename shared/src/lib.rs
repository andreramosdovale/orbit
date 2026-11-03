//! Protocolo do Orbit: tipos trafegados entre `sun` e `moon`.
//!
//! Toda mudança aqui é uma mudança de protocolo: atualize `docs/protocol.md`
//! e os dois lados (`sun` e `moon`) no mesmo commit.

use serde::{Deserialize, Serialize};

/// Mensagens enviadas pelo cliente via WebSocket (`/ws`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Ping,
}

/// Mensagens enviadas pelo servidor via WebSocket (`/ws`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Pong,
}

/// Corpo de `POST /auth/register`: auto-cadastro de um usuário novo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RegisterRequest {
    /// 3 a 32 caracteres (`a-z`, `0-9`, `_`). O servidor converte para minúsculas.
    pub username: String,
    /// 8 a 128 caracteres.
    pub password: String,
}

/// Usuário público. Nunca contém senha nem hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct User {
    pub id: u32,
    pub username: String,
}

/// Corpo de `POST /auth/login`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct LoginRequest {
    /// Convertido para minúsculas pelo servidor antes da busca.
    pub username: String,
    pub password: String,
}

/// Resposta de `POST /auth/login`: o JWT e o usuário logado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct LoginResponse {
    /// JWT HS256, válido por 30 dias. Vai em `Authorization: Bearer <token>`.
    pub token: String,
    pub user: User,
}

/// Corpo de toda resposta de erro da API REST.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ApiErrorBody {
    pub code: ApiErrorCode,
    /// Texto em português, pronto para mostrar ao usuário.
    pub message: String,
}

/// Código estável do erro, para o front decidir o que fazer sem depender do texto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    /// Corpo da requisição malformado (JSON inválido ou campo faltando).
    InvalidRequest,
    /// Nome de usuário fora das regras.
    InvalidUsername,
    /// Senha fora das regras.
    InvalidPassword,
    /// Nome de usuário já cadastrado.
    UsernameTaken,
    /// Usuário ou senha incorretos no login (não diz qual dos dois).
    InvalidCredentials,
    /// Erro interno do servidor. Detalhes só no log do `sun`.
    Internal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Fixam o formato no fio: se um destes quebrar, o protocolo mudou.
    #[test]
    fn client_message_wire_format() {
        assert_eq!(
            serde_json::to_value(ClientMessage::Ping).unwrap(),
            json!({ "type": "ping" })
        );
    }

    #[test]
    fn server_message_wire_format() {
        assert_eq!(
            serde_json::to_value(ServerMessage::Pong).unwrap(),
            json!({ "type": "pong" })
        );
    }

    #[test]
    fn register_request_wire_format() {
        let req = RegisterRequest {
            username: "fulano".into(),
            password: "segredo123".into(),
        };
        assert_eq!(
            serde_json::to_value(req).unwrap(),
            json!({ "username": "fulano", "password": "segredo123" })
        );
    }

    #[test]
    fn user_wire_format() {
        let user = User {
            id: 1,
            username: "fulano".into(),
        };
        assert_eq!(
            serde_json::to_value(user).unwrap(),
            json!({ "id": 1, "username": "fulano" })
        );
    }

    #[test]
    fn login_request_wire_format() {
        let req = LoginRequest {
            username: "fulano".into(),
            password: "segredo123".into(),
        };
        assert_eq!(
            serde_json::to_value(req).unwrap(),
            json!({ "username": "fulano", "password": "segredo123" })
        );
    }

    #[test]
    fn login_response_wire_format() {
        let res = LoginResponse {
            token: "a.b.c".into(),
            user: User {
                id: 1,
                username: "fulano".into(),
            },
        };
        assert_eq!(
            serde_json::to_value(res).unwrap(),
            json!({ "token": "a.b.c", "user": { "id": 1, "username": "fulano" } })
        );
    }

    #[test]
    fn invalid_credentials_wire_format() {
        assert_eq!(
            serde_json::to_value(ApiErrorCode::InvalidCredentials).unwrap(),
            json!("invalid_credentials")
        );
    }

    #[test]
    fn api_error_body_wire_format() {
        let body = ApiErrorBody {
            code: ApiErrorCode::UsernameTaken,
            message: "nome de usuário já existe".into(),
        };
        assert_eq!(
            serde_json::to_value(body).unwrap(),
            json!({ "code": "username_taken", "message": "nome de usuário já existe" })
        );
    }
}
