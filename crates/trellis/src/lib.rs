//! Trellis inference server client library for Crossworld
//!
//! This crate provides integration with the Trellis Python inference server,
//! allowing Crossworld to generate 3D voxel models from text/image prompts using
//! the Trellis diffusion model.
//!
//! # Features
//!
//! - **HTTP Client**: Async HTTP client for the Trellis inference server
//! - **Health Checks**: Monitor server status and model loading state
//! - **Text/Image-to-3D Generation**: Generate voxel models from prompts
//! - **Retry Logic**: Automatic retry with exponential backoff for transient failures
//! - **Timeout Handling**: Configurable timeouts for health checks and generation
//!
//! # Example
//!
//! ```no_run
//! use trellis::{TrellisClient, GenerationRequest, Resolution};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client
//!     let client = TrellisClient::new("http://localhost:8000");
//!
//!     // Check server health
//!     let status = client.health_check().await?;
//!     if !status.is_ready() {
//!         eprintln!("Server not ready: {}", status.status);
//!         return Ok(());
//!     }
//!
//!     // Generate 3D model from image
//!     let base64_image = "..."; // Base64-encoded image
//!     let request = GenerationRequest::new(base64_image)
//!         .with_resolution(Resolution::R1024)
//!         .with_seed(42);
//!
//!     let result = client.generate(&request).await?;
//!
//!     println!("Generated mesh with {} vertices and {} faces",
//!              result.vertex_count(), result.face_count());
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod color_quantizer;
pub mod convert;
pub mod ovoxel;
pub mod types;

pub use client::TrellisClient;
pub use color_quantizer::{quantize_colors, ColorPalette};
pub use convert::{ovoxel_to_csm, ovoxel_to_cube, ovoxel_to_cube_auto_depth, ConversionError};
pub use ovoxel::{OVoxel, OVoxelError};
pub use types::{GenerationRequest, Resolution, ServerStatus, TrellisError, TrellisResult};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::client::TrellisClient;
    pub use crate::color_quantizer::{quantize_colors, ColorPalette};
    pub use crate::convert::{ovoxel_to_csm, ovoxel_to_cube, ovoxel_to_cube_auto_depth};
    pub use crate::ovoxel::OVoxel;
    pub use crate::types::{
        GenerationRequest, Resolution, ServerStatus, TrellisError, TrellisResult,
    };
}
