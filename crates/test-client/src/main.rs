use anyhow::{Context, Result};
use clap::Parser;
use crossworld_server::messages::{PlayerIdentity, ReliableMessage, UnreliableMessage};
use glam::Vec3;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use wtransport::ClientConfig;

#[derive(Parser, Debug)]
#[command(name = "test-client")]
#[command(about = "Test client for Crossworld game server", long_about = None)]
struct Args {
    /// Server URL
    #[arg(long, default_value = "https://127.0.0.1:4433")]
    server: String,

    /// Player name
    #[arg(long, default_value = "TestPlayer")]
    name: String,

    /// Number of position updates to send
    #[arg(long, default_value = "100")]
    updates: usize,

    /// Update rate in milliseconds
    #[arg(long, default_value = "100")]
    rate_ms: u64,

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
                .unwrap_or_else(|_| format!("test_client={}", log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Crossworld Test Client v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Connecting to: {}", args.server);

    // Configure client to accept self-signed certificates (for testing)
    // Note: In production, use proper certificate validation
    let config = ClientConfig::builder()
        .with_bind_default()
        .with_no_cert_validation()
        .build();

    // Connect to server
    let connection = wtransport::Endpoint::client(config)?
        .connect(&args.server)
        .await
        .context("Failed to connect to server")?;

    tracing::info!("Connected to server!");

    // Spawn task to receive position broadcasts
    let recv_connection = connection.clone();
    tokio::spawn(async move {
        if let Err(e) = receive_positions(recv_connection).await {
            tracing::error!("Position receiver error: {}", e);
        }
    });

    // Send Join message
    let npub = format!("npub1test{}", rand::random::<u32>());
    let join_msg = ReliableMessage::Join {
        npub: npub.clone(),
        display_name: Some(args.name.clone()),
        avatar_url: None,
        position: [0.0, 10.0, 0.0],
    };

    send_reliable_message(&connection, &join_msg).await?;
    tracing::info!("Sent Join message for player: {}", args.name);

    // Wait a bit for join to process
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Send position updates
    let mut position = Vec3::new(0.0, 10.0, 0.0);
    let velocity = Vec3::new(0.1, 0.0, 0.05);
    let mut seq = 0u32;

    tracing::info!("Sending {} position updates at {}ms intervals", args.updates, args.rate_ms);

    for i in 0..args.updates {
        // Update position (simple movement)
        position += velocity;
        seq += 1;

        // Send position update via datagram
        let pos_msg = UnreliableMessage::Position {
            x: position.x,
            y: position.y,
            z: position.z,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            rw: 1.0,
            seq,
        };

        if let Ok(data) = bincode::serialize(&pos_msg) {
            match connection.send_datagram(&data) {
                Ok(_) => {
                    if i % 10 == 0 {
                        tracing::debug!("Sent position update #{}: {:?}", seq, position);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to send datagram: {}", e);
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(args.rate_ms)).await;
    }

    tracing::info!("Finished sending {} position updates", args.updates);

    // Send Leave message
    let leave_msg = ReliableMessage::Leave { npub };
    send_reliable_message(&connection, &leave_msg).await?;
    tracing::info!("Sent Leave message");

    // Wait a bit to receive final broadcasts
    tokio::time::sleep(Duration::from_secs(2)).await;

    tracing::info!("Test client finished successfully");
    Ok(())
}

/// Send a reliable message over a bidirectional stream
async fn send_reliable_message(
    connection: &wtransport::Connection,
    msg: &ReliableMessage,
) -> Result<()> {
    let stream = connection.open_bi().await?;
    let (mut send_stream, mut recv_stream) = stream.await?;

    // Serialize and send message
    let data = bincode::serialize(msg)?;
    send_stream.write_all(&data).await?;
    send_stream.finish().await?;

    // Read response
    let mut response_data = Vec::new();
    let mut buf = [0u8; 4096];
    while let Some(n) = recv_stream.read(&mut buf).await? {
        response_data.extend_from_slice(&buf[..n]);
    }

    if !response_data.is_empty() {
        if let Ok(response) = bincode::deserialize::<ReliableMessage>(&response_data) {
            tracing::debug!("Received response: {:?}", response);
        }
    }

    Ok(())
}

/// Receive position broadcasts from server
async fn receive_positions(connection: wtransport::Connection) -> Result<()> {
    let mut received_count = 0;

    loop {
        match connection.receive_datagram().await {
            Ok(data) => {
                match bincode::deserialize::<UnreliableMessage>(&data) {
                    Ok(UnreliableMessage::Batch { positions, timestamp }) => {
                        received_count += 1;
                        if received_count % 10 == 0 {
                            tracing::info!(
                                "Received position batch with {} players (total batches: {})",
                                positions.len(),
                                received_count
                            );
                        }

                        for pos in &positions {
                            tracing::debug!(
                                "  Player {}: pos=[{:.1}, {:.1}, {:.1}]",
                                &pos.id[..8],
                                pos.pos[0],
                                pos.pos[1],
                                pos.pos[2]
                            );
                        }
                    }
                    Ok(UnreliableMessage::Pong { timestamp }) => {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64;
                        let latency = now.saturating_sub(timestamp);
                        tracing::debug!("Received Pong, latency: {}ms", latency);
                    }
                    Ok(_) => {
                        tracing::debug!("Received other unreliable message");
                    }
                    Err(e) => {
                        tracing::warn!("Failed to deserialize datagram: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Datagram receive error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
