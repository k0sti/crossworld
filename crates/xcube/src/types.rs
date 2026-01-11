//! Types for XCube API responses and models

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// XCube API error types
#[derive(Debug, Error)]
pub enum XCubeError {
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

    /// Fine mesh vertex positions (x, y, z)
    pub fine_xyz: Vec<[f32; 3]>,

    /// Fine mesh vertex normals (nx, ny, nz)
    pub fine_normal: Vec<[f32; 3]>,
}

/// Request parameters for XCube generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    /// Text prompt describing the voxel model to generate
    pub prompt: String,

    /// Number of DDIM sampling steps (default: 50, range: 1-1000)
    #[serde(default = "default_ddim_steps")]
    pub ddim_steps: u32,

    /// Guidance scale for classifier-free guidance (default: 7.5, range: 1.0-20.0)
    #[serde(default = "default_guidance_scale")]
    pub guidance_scale: f32,

    /// Random seed for reproducibility (None = random)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,

    /// Number of models to generate in parallel (default: 1, range: 1-8)
    #[serde(default = "default_batch_size")]
    pub batch_size: u32,
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
    /// Server status ("healthy", "degraded", "unavailable")
    pub status: String,

    /// Server version string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Model loaded status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_loaded: Option<bool>,

    /// GPU availability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_available: Option<bool>,

    /// Additional server info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<String>,
}

// Default value functions for Serde
fn default_ddim_steps() -> u32 {
    50
}

fn default_guidance_scale() -> f32 {
    7.5
}

fn default_batch_size() -> u32 {
    1
}

fn default_timeout() -> u64 {
    300
}

impl Default for GenerationRequest {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            ddim_steps: default_ddim_steps(),
            guidance_scale: default_guidance_scale(),
            seed: None,
            batch_size: default_batch_size(),
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

    /// Set DDIM steps (builder pattern)
    pub fn with_ddim_steps(mut self, steps: u32) -> Self {
        self.ddim_steps = steps;
        self
    }

    /// Set guidance scale (builder pattern)
    pub fn with_guidance_scale(mut self, scale: f32) -> Self {
        self.guidance_scale = scale;
        self
    }

    /// Set random seed (builder pattern)
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set batch size (builder pattern)
    pub fn with_batch_size(mut self, size: u32) -> Self {
        self.batch_size = size;
        self
    }

    /// Validate request parameters
    pub fn validate(&self) -> Result<(), XCubeError> {
        if self.prompt.is_empty() {
            return Err(XCubeError::InvalidModelData(
                "Prompt cannot be empty".to_string(),
            ));
        }

        if self.ddim_steps == 0 || self.ddim_steps > 1000 {
            return Err(XCubeError::InvalidModelData(format!(
                "DDIM steps must be between 1 and 1000, got {}",
                self.ddim_steps
            )));
        }

        if self.guidance_scale < 1.0 || self.guidance_scale > 20.0 {
            return Err(XCubeError::InvalidModelData(format!(
                "Guidance scale must be between 1.0 and 20.0, got {}",
                self.guidance_scale
            )));
        }

        if self.batch_size == 0 || self.batch_size > 8 {
            return Err(XCubeError::InvalidModelData(format!(
                "Batch size must be between 1 and 8, got {}",
                self.batch_size
            )));
        }

        Ok(())
    }
}

impl XCubeResult {
    /// Validate that result has consistent data
    pub fn validate(&self) -> Result<(), XCubeError> {
        if self.coarse_xyz.len() != self.coarse_normal.len() {
            return Err(XCubeError::InvalidModelData(format!(
                "Coarse vertex count mismatch: {} positions vs {} normals",
                self.coarse_xyz.len(),
                self.coarse_normal.len()
            )));
        }

        if self.fine_xyz.len() != self.fine_normal.len() {
            return Err(XCubeError::InvalidModelData(format!(
                "Fine vertex count mismatch: {} positions vs {} normals",
                self.fine_xyz.len(),
                self.fine_normal.len()
            )));
        }

        Ok(())
    }

    /// Get number of vertices in coarse mesh
    pub fn coarse_vertex_count(&self) -> usize {
        self.coarse_xyz.len()
    }

    /// Get number of vertices in fine mesh
    pub fn fine_vertex_count(&self) -> usize {
        self.fine_xyz.len()
    }
}
