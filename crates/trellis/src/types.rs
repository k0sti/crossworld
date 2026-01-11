//! Types for Trellis.2 API requests and responses

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Trellis client error types
#[derive(Debug, Error)]
pub enum TrellisError {
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

    #[error("Conversion error: {0}")]
    ConversionError(String),

    #[error("Image error: {0}")]
    ImageError(String),
}

/// Result type alias for Trellis operations that may fail
pub type Result<T> = std::result::Result<T, TrellisError>;

/// Image resolution options for Trellis.2 generation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Resolution {
    #[serde(rename = "512")]
    R512,
    #[serde(rename = "1024")]
    #[default]
    R1024,
    #[serde(rename = "1536")]
    R1536,
}

impl Resolution {
    /// Get the resolution as a u32 value
    pub fn as_u32(self) -> u32 {
        match self {
            Resolution::R512 => 512,
            Resolution::R1024 => 1024,
            Resolution::R1536 => 1536,
        }
    }
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_u32())
    }
}

/// Request parameters for Trellis.2 generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    /// Base64-encoded image
    pub image: String,

    /// Random seed for reproducibility (null for random)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,

    /// Image resolution (512, 1024, or 1536)
    #[serde(default)]
    pub resolution: Resolution,

    /// Sparse structure guidance strength (default: 7.5)
    #[serde(default = "default_ss_guidance_strength")]
    pub ss_guidance_strength: f32,

    /// Sparse structure sampling steps (default: 12)
    #[serde(default = "default_ss_sampling_steps")]
    pub ss_sampling_steps: u32,

    /// Shape SLAT guidance strength (default: 3.0)
    #[serde(default = "default_shape_slat_guidance_strength")]
    pub shape_slat_guidance_strength: f32,

    /// Shape SLAT sampling steps (default: 12)
    #[serde(default = "default_shape_slat_sampling_steps")]
    pub shape_slat_sampling_steps: u32,

    /// Texture SLAT guidance strength (default: 3.0)
    #[serde(default = "default_tex_slat_guidance_strength")]
    pub tex_slat_guidance_strength: f32,

    /// Texture SLAT sampling steps (default: 12)
    #[serde(default = "default_tex_slat_sampling_steps")]
    pub tex_slat_sampling_steps: u32,
}

/// Default value functions for Serde
fn default_ss_guidance_strength() -> f32 {
    7.5
}

fn default_ss_sampling_steps() -> u32 {
    12
}

fn default_shape_slat_guidance_strength() -> f32 {
    3.0
}

fn default_shape_slat_sampling_steps() -> u32 {
    12
}

fn default_tex_slat_guidance_strength() -> f32 {
    3.0
}

fn default_tex_slat_sampling_steps() -> u32 {
    12
}

impl Default for GenerationRequest {
    fn default() -> Self {
        Self {
            image: String::new(),
            seed: None,
            resolution: Resolution::default(),
            ss_guidance_strength: default_ss_guidance_strength(),
            ss_sampling_steps: default_ss_sampling_steps(),
            shape_slat_guidance_strength: default_shape_slat_guidance_strength(),
            shape_slat_sampling_steps: default_shape_slat_sampling_steps(),
            tex_slat_guidance_strength: default_tex_slat_guidance_strength(),
            tex_slat_sampling_steps: default_tex_slat_sampling_steps(),
        }
    }
}

impl GenerationRequest {
    /// Create a new generation request with a base64-encoded image
    pub fn new(image: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            ..Default::default()
        }
    }

    /// Set the random seed
    pub fn with_seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set the image resolution
    pub fn with_resolution(mut self, resolution: Resolution) -> Self {
        self.resolution = resolution;
        self
    }

    /// Set the sparse structure guidance strength
    pub fn with_ss_guidance_strength(mut self, strength: f32) -> Self {
        self.ss_guidance_strength = strength.max(0.0);
        self
    }

    /// Set the sparse structure sampling steps
    pub fn with_ss_sampling_steps(mut self, steps: u32) -> Self {
        self.ss_sampling_steps = steps.clamp(1, 100);
        self
    }

    /// Set the shape SLAT guidance strength
    pub fn with_shape_slat_guidance_strength(mut self, strength: f32) -> Self {
        self.shape_slat_guidance_strength = strength.max(0.0);
        self
    }

    /// Set the shape SLAT sampling steps
    pub fn with_shape_slat_sampling_steps(mut self, steps: u32) -> Self {
        self.shape_slat_sampling_steps = steps.clamp(1, 100);
        self
    }

    /// Set the texture SLAT guidance strength
    pub fn with_tex_slat_guidance_strength(mut self, strength: f32) -> Self {
        self.tex_slat_guidance_strength = strength.max(0.0);
        self
    }

    /// Set the texture SLAT sampling steps
    pub fn with_tex_slat_sampling_steps(mut self, steps: u32) -> Self {
        self.tex_slat_sampling_steps = steps.clamp(1, 100);
        self
    }
}

/// Trellis.2 generation result containing mesh data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrellisResult {
    /// Mesh vertex positions (x, y, z)
    pub vertices: Vec<[f32; 3]>,

    /// Mesh triangle faces (vertex indices)
    pub faces: Vec<[u32; 3]>,

    /// Per-vertex RGB colors (0.0-1.0 range)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertex_colors: Option<Vec<[f32; 3]>>,

    /// Per-vertex normals (nx, ny, nz)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertex_normals: Option<Vec<[f32; 3]>>,

    /// Raw GLB (binary glTF) data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glb_data: Option<Vec<u8>>,
}

impl TrellisResult {
    /// Get the number of vertices in the mesh
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Get the number of faces in the mesh
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    /// Check if vertex colors are available
    pub fn has_vertex_colors(&self) -> bool {
        self.vertex_colors.is_some()
    }

    /// Check if vertex normals are available
    pub fn has_vertex_normals(&self) -> bool {
        self.vertex_normals.is_some()
    }

    /// Check if GLB data is available
    pub fn has_glb_data(&self) -> bool {
        self.glb_data.is_some()
    }

    /// Get the size of GLB data in bytes
    pub fn glb_size(&self) -> usize {
        self.glb_data.as_ref().map_or(0, |data| data.len())
    }
}

/// Health check response from Trellis server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    /// Server status: 'ready', 'loading', or 'error'
    pub status: String,

    /// Whether Trellis dependencies are available
    pub trellis_available: bool,

    /// Whether CUDA GPU is available
    pub gpu_available: bool,

    /// GPU device name (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_name: Option<String>,

    /// Whether Trellis models are loaded
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolution_values() {
        assert_eq!(Resolution::R512.as_u32(), 512);
        assert_eq!(Resolution::R1024.as_u32(), 1024);
        assert_eq!(Resolution::R1536.as_u32(), 1536);
        assert_eq!(Resolution::default().as_u32(), 1024);
    }

    #[test]
    fn test_generation_request_builder() {
        let request = GenerationRequest::new("base64data")
            .with_seed(42)
            .with_resolution(Resolution::R512)
            .with_ss_guidance_strength(5.0)
            .with_ss_sampling_steps(20);

        assert_eq!(request.image, "base64data");
        assert_eq!(request.seed, Some(42));
        assert_eq!(request.resolution, Resolution::R512);
        assert_eq!(request.ss_guidance_strength, 5.0);
        assert_eq!(request.ss_sampling_steps, 20);
    }

    #[test]
    fn test_generation_request_clamping() {
        let request = GenerationRequest::new("")
            .with_ss_guidance_strength(-1.0) // Should clamp to 0.0
            .with_ss_sampling_steps(0) // Should clamp to 1
            .with_shape_slat_sampling_steps(200); // Should clamp to 100

        assert_eq!(request.ss_guidance_strength, 0.0);
        assert_eq!(request.ss_sampling_steps, 1);
        assert_eq!(request.shape_slat_sampling_steps, 100);
    }

    #[test]
    fn test_trellis_result_helpers() {
        let result = TrellisResult {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            faces: vec![[0, 1, 2]],
            vertex_colors: Some(vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
            vertex_normals: None,
            glb_data: Some(vec![0u8; 1024]),
        };

        assert_eq!(result.vertex_count(), 3);
        assert_eq!(result.face_count(), 1);
        assert!(result.has_vertex_colors());
        assert!(!result.has_vertex_normals());
        assert!(result.has_glb_data());
        assert_eq!(result.glb_size(), 1024);
    }

    #[test]
    fn test_server_status_states() {
        let ready = ServerStatus {
            status: "ready".to_string(),
            trellis_available: true,
            gpu_available: true,
            gpu_name: Some("NVIDIA A100".to_string()),
            model_loaded: true,
            error: None,
        };
        assert!(ready.is_ready());
        assert!(!ready.is_loading());
        assert!(!ready.is_error());

        let loading = ServerStatus {
            status: "loading".to_string(),
            trellis_available: true,
            gpu_available: true,
            gpu_name: Some("NVIDIA A100".to_string()),
            model_loaded: false,
            error: None,
        };
        assert!(!loading.is_ready());
        assert!(loading.is_loading());
        assert!(!loading.is_error());

        let error = ServerStatus {
            status: "error".to_string(),
            trellis_available: false,
            gpu_available: false,
            gpu_name: None,
            model_loaded: false,
            error: Some("Failed to load models".to_string()),
        };
        assert!(!error.is_ready());
        assert!(!error.is_loading());
        assert!(error.is_error());
    }
}
