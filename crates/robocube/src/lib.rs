//! Robocube - Roblox Cube3D to Crossworld voxel converter
//!
//! This crate provides integration with [Roblox Cube3D](https://github.com/Roblox/cube),
//! a text-to-3D generative AI model, converting its output to Crossworld's native
//! CSM (CubeScript Model) voxel format.
//!
//! ## Occupancy Field Approach
//!
//! Unlike traditional mesh-to-voxel conversion, this crate directly queries the
//! Cube3D model's occupancy decoder at discrete grid points. This produces more
//! accurate voxel representations without the artifacts of surface voxelization.
//!
//! ## Architecture
//!
//! ```text
//! Text Prompt
//!     ↓ HTTP POST /generate_occupancy
//! Cube3D Python Server
//!     ├─ GPT generates shape tokens
//!     ├─ VQ-VAE decoder → latent representation
//!     └─ Occupancy decoder at grid points → logits
//!     ↓
//! OccupancyResult (occupied voxel positions)
//!     ↓
//! convert::occupancy_to_cube()
//!     ↓
//! Cube<u8> Octree → CSM text format
//! ```
//!
//! ## Quick Start
//!
//! ```no_run
//! use robocube::{RobocubeClient, OccupancyRequest};
//! use robocube::convert::occupancy_to_csm;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to Cube3D server
//!     let client = RobocubeClient::new("http://localhost:8642");
//!
//!     // Generate occupancy field from text prompt
//!     let request = OccupancyRequest::new("A wooden chair")
//!         .with_grid_resolution(64)
//!         .with_seed(42);
//!
//!     let result = client.generate_occupancy(&request).await?;
//!     println!("Generated {} voxels", result.occupied_count());
//!
//!     // Convert to CSM format
//!     let csm = occupancy_to_csm(&result, None)?;
//!     std::fs::write("chair.csm", csm)?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Server Setup
//!
//! The Cube3D inference requires a Python server. See `crates/robocube/server/` for
//! the FastAPI server implementation that wraps the Cube3D library.
//!
//! ```bash
//! cd crates/robocube/server
//! pip install -e .[meshlab]
//! huggingface-cli download Roblox/cube3d-v0.5 --local-dir ./model_weights
//! python server.py
//! ```

pub mod client;
pub mod convert;
pub mod types;

// Re-export main types for convenience
pub use client::{RobocubeClient, DEFAULT_SERVER_URL};
pub use convert::{
    decode_r2g3b2, encode_r2g3b2, encode_r2g3b2_u8, occupancy_to_csm, occupancy_to_cube,
    DEFAULT_MATERIAL,
};
pub use types::{
    ColorMode, GenerationMetadata, GenerationRequest, OccupancyRequest, OccupancyResult, Result,
    RobocubeError, RobocubeResult, ServerStatus,
};
