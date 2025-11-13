use super::{broadcast::BroadcastHub, io, session::ClientSession, ServerError};
use crate::{
    auth::AuthManager,
    config::ServerConfig,
    protocol::{
        generate_session_id, ClientMessage, EditError, EditResult, Handshake, HandshakeAck,
        ServerMessage, WorldData, WorldEdit, WorldEditAck, WorldRequest, WorldUpdate,
    },
    world::{storage::StorageBackend, WorldState},
};
use std::sync::Arc;
use tokio::sync::broadcast;
use wtransport::endpoint::endpoint_side::Server as EndpointSide;
use wtransport::endpoint::IncomingSession;
use wtransport::{
    Connection, Endpoint, Identity, RecvStream, SendStream, ServerConfig as WtConfig,
};

/// High-level server wrapper that keeps track of world state and active sessions.
pub struct WebTransportServer<B: StorageBackend> {
    endpoint: Endpoint<EndpointSide>,
    auth: AuthManager,
    world: WorldState<B>,
    broadcast: BroadcastHub,
    public_url: String,
}

impl<B: StorageBackend> WebTransportServer<B> {
    pub async fn new(config: ServerConfig, world: WorldState<B>) -> anyhow::Result<Arc<Self>> {
        let identity = Identity::load_pemfiles(&config.cert_path, &config.key_path).await?;
        let bind_addr = config.bind_address.parse()?;

        let wt_config = WtConfig::builder()
            .with_bind_address(bind_addr)
            .with_identity(identity)
            .build();

        let endpoint = Endpoint::server(wt_config)?;
        let auth = AuthManager::new(config.auth.clone());

        Ok(Arc::new(Self {
            endpoint,
            auth,
            world,
            broadcast: BroadcastHub::new(1024),
            public_url: config.public_url().to_string(),
        }))
    }

    pub async fn run(self: Arc<Self>) -> Result<(), ServerError> {
        if let Ok(addr) = self.endpoint.local_addr() {
            tracing::info!("WebTransport server listening on {}", addr);
        }

        loop {
            let incoming = self.endpoint.accept().await;
            if let Err(err) = self.clone().accept_connection(incoming).await {
                tracing::warn!("Session error: {err}");
            }
        }
    }

    async fn accept_connection(
        self: Arc<Self>,
        incoming: IncomingSession,
    ) -> Result<(), ServerError> {
        let request = incoming
            .await
            .map_err(|err| ServerError::Transport(err.to_string()))?;
        let remote = request.remote_address();
        let connection = request
            .accept()
            .await
            .map_err(|err| ServerError::Transport(err.to_string()))?;
        tracing::info!("Accepted connection from {remote:?}");

        self.process_connection(connection).await
    }

    async fn process_connection(
        self: Arc<Self>,
        connection: Connection,
    ) -> Result<(), ServerError> {
        let (mut send, mut recv) = connection
            .accept_bi()
            .await
            .map_err(|err| ServerError::Transport(err.to_string()))?;

        let handshake: Handshake = io::read_message(&mut recv).await?;
        let (ack, session) = self.handle_handshake(handshake).await?;
        io::write_message(&mut send, &ServerMessage::HandshakeAck(ack)).await?;

        let mut broadcast_rx = self.broadcast.subscribe();
        self.handle_session_messages(session, send, recv, &mut broadcast_rx)
            .await
    }

    async fn handle_handshake(
        &self,
        handshake: Handshake,
    ) -> Result<(HandshakeAck, ClientSession), ServerError> {
        let auth_level = self.auth.verify_handshake(&self.public_url, &handshake)?;

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

    async fn handle_session_messages(
        &self,
        session: ClientSession,
        mut send: SendStream,
        mut recv: RecvStream,
        broadcast_rx: &mut broadcast::Receiver<WorldUpdate>,
    ) -> Result<(), ServerError> {
        loop {
            tokio::select! {
                msg = io::read_message::<ClientMessage>(&mut recv) => {
                    match msg {
                        Ok(ClientMessage::WorldRequest(req)) => {
                            let response = self.handle_world_request(&session, req).await?;
                            io::write_message(&mut send, &ServerMessage::WorldData(response)).await?;
                        }
                        Ok(ClientMessage::WorldEdit(edit)) => {
                            let ack = self.handle_world_edit(&session, edit).await?;
                            io::write_message(&mut send, &ServerMessage::WorldEditAck(ack)).await?;
                        }
                        Ok(ClientMessage::Disconnect) => break,
                        Err(ServerError::ConnectionClosed) => break,
                        Err(err) => return Err(err),
                    }
                }
                update = broadcast_rx.recv() => {
                    match update {
                        Ok(update) => {
                            if let Some(sub_id) = session.subscription_for_operation(&update.operation).await {
                                let mut targeted = update.clone();
                                targeted.subscription_id = sub_id;
                                io::write_message(&mut send, &ServerMessage::WorldUpdate(targeted)).await?;
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_world_request(
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

    async fn handle_world_edit(
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
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
