mod broadcast;
mod connection;
mod discovery;
mod messages;
mod metrics;
mod server;

use anyhow::{Context, Result};
use clap::Parser;
use dashmap::DashMap;
use server::{GameServer, ServerConfig};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use wtransport::{Endpoint, Identity, ServerConfig as WtServerConfig};

#[derive(Parser, Debug)]
#[command(name = "crossworld-server")]
#[command(about = "Crossworld Game Server", long_about = None)]
struct Args {
    /// Bind address
    #[arg(long, default_value = "127.0.0.1:4433")]
    bind: String,

    /// Certificate path (PEM format)
    #[arg(long, default_value = "localhost.pem")]
    cert: String,

    /// Private key path (PEM format)
    #[arg(long, default_value = "localhost-key.pem")]
    key: String,

    /// Maximum players
    #[arg(long, default_value = "10")]
    max_players: usize,

    /// World size (units)
    #[arg(long, default_value = "1000.0")]
    world_size: f32,

    /// Position broadcast rate (Hz)
    #[arg(long, default_value = "30")]
    broadcast_rate: u32,

    /// Interest management radius (0 = disabled)
    #[arg(long, default_value = "0.0")]
    interest_radius: f32,

    /// Maximum visible players
    #[arg(long, default_value = "100")]
    max_visible_players: usize,

    /// Enable anti-cheat position validation
    #[arg(long, default_value = "false")]
    validate_positions: bool,

    /// Maximum movement speed (units/sec)
    #[arg(long, default_value = "20.0")]
    max_move_speed: f32,

    /// Teleport detection threshold (units)
    #[arg(long, default_value = "50.0")]
    teleport_threshold: f32,

    /// Enable Nostr discovery announcements
    #[arg(long, default_value = "false")]
    enable_discovery: bool,

    /// Nostr relays for discovery (comma-separated)
    #[arg(long, value_delimiter = ',')]
    relays: Vec<String>,

    /// Server name
    #[arg(long, default_value = "Crossworld Dev Server")]
    server_name: String,

    /// Server region
    #[arg(long, default_value = "local")]
    server_region: String,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let log_level = args.log_level.parse().unwrap_or(tracing::Level::INFO);
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("crossworld_server={}", log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Crossworld Server v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Bind address: {}", args.bind);
    tracing::info!("Max players: {}", args.max_players);
    tracing::info!("Certificate: {}", args.cert);

    // Create server configuration
    let config = ServerConfig {
        bind_address: args.bind.clone(),
        cert_path: args.cert.clone(),
        key_path: args.key.clone(),
        max_players: args.max_players,
        world_size: args.world_size,
        tick_rate: 60,
        position_broadcast_rate: args.broadcast_rate,
        interest_radius: args.interest_radius,
        max_visible_players: args.max_visible_players,
        max_move_speed: args.max_move_speed,
        position_validation: args.validate_positions,
        teleport_threshold: args.teleport_threshold,
        enable_discovery: args.enable_discovery,
        nostr_relays: args.relays.clone(),
        announce_interval: Duration::from_secs(60),
        server_name: args.server_name.clone(),
        server_region: args.server_region.clone(),
    };

    // Create game server
    let server = Arc::new(GameServer::new(config));

    // Track active connections
    let connections: Arc<DashMap<String, Arc<wtransport::Connection>>> = Arc::new(DashMap::new());

    // Start position broadcaster
    let broadcaster_server = server.clone();
    let broadcaster_connections = connections.clone();
    tokio::spawn(async move {
        GameServer::start_position_broadcaster(broadcaster_server, broadcaster_connections).await;
    });

    // Start metrics reporter
    let metrics = server.metrics.clone();
    tokio::spawn(async move {
        metrics::start_metrics_reporter(metrics).await;
    });

    // Start discovery service if enabled
    if server.config.enable_discovery && !server.config.nostr_relays.is_empty() {
        tracing::info!("Starting Nostr discovery service");
        match discovery::DiscoveryService::new(
            server.config.nostr_relays.clone(),
            "crossworld-server".to_string(),
            args.bind.clone(),
            server.config.server_name.clone(),
            server.config.server_region.clone(),
            server.config.announce_interval,
        )
        .await
        {
            Ok(discovery) => {
                let discovery = Arc::new(discovery);
                let discovery_server = server.clone();
                tokio::spawn(async move {
                    discovery.start_announcer(discovery_server).await;
                });
            }
            Err(e) => {
                tracing::error!("Failed to start discovery service: {}", e);
            }
        }
    }

    // Load TLS certificate and key
    let cert_path = PathBuf::from(&args.cert);
    let key_path = PathBuf::from(&args.key);

    if !cert_path.exists() {
        tracing::error!("Certificate file not found: {}", args.cert);
        tracing::info!("Generate a self-signed certificate with:");
        tracing::info!("  openssl req -x509 -newkey rsa:4096 -keyout localhost-key.pem -out localhost.pem -days 365 -nodes -subj '/CN=localhost'");
        return Err(anyhow::anyhow!("Certificate file not found"));
    }

    if !key_path.exists() {
        tracing::error!("Private key file not found: {}", args.key);
        return Err(anyhow::anyhow!("Private key file not found"));
    }

    let identity = Identity::load_pemfiles(&cert_path, &key_path)
        .await
        .context("Failed to load certificate and key")?;

    // Configure WebTransport endpoint
    let wt_config = WtServerConfig::builder()
        .with_bind_address(args.bind.parse()?)
        .with_identity(identity)
        .build();

    let endpoint = Endpoint::server(wt_config)?;

    tracing::info!("Server listening on {}", args.bind);
    tracing::info!("Waiting for connections...");

    // Accept connections
    loop {
        let session = endpoint.accept().await;

        let server = server.clone();
        let _connections = connections.clone();

        tokio::spawn(async move {
            if let Err(e) = GameServer::handle_connection(server, session).await {
                tracing::error!("Connection error: {}", e);
            }
        });
    }
}
