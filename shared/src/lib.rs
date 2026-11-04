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
    /// Primeira mensagem da conexão: o JWT recebido no login.
    Auth {
        token: String,
    },
    Ping,
    /// Envia uma mensagem a um canal. O autor sai da conexão autenticada.
    SendMessage {
        channel_id: u32,
        content: String,
    },
}

/// Mensagens enviadas pelo servidor via WebSocket (`/ws`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Token aceito: a conexão está autenticada como `user`.
    AuthOk {
        user: User,
    },
    /// Autenticação recusada. O servidor fecha a conexão com o código 1008 logo depois.
    AuthError,
    Pong,
    /// Mensagem nova, enviada a todas as conexões, inclusive a de quem enviou.
    NewMessage {
        message: ChatMessage,
    },
    /// Mensagem recusada, enviada só a quem tentou enviar.
    MessageRejected {
        reason: MessageRejectReason,
    },
    /// Canal novo, enviado a todas as conexões.
    ChannelCreated {
        channel: TextChannel,
    },
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
    /// JWT HS256, válido por 30 dias. No `/ws`, vai na mensagem `auth`; no REST, em `Authorization: Bearer <token>`.
    pub token: String,
    pub user: User,
}

/// Canal de texto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TextChannel {
    pub id: u32,
    pub name: String,
}

/// Mensagem de texto de um canal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ChatMessage {
    pub id: u32,
    pub channel_id: u32,
    pub author: User,
    pub content: String,
    /// Horário do servidor, em UTC (ISO 8601).
    pub created_at: String,
}

/// Uma página do histórico de um canal: resposta de `GET /channels/{id}/messages`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MessagePage {
    /// Da mais antiga para a mais nova.
    pub messages: Vec<ChatMessage>,
    /// Existem mensagens mais antigas que as desta página.
    pub has_more: bool,
}

/// Por que o servidor recusou uma mensagem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum MessageRejectReason {
    /// Conteúdo vazio ou acima de 2000 caracteres.
    InvalidContent,
    /// O canal não existe.
    UnknownChannel,
    /// Erro interno do servidor. Detalhes só no log do `sun`.
    Internal,
}

/// Corpo de `POST /channels`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CreateChannelRequest {
    /// 1 a 32 letras (com acento), números, `-` ou `_`. O servidor remove espaços nas pontas e converte para minúsculas.
    pub name: String,
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
    /// Token ausente, inválido ou expirado em uma rota protegida.
    Unauthorized,
    /// Nome de canal fora das regras.
    InvalidChannelName,
    /// Já existe um canal com esse nome.
    ChannelNameTaken,
    /// O canal pedido não existe.
    UnknownChannel,
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
    fn client_auth_wire_format() {
        let msg = ClientMessage::Auth {
            token: "a.b.c".into(),
        };
        assert_eq!(
            serde_json::to_value(msg).unwrap(),
            json!({ "type": "auth", "token": "a.b.c" })
        );
    }

    #[test]
    fn server_auth_ok_wire_format() {
        let msg = ServerMessage::AuthOk {
            user: User {
                id: 1,
                username: "fulano".into(),
            },
        };
        assert_eq!(
            serde_json::to_value(msg).unwrap(),
            json!({ "type": "auth_ok", "user": { "id": 1, "username": "fulano" } })
        );
    }

    #[test]
    fn server_auth_error_wire_format() {
        assert_eq!(
            serde_json::to_value(ServerMessage::AuthError).unwrap(),
            json!({ "type": "auth_error" })
        );
    }

    fn sample_message() -> ChatMessage {
        ChatMessage {
            id: 1,
            channel_id: 2,
            author: User {
                id: 3,
                username: "fulano".into(),
            },
            content: "oi".into(),
            created_at: "2026-11-03T12:00:00.000Z".into(),
        }
    }

    #[test]
    fn send_message_wire_format() {
        let msg = ClientMessage::SendMessage {
            channel_id: 2,
            content: "oi".into(),
        };
        assert_eq!(
            serde_json::to_value(msg).unwrap(),
            json!({ "type": "send_message", "channel_id": 2, "content": "oi" })
        );
    }

    #[test]
    fn new_message_wire_format() {
        let msg = ServerMessage::NewMessage {
            message: sample_message(),
        };
        assert_eq!(
            serde_json::to_value(msg).unwrap(),
            json!({
                "type": "new_message",
                "message": {
                    "id": 1,
                    "channel_id": 2,
                    "author": { "id": 3, "username": "fulano" },
                    "content": "oi",
                    "created_at": "2026-11-03T12:00:00.000Z",
                },
            })
        );
    }

    #[test]
    fn message_rejected_wire_format() {
        for (reason, expected) in [
            (MessageRejectReason::InvalidContent, "invalid_content"),
            (MessageRejectReason::UnknownChannel, "unknown_channel"),
            (MessageRejectReason::Internal, "internal"),
        ] {
            assert_eq!(
                serde_json::to_value(ServerMessage::MessageRejected { reason }).unwrap(),
                json!({ "type": "message_rejected", "reason": expected })
            );
        }
    }

    #[test]
    fn channel_created_wire_format() {
        let msg = ServerMessage::ChannelCreated {
            channel: TextChannel {
                id: 2,
                name: "jogos".into(),
            },
        };
        assert_eq!(
            serde_json::to_value(msg).unwrap(),
            json!({ "type": "channel_created", "channel": { "id": 2, "name": "jogos" } })
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
    fn channel_wire_format() {
        let channel = TextChannel {
            id: 1,
            name: "geral".into(),
        };
        assert_eq!(
            serde_json::to_value(channel).unwrap(),
            json!({ "id": 1, "name": "geral" })
        );
    }

    #[test]
    fn create_channel_request_wire_format() {
        let req = CreateChannelRequest {
            name: "música".into(),
        };
        assert_eq!(
            serde_json::to_value(req).unwrap(),
            json!({ "name": "música" })
        );
    }

    #[test]
    fn message_page_wire_format() {
        let page = MessagePage {
            messages: vec![sample_message()],
            has_more: true,
        };
        assert_eq!(
            serde_json::to_value(page).unwrap(),
            json!({
                "messages": [{
                    "id": 1,
                    "channel_id": 2,
                    "author": { "id": 3, "username": "fulano" },
                    "content": "oi",
                    "created_at": "2026-11-03T12:00:00.000Z",
                }],
                "has_more": true,
            })
        );
    }

    #[test]
    fn unknown_channel_error_code_wire_format() {
        assert_eq!(
            serde_json::to_value(ApiErrorCode::UnknownChannel).unwrap(),
            json!("unknown_channel")
        );
    }

    #[test]
    fn channel_error_codes_wire_format() {
        assert_eq!(
            serde_json::to_value(ApiErrorCode::Unauthorized).unwrap(),
            json!("unauthorized")
        );
        assert_eq!(
            serde_json::to_value(ApiErrorCode::InvalidChannelName).unwrap(),
            json!("invalid_channel_name")
        );
        assert_eq!(
            serde_json::to_value(ApiErrorCode::ChannelNameTaken).unwrap(),
            json!("channel_name_taken")
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
