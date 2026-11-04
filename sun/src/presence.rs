//! Presença: quem tem conexão autenticada agora. Fica só em memória.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use orbit_shared::{ServerMessage, User, UserPresence};
use sqlx::SqlitePool;
use tokio::sync::broadcast;

#[derive(Debug, thiserror::Error)]
pub enum PresenceError {
    #[error("erro no banco")]
    Db(#[source] sqlx::Error),
    #[error("id de usuário fora do intervalo: {0}")]
    IdOutOfRange(i64),
}

/// Conta as conexões de cada usuário e anuncia quando alguém fica online ou offline.
#[derive(Clone)]
pub struct Presence {
    connections: Arc<Mutex<HashMap<u32, usize>>>,
    hub: broadcast::Sender<ServerMessage>,
}

impl Presence {
    pub fn new(hub: broadcast::Sender<ServerMessage>) -> Self {
        Self {
            connections: Arc::default(),
            hub,
        }
    }

    /// Conta uma conexão de `user` até o guard devolvido ser destruído.
    pub fn connect(&self, user: &User) -> PresenceGuard {
        let mut connections = self.lock();
        let count = connections.entry(user.id).or_default();
        *count += 1;
        if *count == 1 {
            tracing::info!(user_id = user.id, "usuário online");
            self.announce(ServerMessage::UserOnline { user: user.clone() });
        }
        PresenceGuard {
            presence: self.clone(),
            user: user.clone(),
        }
    }

    fn disconnect(&self, user: &User) {
        let mut connections = self.lock();
        let Some(count) = connections.get_mut(&user.id) else {
            return;
        };
        *count -= 1;
        if *count == 0 {
            connections.remove(&user.id);
            tracing::info!(user_id = user.id, "usuário offline");
            self.announce(ServerMessage::UserOffline { user: user.clone() });
        }
    }

    fn is_online(&self, user_id: u32) -> bool {
        self.lock().contains_key(&user_id)
    }

    /// Só o contador fica atrás do lock, e ele continua válido mesmo depois de um
    /// panic em outra conexão. Por isso um lock envenenado pode ser reaproveitado.
    fn lock(&self) -> MutexGuard<'_, HashMap<u32, usize>> {
        self.connections
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Chamado com o lock na mão: os anúncios saem na mesma ordem das mudanças.
    fn announce(&self, msg: ServerMessage) {
        // Só falha sem nenhuma conexão assinando, e aí não há a quem avisar.
        if self.hub.send(msg).is_err() {
            tracing::debug!("nenhuma conexão para receber o anúncio de presença");
        }
    }
}

/// Mantém uma conexão contada. Destruí-lo cobre qualquer saída, inclusive por erro.
pub struct PresenceGuard {
    presence: Presence,
    user: User,
}

impl Drop for PresenceGuard {
    fn drop(&mut self) {
        self.presence.disconnect(&self.user);
    }
}

/// Todas as contas e o estado de cada uma.
pub async fn snapshot(
    db: &SqlitePool,
    presence: &Presence,
) -> Result<Vec<UserPresence>, PresenceError> {
    let rows: Vec<(i64, String)> = sqlx::query_as("SELECT id, username FROM users ORDER BY id")
        .fetch_all(db)
        .await
        .map_err(PresenceError::Db)?;
    rows.into_iter()
        .map(|(id, username)| {
            let id = u32::try_from(id).map_err(|_| PresenceError::IdOutOfRange(id))?;
            Ok(UserPresence {
                online: presence.is_online(id),
                user: User { id, username },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(id: u32) -> User {
        User {
            id,
            username: format!("user{id}"),
        }
    }

    fn setup() -> (Presence, broadcast::Receiver<ServerMessage>) {
        let (hub, events) = broadcast::channel(16);
        (Presence::new(hub), events)
    }

    #[test]
    fn counts_connections_per_user() {
        let (presence, mut events) = setup();
        let fulano = user(1);

        let first = presence.connect(&fulano);
        assert_eq!(
            events.try_recv().ok(),
            Some(ServerMessage::UserOnline {
                user: fulano.clone()
            })
        );

        let second = presence.connect(&fulano);
        assert!(events.try_recv().is_err(), "a segunda conexão não anuncia");

        drop(first);
        assert!(events.try_recv().is_err(), "ainda resta uma conexão");
        assert!(presence.is_online(fulano.id));

        drop(second);
        assert_eq!(
            events.try_recv().ok(),
            Some(ServerMessage::UserOffline {
                user: fulano.clone()
            })
        );
        assert!(!presence.is_online(fulano.id));
    }

    #[test]
    fn users_are_independent() {
        let (presence, _events) = setup();

        let _fulano = presence.connect(&user(1));

        assert!(presence.is_online(1));
        assert!(!presence.is_online(2));
    }
}
