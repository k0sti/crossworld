use anyhow::Result;
use tracing::{Level, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("CrossWorld Server starting...");

    // TODO: Implement WebTransport server
    // TODO: Implement game state management
    // TODO: Implement player connection handling

    info!("Server initialized (stub)");

    Ok(())
}
