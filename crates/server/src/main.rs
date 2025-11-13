use crossworld_server::{
    config::ServerConfig,
    network::WebTransportServer,
    world::{storage::FileStorage, WorldState},
};
use std::fs;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = ServerConfig::from_env()?;
    prepare_directories(&config)?;

    let storage = FileStorage::new(
        config.world.world_path.clone(),
        config.world.edit_log_path.clone(),
    );
    let world_state = WorldState::load_or_default(config.world.clone(), storage)?;

    let server = WebTransportServer::new(config, world_state);
    server.run().await?;

    Ok(())
}

fn prepare_directories(config: &ServerConfig) -> anyhow::Result<()> {
    if let Some(dir) = config.world.world_path.parent() {
        fs::create_dir_all(dir)?;
    }
    if let Some(dir) = config.world.edit_log_path.parent() {
        fs::create_dir_all(dir)?;
    }
    Ok(())
}
