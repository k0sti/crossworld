use crate::auth::AuthConfig;
use std::{env, path::PathBuf};

/// Configuration for the Crossworld server binary.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address and port the server binds to (e.g. `0.0.0.0:4443`).
    pub bind_address: String,
    /// Public URL advertised to clients (used for signature checks).
    pub public_url: String,
    /// Path to the TLS certificate in PEM format.
    pub cert_path: PathBuf,
    /// Path to the TLS private key in PEM format.
    pub key_path: PathBuf,
    /// Authentication configuration.
    pub auth: AuthConfig,
    /// World-specific configuration.
    pub world: WorldConfig,
}

impl ServerConfig {
    /// Builds a configuration from environment variables while falling back to
    /// sensible defaults that match the documentation examples.
    pub fn from_env() -> anyhow::Result<Self> {
        let data_dir = env::var("CROSSWORLD_DATA").unwrap_or_else(|_| "data".to_string());
        let bind_address = env::var("CROSSWORLD_BIND").unwrap_or_else(|_| "0.0.0.0:4443".into());
        let public_url =
            env::var("CROSSWORLD_PUBLIC_URL").unwrap_or_else(|_| "https://localhost:4443".into());
        let cert_path = env::var("CROSSWORLD_CERT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(format!("{data_dir}/cert.pem")));
        let key_path = env::var("CROSSWORLD_KEY")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(format!("{data_dir}/key.pem")));

        let auth = AuthConfig::from_env();
        let world = WorldConfig::from_env(&data_dir)?;

        Ok(Self {
            bind_address,
            public_url,
            cert_path,
            key_path,
            auth,
            world,
        })
    }

    /// Returns the URL string that handshake signatures should use.
    pub fn public_url(&self) -> &str {
        &self.public_url
    }
}

/// Configuration describing how world data is persisted and served.
#[derive(Debug, Clone)]
pub struct WorldConfig {
    pub world_id: String,
    pub world_path: PathBuf,
    pub edit_log_path: PathBuf,
    pub macro_depth: u32,
    pub micro_depth: u32,
    pub border_depth: u32,
    pub cache_capacity: usize,
}

impl WorldConfig {
    pub fn from_env(data_dir: &str) -> anyhow::Result<Self> {
        let base = PathBuf::from(data_dir);
        let world_id =
            env::var("CROSSWORLD_WORLD_ID").unwrap_or_else(|_| "crossworld-default".into());

        let world_path = env::var("CROSSWORLD_WORLD_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| base.join("world.bin"));
        let edit_log_path = env::var("CROSSWORLD_EDIT_LOG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| base.join("world.edits"));

        let macro_depth = env::var("CROSSWORLD_MACRO_DEPTH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        let micro_depth = env::var("CROSSWORLD_MICRO_DEPTH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);
        let border_depth = env::var("CROSSWORLD_BORDER_DEPTH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);
        let cache_capacity = env::var("CROSSWORLD_CACHE_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(256);

        anyhow::ensure!(macro_depth >= 1, "macro depth must be >= 1");

        Ok(Self {
            world_id,
            world_path,
            edit_log_path,
            macro_depth,
            micro_depth,
            border_depth,
            cache_capacity,
        })
    }

    pub fn max_depth(&self) -> u32 {
        self.macro_depth + self.micro_depth
    }
}
