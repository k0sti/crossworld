//! XCube inference server client library for Crossworld
//!
//! This crate provides integration with the XCube Python inference server,
//! allowing Crossworld to generate 3D voxel models from text prompts using
//! the XCube diffusion model.
//!
//! # Features
//!
//! - **HTTP Client**: Async HTTP client for the XCube inference server
//! - **Health Checks**: Monitor server status and model loading state
//! - **Text-to-3D Generation**: Generate point clouds from text prompts
//! - **Retry Logic**: Automatic retry with exponential backoff for transient failures
//! - **Timeout Handling**: Configurable timeouts for health checks and generation
//!
//! # Example
//!
//! ```no_run
//! use xcube::{XCubeClient, GenerationRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client
//!     let client = XCubeClient::new("http://localhost:8000");
//!
//!     // Check server health
//!     let status = client.health_check().await?;
//!     if !status.is_ready() {
//!         eprintln!("Server not ready: {}", status.status);
//!         return Ok(());
//!     }
//!
//!     // Generate 3D model from text
//!     let request = GenerationRequest::new("a wooden chair")
//!         .with_ddim_steps(50)
//!         .with_fine(false);
//!
//!     let result = client.generate(&request).await?;
//!
//!     println!("Generated {} coarse points", result.coarse_point_count());
//!     if result.has_fine() {
//!         println!("Generated {} fine points", result.fine_point_count());
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod convert;
pub mod types;

pub use client::XCubeClient;
pub use convert::{voxelize, VoxelizeConfig};
pub use types::{GenerationRequest, ServerStatus, XCubeError, XCubeResult};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::client::XCubeClient;
    pub use crate::convert::{voxelize, xcube_to_csm, VoxelizeConfig};
    pub use crate::types::{GenerationRequest, ServerStatus, XCubeError, XCubeResult};
}
