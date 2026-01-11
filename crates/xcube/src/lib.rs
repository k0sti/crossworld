//! XCube API client library for Crossworld
//!
//! This crate provides integration with the XCube voxel model API,
//! allowing Crossworld to fetch and convert voxel models from XCube's
//! online repository.

pub mod client;
pub mod convert;
pub mod types;

pub use client::XCubeClient;
pub use types::{
    GenerationConfig, GenerationRequest, ServerStatus, XCubeError, XCubeModel, XCubeResponse,
    XCubeResult,
};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::client::XCubeClient;
    pub use crate::convert::xcube_to_csm;
    pub use crate::types::{
        GenerationConfig, GenerationRequest, ServerStatus, XCubeError, XCubeModel, XCubeResponse,
        XCubeResult,
    };
}
