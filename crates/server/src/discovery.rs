use crate::server::GameServer;
use anyhow::Result;
use nostr_sdk::prelude::*;
use std::sync::Arc;
use std::time::Duration;

pub struct DiscoveryService {
    client: Client,
    server_id: String,
    endpoint: String,
    name: String,
    region: String,
    announce_interval: Duration,
}

impl DiscoveryService {
    /// Create a new discovery service
    pub async fn new(
        relays: Vec<String>,
        server_id: String,
        endpoint: String,
        name: String,
        region: String,
        announce_interval: Duration,
    ) -> Result<Self> {
        // Generate a keypair for the server
        let keys = Keys::generate();
        let client = Client::new(keys);

        // Add relays
        for relay in relays {
            client.add_relay(&relay).await?;
        }

        // Connect to relays
        client.connect().await;

        Ok(Self {
            client,
            server_id,
            endpoint,
            name,
            region,
            announce_interval,
        })
    }

    /// Start announcement loop
    pub async fn start_announcer(self: Arc<Self>, server: Arc<GameServer>) {
        let mut interval = tokio::time::interval(self.announce_interval);

        loop {
            interval.tick().await;

            if let Err(e) = self.announce(server.clone()).await {
                tracing::error!("Failed to announce server: {}", e);
            }
        }
    }

    /// Announce server to Nostr relays
    async fn announce(&self, server: Arc<GameServer>) -> Result<()> {
        let player_count = server.player_count();
        let max_players = server.config.max_players;

        // Create server announcement as kind 30311 (Live Event)
        let content = serde_json::json!({
            "game": "crossworld",
            "endpoint": self.endpoint,
            "name": self.name,
            "region": self.region,
            "players": player_count,
            "max_players": max_players,
            "version": env!("CARGO_PKG_VERSION"),
            "features": ["webtransport", "nostr-auth"],
        })
        .to_string();

        let event = EventBuilder::new(
            Kind::LiveEvent,
            content,
        )
        .tags([
            Tag::identifier(&self.server_id),
            Tag::custom(
                TagKind::custom("g"),
                vec!["crossworld"],
            ),
            Tag::custom(
                TagKind::custom("status"),
                vec!["live"],
            ),
            Tag::custom(
                TagKind::custom("participants"),
                vec![player_count.to_string()],
            ),
        ])
        .sign(&self.client.signer().await?).await?;

        self.client.send_event(event).await?;

        tracing::info!(
            "Server announced: {} ({}/{})",
            self.name,
            player_count,
            max_players
        );

        Ok(())
    }
}
