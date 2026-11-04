//! Token de acesso ao LiveKit: um JWT HS256 assinado com o segredo dele (ADR 0014).

use std::fmt;

use jsonwebtoken::{Algorithm, EncodingKey, Header};
use orbit_shared::User;
use serde::Serialize;

/// Porta padrão do LiveKit.
const DEFAULT_PORT: u16 = 7880;

/// Quanto o token dura. Só precisa valer até conectar: depois, o SDK cuida da renovação.
const TOKEN_TTL_SECS: u64 = 10 * 60;

/// Só o microfone: tela e câmera ficam para as Fases 4 e 5.
const PUBLISH_SOURCES: [&str; 1] = ["microphone"];

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum LiveKitConfigError {
    #[error("LIVEKIT_API_KEY e LIVEKIT_API_SECRET precisam estar definidas juntas")]
    MissingPair,
    #[error("LIVEKIT_PORT inválida: {0:?}")]
    InvalidPort(String),
}

/// Chaves e porta do LiveKit.
#[derive(Clone)]
pub struct LiveKitConfig {
    api_key: String,
    api_secret: String,
    port: u16,
}

// O segredo nunca pode aparecer em um log.
impl fmt::Debug for LiveKitConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LiveKitConfig")
            .field("api_key", &self.api_key)
            .field("api_secret", &"<oculto>")
            .field("port", &self.port)
            .finish()
    }
}

#[derive(Serialize)]
struct Claims<'a> {
    iss: &'a str,
    sub: String,
    name: &'a str,
    nbf: u64,
    exp: u64,
    video: VideoGrant<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoGrant<'a> {
    room: String,
    room_join: bool,
    can_subscribe: bool,
    can_publish: bool,
    can_publish_data: bool,
    can_publish_sources: &'a [&'a str],
}

impl LiveKitConfig {
    /// Monta a configuração a partir das variáveis de ambiente.
    /// Sem as duas chaves, a voz fica desativada (`None`). Só uma delas é erro.
    pub fn from_values(
        api_key: Option<String>,
        api_secret: Option<String>,
        port: Option<String>,
    ) -> Result<Option<Self>, LiveKitConfigError> {
        let (api_key, api_secret) = match (api_key, api_secret) {
            (None, None) => return Ok(None),
            (Some(api_key), Some(api_secret)) if !api_key.is_empty() && !api_secret.is_empty() => {
                (api_key, api_secret)
            }
            _ => return Err(LiveKitConfigError::MissingPair),
        };
        let port = match port {
            None => DEFAULT_PORT,
            Some(value) => value
                .parse()
                .map_err(|_| LiveKitConfigError::InvalidPort(value))?,
        };
        Ok(Some(Self {
            api_key,
            api_secret,
            port,
        }))
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Token de `user` para a sala do canal `channel_id`. `now` em segundos Unix.
    pub fn issue(
        &self,
        user: &User,
        channel_id: u32,
        now: u64,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let claims = Claims {
            iss: &self.api_key,
            sub: user.id.to_string(),
            name: &user.username,
            nbf: now,
            exp: now.saturating_add(TOKEN_TTL_SECS),
            video: VideoGrant {
                room: format!("voice-{channel_id}"),
                room_join: true,
                can_subscribe: true,
                can_publish: true,
                can_publish_data: false,
                can_publish_sources: &PUBLISH_SOURCES,
            },
        };
        jsonwebtoken::encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.api_secret.as_bytes()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(text: &str) -> Option<String> {
        Some(text.to_owned())
    }

    #[test]
    fn without_keys_voice_is_disabled() {
        let config = LiveKitConfig::from_values(None, None, None);
        assert!(matches!(config, Ok(None)));
    }

    #[test]
    fn only_one_key_is_an_error() {
        for (key, secret) in [
            (value("chave"), None),
            (None, value("segredo")),
            (value(""), value("segredo")),
            (value("chave"), value("")),
        ] {
            let config = LiveKitConfig::from_values(key, secret, None);
            assert_eq!(config.err(), Some(LiveKitConfigError::MissingPair));
        }
    }

    #[test]
    fn port_defaults_and_is_validated() -> anyhow::Result<()> {
        let config = LiveKitConfig::from_values(value("c"), value("s"), None)?;
        assert_eq!(config.map(|c| c.port()), Some(7880));

        let config = LiveKitConfig::from_values(value("c"), value("s"), value("7999"))?;
        assert_eq!(config.map(|c| c.port()), Some(7999));

        let config = LiveKitConfig::from_values(value("c"), value("s"), value("abc"));
        assert_eq!(
            config.err(),
            Some(LiveKitConfigError::InvalidPort("abc".into()))
        );
        Ok(())
    }

    #[test]
    fn debug_hides_the_secret() -> anyhow::Result<()> {
        let config = LiveKitConfig::from_values(value("c"), value("segredo-real"), None)?;
        assert!(!format!("{config:?}").contains("segredo-real"));
        Ok(())
    }
}
