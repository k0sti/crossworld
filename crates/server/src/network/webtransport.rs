use super::{broadcast::BroadcastHub, session::ClientSession, ServerError};
use crate::{
    auth::AuthManager,
    config::ServerConfig,
    protocol::{
        generate_session_id, EditError, EditResult, Handshake, HandshakeAck, WorldData, WorldEdit,
        WorldEditAck, WorldRequest, WorldUpdate,
    },
    world::{storage::StorageBackend, WorldState},
};
use std::sync::Arc;

/// High-level server wrapper that keeps track of world state and active sessions.
pub struct WebTransportServer<B: StorageBackend> {
    config: ServerConfig,
    auth: AuthManager,
    world: Arc<WorldState<B>>,
    broadcast: BroadcastHub,
}

impl<B: StorageBackend> WebTransportServer<B> {
    pub fn new(config: ServerConfig, world: WorldState<B>) -> Self {
        let auth = AuthManager::new(config.auth.clone());
        Self {
            auth,
            world: Arc::new(world),
            broadcast: BroadcastHub::new(1024),
            config,
        }
    }

    pub async fn handle_handshake(
        &self,
        handshake: Handshake,
    ) -> Result<(HandshakeAck, ClientSession), ServerError> {
        let auth_level = self
            .auth
            .verify_handshake(self.config.public_url(), &handshake)?;

        let session_id = generate_session_id();
        let world_info = self.world.info();

        let ack = HandshakeAck {
            session_id,
            world_info,
            auth_level,
        };

        let session = ClientSession::new(session_id, handshake.npub, auth_level);
        Ok((ack, session))
    }

    pub async fn handle_world_request(
        &self,
        session: &ClientSession,
        request: WorldRequest,
    ) -> Result<WorldData, ServerError> {
        if request.session_id != session.session_id {
            return Err(ServerError::InvalidSession);
        }

        let cube = self.world.get_cube(request.coord).await?;
        let subscription_id = if request.subscribe {
            Some(session.add_subscription(request.coord).await)
        } else {
            None
        };

        Ok(WorldData {
            coord: request.coord,
            root: cube,
            subscription_id,
        })
    }

    pub async fn handle_world_edit(
        &self,
        session: &ClientSession,
        edit: WorldEdit,
    ) -> Result<WorldEditAck, ServerError> {
        if edit.session_id != session.session_id {
            return Err(ServerError::InvalidSession);
        }

        if !session.can_edit() {
            return Ok(WorldEditAck {
                transaction_id: edit.transaction_id,
                result: EditResult::Error(EditError::Unauthorized),
            });
        }

        if !session.check_rate_limit().await {
            return Ok(WorldEditAck {
                transaction_id: edit.transaction_id,
                result: EditResult::Error(EditError::QuotaExceeded),
            });
        }

        let timestamp = current_timestamp();
        let result = self
            .world
            .apply_edit(&edit.operation, &session.npub, timestamp)
            .await;

        let ack = match result {
            Ok(_) => {
                self.broadcast.publish(WorldUpdate {
                    subscription_id: 0,
                    operation: edit.operation.clone(),
                    author: session.npub.clone(),
                    timestamp,
                });

                WorldEditAck {
                    transaction_id: edit.transaction_id,
                    result: EditResult::Success,
                }
            }
            Err(err) => WorldEditAck {
                transaction_id: edit.transaction_id,
                result: EditResult::Error(EditError::ServerError(err.to_string())),
            },
        };

        Ok(ack)
    }

    pub async fn run(&self) -> Result<(), ServerError> {
        tracing::info!(
            "Crossworld server (stub) listening on {}",
            self.config.bind_address
        );
        Ok(())
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
