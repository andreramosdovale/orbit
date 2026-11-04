//! Erro das rotas HTTP. Todo handler retorna `Result<T, ApiError>`.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use orbit_shared::{ApiErrorBody, ApiErrorCode};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Corpo malformado. O detalhe vem do extrator do axum e é seguro para o cliente.
    #[error("requisição inválida: {0}")]
    InvalidRequest(String),
    #[error("nome de usuário inválido")]
    InvalidUsername,
    #[error("senha inválida")]
    InvalidPassword,
    #[error("nome de usuário já existe")]
    UsernameTaken,
    #[error("usuário ou senha incorretos")]
    InvalidCredentials,
    #[error("não autenticado")]
    Unauthorized,
    #[error("nome de canal inválido")]
    InvalidChannelName,
    #[error("nome de canal já existe")]
    ChannelNameTaken,
    #[error("canal inexistente")]
    UnknownChannel,
    #[error("voz não configurada")]
    VoiceUnavailable,
    /// A causa só vai para o log; o cliente recebe uma mensagem genérica.
    #[error("erro interno")]
    Internal(#[source] BoxError),
}

impl ApiError {
    pub fn internal(err: impl Into<BoxError>) -> Self {
        Self::Internal(err.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            Self::InvalidRequest(detail) => (
                StatusCode::BAD_REQUEST,
                ApiErrorCode::InvalidRequest,
                format!("Requisição inválida: {detail}"),
            ),
            Self::InvalidUsername => (
                StatusCode::BAD_REQUEST,
                ApiErrorCode::InvalidUsername,
                "O nome de usuário deve ter de 3 a 32 caracteres: letras, números ou _.".to_owned(),
            ),
            Self::InvalidPassword => (
                StatusCode::BAD_REQUEST,
                ApiErrorCode::InvalidPassword,
                "A senha deve ter de 8 a 128 caracteres.".to_owned(),
            ),
            Self::UsernameTaken => (
                StatusCode::CONFLICT,
                ApiErrorCode::UsernameTaken,
                "Esse nome de usuário já existe.".to_owned(),
            ),
            Self::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                ApiErrorCode::InvalidCredentials,
                "Usuário ou senha incorretos.".to_owned(),
            ),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                ApiErrorCode::Unauthorized,
                "Sua sessão expirou. Entre novamente.".to_owned(),
            ),
            Self::InvalidChannelName => (
                StatusCode::BAD_REQUEST,
                ApiErrorCode::InvalidChannelName,
                "O nome do canal deve ter de 1 a 32 caracteres: letras, números, - ou _."
                    .to_owned(),
            ),
            Self::ChannelNameTaken => (
                StatusCode::CONFLICT,
                ApiErrorCode::ChannelNameTaken,
                "Já existe um canal com esse nome.".to_owned(),
            ),
            Self::UnknownChannel => (
                StatusCode::NOT_FOUND,
                ApiErrorCode::UnknownChannel,
                "Esse canal não existe mais.".to_owned(),
            ),
            Self::VoiceUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                ApiErrorCode::VoiceUnavailable,
                "A voz não está configurada neste servidor.".to_owned(),
            ),
            Self::Internal(source) => {
                tracing::error!(error = ?source, "erro interno ao processar requisição");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiErrorCode::Internal,
                    "Erro interno no servidor. Tente novamente.".to_owned(),
                )
            }
        };
        (status, Json(ApiErrorBody { code, message })).into_response()
    }
}
