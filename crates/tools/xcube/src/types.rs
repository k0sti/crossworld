//! Types for XCube API responses and models

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// XCube client error types
#[derive(Debug, Error)]
pub enum XCubeError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Request timeout after {0}s")]
    TimeoutError(u64),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Models not loaded on server")]
    ModelsNotLoaded,

    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Invalid model data: {0}")]
    InvalidModelData(String),

    #[error("Conversion error: {0}")]
    ConversionError(String),
}

/// XCube API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XCubeResponse {
    pub success: bool,
    pub data: Option<XCubeModel>,
    pub error: Option<String>,
}

/// XCube voxel model representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XCubeModel {
    pub id: String,
    pub name: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub voxels: Vec<Voxel>,
    pub dimensions: Dimensions,
    pub palette: Option<Vec<Color>>,
}

/// Individual voxel in the model
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Voxel {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub color_index: u8,
}

/// Model dimensions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

/// RGB color representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

// XCube Python Inference Server Types

/// XCube generation result from Python inference server
/// Contains coarse and fine mesh data with positions and normals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XCubeResult {
    /// Coarse mesh vertex positions (x, y, z)
    pub coarse_xyz: Vec<[f32; 3]>,

    /// Coarse mesh vertex normals (nx, ny, nz)
    pub coarse_normal: Vec<[f32; 3]>,

    /// Fine mesh vertex positions (x, y, z) - only present if use_fine=true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fine_xyz: Option<Vec<[f32; 3]>>,

    /// Fine mesh vertex normals (nx, ny, nz) - only present if use_fine=true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fine_normal: Option<Vec<[f32; 3]>>,
}

impl XCubeResult {
    /// Get the number of points in the coarse point cloud
    pub fn coarse_point_count(&self) -> usize {
        self.coarse_xyz.len()
    }

    /// Get the number of points in the fine point cloud
    pub fn fine_point_count(&self) -> usize {
        self.fine_xyz.as_ref().map_or(0, |xyz| xyz.len())
    }

    /// Check if fine-resolution data is available
    pub fn has_fine(&self) -> bool {
        self.fine_xyz.is_some()
    }
}

/// Request parameters for XCube generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    /// Text prompt describing the 3D object to generate
    pub prompt: String,

    /// Number of DDIM diffusion steps (1-1000, default: 100)
    #[serde(default = "default_ddim_steps")]
    pub ddim_steps: u32,

    /// Classifier-free guidance scale (1.0-20.0, default: 7.5)
    #[serde(default = "default_guidance_scale")]
    pub guidance_scale: f32,

    /// Random seed for reproducibility (null for random)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,

    /// Use fine-resolution model (slower but higher quality, default: true)
    #[serde(default = "default_use_fine")]
    pub use_fine: bool,
}

/// Configuration for XCube inference server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    /// Server URL (e.g., "http://localhost:8000")
    pub server_url: String,

    /// Request timeout in seconds (default: 300 = 5 minutes)
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Default DDIM steps if not specified in request
    #[serde(default = "default_ddim_steps")]
    pub default_ddim_steps: u32,

    /// Default guidance scale if not specified in request
    #[serde(default = "default_guidance_scale")]
    pub default_guidance_scale: f32,

    /// Default batch size if not specified in request
    #[serde(default = "default_batch_size")]
    pub default_batch_size: u32,
}

/// Health check response from XCube server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    /// Server status: 'ready', 'loading', or 'error'
    pub status: String,

    /// Whether XCube dependencies are available
    pub xcube_available: bool,

    /// Whether CUDA GPU is available
    pub gpu_available: bool,

    /// GPU device name (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_name: Option<String>,

    /// Whether XCube models are loaded
    pub model_loaded: bool,

    /// Error message if status is 'error'
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ServerStatus {
    /// Check if the server is ready to handle generation requests
    pub fn is_ready(&self) -> bool {
        self.status == "ready" && self.model_loaded
    }

    /// Check if the server is still loading models
    pub fn is_loading(&self) -> bool {
        self.status == "loading"
    }

    /// Check if the server encountered an error
    pub fn is_error(&self) -> bool {
        self.status == "error"
    }
}

// Default value functions for Serde
fn default_ddim_steps() -> u32 {
    100
}

fn default_guidance_scale() -> f32 {
    7.5
}

fn default_use_fine() -> bool {
    true
}

fn default_timeout() -> u64 {
    300
}

fn default_batch_size() -> u32 {
    1
}

impl Default for GenerationRequest {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            ddim_steps: 100,
            guidance_scale: 7.5,
            seed: None,
            use_fine: true,
        }
    }
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:8000".to_string(),
            timeout_secs: default_timeout(),
            default_ddim_steps: default_ddim_steps(),
            default_guidance_scale: default_guidance_scale(),
            default_batch_size: default_batch_size(),
        }
    }
}

impl GenerationRequest {
    /// Create a new generation request with a prompt
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            ..Default::default()
        }
    }

    /// Set the number of DDIM steps
    pub fn with_ddim_steps(mut self, steps: u32) -> Self {
        self.ddim_steps = steps.clamp(1, 1000);
        self
    }

    /// Set the guidance scale
    pub fn with_guidance_scale(mut self, scale: f32) -> Self {
        self.guidance_scale = scale.clamp(1.0, 20.0);
        self
    }

    /// Set the random seed
    pub fn with_seed(mut self, seed: i32) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Enable or disable fine-resolution generation
    pub fn with_fine(mut self, use_fine: bool) -> Self {
        self.use_fine = use_fine;
        self
    }
}
