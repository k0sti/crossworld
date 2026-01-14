//! Types for Roblox Cube3D API requests and responses
//!
//! This module defines the data structures used for communication with
//! the Cube3D inference server and internal processing.
//!
//! ## Occupancy Field Approach
//!
//! Instead of mesh-based conversion, this crate queries the occupancy field
//! directly from the Cube3D shape model's decoder. The occupancy field provides
//! binary occupancy values at discrete grid points, which map directly to voxels.
//!
//! Pipeline:
//! 1. Text prompt → GPT generates shape tokens
//! 2. Shape tokens → VQ-VAE decoder reconstructs latent
//! 3. Query occupancy decoder at grid points → occupancy logits
//! 4. Threshold logits → binary occupied/empty voxels
//! 5. Convert to CSM octree format

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Robocube client error types
#[derive(Debug, Error)]
pub enum RobocubeError {
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

    #[error("Invalid prompt: {0}")]
    InvalidPrompt(String),

    #[error("OBJ parse error: {0}")]
    ObjParseError(String),
}

/// Result type alias for Robocube operations
pub type Result<T> = std::result::Result<T, RobocubeError>;

/// Request parameters for Cube3D text-to-3D generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    /// Text prompt describing the desired 3D model
    pub prompt: String,

    /// Random seed for reproducibility (null for random)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,

    /// DDIM sampling steps (default: 50)
    #[serde(default = "default_ddim_steps")]
    pub ddim_steps: u32,

    /// Classifier-free guidance scale (default: 7.5)
    #[serde(default = "default_guidance_scale")]
    pub guidance_scale: f32,

    /// Bounding box aspect ratio [x, y, z] (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounding_box_xyz: Option<[f32; 3]>,

    /// Resolution base for shape decode (default: 8.0)
    #[serde(default = "default_resolution_base")]
    pub resolution_base: f32,

    /// Top-p sampling parameter (null for deterministic)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

fn default_ddim_steps() -> u32 {
    50
}

fn default_guidance_scale() -> f32 {
    7.5
}

fn default_resolution_base() -> f32 {
    8.0
}

impl Default for GenerationRequest {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            seed: None,
            ddim_steps: default_ddim_steps(),
            guidance_scale: default_guidance_scale(),
            bounding_box_xyz: None,
            resolution_base: default_resolution_base(),
            top_p: None,
        }
    }
}

impl GenerationRequest {
    /// Create a new generation request with a text prompt
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            ..Default::default()
        }
    }

    /// Set the random seed for reproducibility
    pub fn with_seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set the DDIM sampling steps
    pub fn with_ddim_steps(mut self, steps: u32) -> Self {
        self.ddim_steps = steps.clamp(1, 1000);
        self
    }

    /// Set the classifier-free guidance scale
    pub fn with_guidance_scale(mut self, scale: f32) -> Self {
        self.guidance_scale = scale.max(0.0);
        self
    }

    /// Set the bounding box aspect ratio
    pub fn with_bounding_box(mut self, bbox: [f32; 3]) -> Self {
        self.bounding_box_xyz = Some(bbox);
        self
    }

    /// Set the resolution base for shape decode
    pub fn with_resolution_base(mut self, resolution: f32) -> Self {
        self.resolution_base = resolution.max(1.0);
        self
    }

    /// Set the top-p sampling parameter (None for deterministic)
    pub fn with_top_p(mut self, top_p: Option<f32>) -> Self {
        self.top_p = top_p.map(|p| p.clamp(0.0, 1.0));
        self
    }

    /// Validate the request parameters
    pub fn validate(&self) -> Result<()> {
        if self.prompt.trim().is_empty() {
            return Err(RobocubeError::InvalidPrompt(
                "Prompt cannot be empty".to_string(),
            ));
        }
        if self.prompt.len() > 10000 {
            return Err(RobocubeError::InvalidPrompt(
                "Prompt exceeds maximum length of 10000 characters".to_string(),
            ));
        }
        Ok(())
    }
}

/// Cube3D generation result containing mesh data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobocubeResult {
    /// Mesh vertex positions [x, y, z]
    pub vertices: Vec<[f32; 3]>,

    /// Mesh triangle faces as vertex indices [v0, v1, v2]
    pub faces: Vec<[u32; 3]>,

    /// Per-vertex RGB colors (0.0-1.0 range)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertex_colors: Option<Vec<[f32; 3]>>,

    /// Per-vertex normals [nx, ny, nz]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertex_normals: Option<Vec<[f32; 3]>>,

    /// Generation metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenerationMetadata>,
}

/// Metadata from the generation process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMetadata {
    /// Time taken for generation in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_time_secs: Option<f32>,

    /// Seed used for generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_used: Option<i64>,

    /// Model version used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_version: Option<String>,
}

impl RobocubeResult {
    /// Create a new empty result
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            faces: Vec::new(),
            vertex_colors: None,
            vertex_normals: None,
            metadata: None,
        }
    }

    /// Get the number of vertices
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Get the number of faces
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

    /// Validate the mesh data
    pub fn validate(&self) -> Result<()> {
        if self.vertices.is_empty() {
            return Err(RobocubeError::ConversionError(
                "Result has no vertices".to_string(),
            ));
        }
        if self.faces.is_empty() {
            return Err(RobocubeError::ConversionError(
                "Result has no faces".to_string(),
            ));
        }

        // Validate face indices
        let vertex_count = self.vertices.len() as u32;
        for (i, face) in self.faces.iter().enumerate() {
            for &idx in face {
                if idx >= vertex_count {
                    return Err(RobocubeError::ConversionError(format!(
                        "Face {} has invalid vertex index {} (max: {})",
                        i,
                        idx,
                        vertex_count - 1
                    )));
                }
            }
        }

        // Validate colors length if present
        if let Some(colors) = &self.vertex_colors {
            if colors.len() != self.vertices.len() {
                return Err(RobocubeError::ConversionError(format!(
                    "Color count ({}) doesn't match vertex count ({})",
                    colors.len(),
                    self.vertices.len()
                )));
            }
        }

        // Validate normals length if present
        if let Some(normals) = &self.vertex_normals {
            if normals.len() != self.vertices.len() {
                return Err(RobocubeError::ConversionError(format!(
                    "Normal count ({}) doesn't match vertex count ({})",
                    normals.len(),
                    self.vertices.len()
                )));
            }
        }

        Ok(())
    }
}

impl Default for RobocubeResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check response from Cube3D server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    /// Server status: 'ready', 'loading', or 'error'
    pub status: String,

    /// Whether CUDA GPU is available
    pub gpu_available: bool,

    /// GPU device name (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_name: Option<String>,

    /// Whether Cube3D models are loaded
    pub model_loaded: bool,

    /// Model version loaded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_version: Option<String>,

    /// Error message if status is 'error'
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Server uptime in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_secs: Option<f64>,
}

impl ServerStatus {
    /// Check if the server is ready for generation requests
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

// =============================================================================
// Occupancy Field Types
// =============================================================================

/// Request for occupancy field generation
///
/// Instead of returning a mesh, this endpoint returns the raw occupancy values
/// at discrete grid points, which can be directly converted to voxels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OccupancyRequest {
    /// Text prompt describing the desired 3D model
    pub prompt: String,

    /// Random seed for reproducibility (null for random)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,

    /// Grid resolution (power of 2, e.g., 32, 64, 128)
    /// Higher values = more detail but more data
    #[serde(default = "default_grid_resolution")]
    pub grid_resolution: u32,

    /// Classifier-free guidance scale (default: 3.0)
    #[serde(default = "default_occupancy_guidance_scale")]
    pub guidance_scale: f32,

    /// Top-p sampling parameter (null for deterministic)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Bounding box aspect ratio [x, y, z] (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounding_box_xyz: Option<[f32; 3]>,

    /// Occupancy threshold (default: 0.0, values > threshold are occupied)
    #[serde(default)]
    pub threshold: f32,
}

fn default_grid_resolution() -> u32 {
    64
}

fn default_occupancy_guidance_scale() -> f32 {
    3.0
}

impl Default for OccupancyRequest {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            seed: None,
            grid_resolution: default_grid_resolution(),
            guidance_scale: default_occupancy_guidance_scale(),
            top_p: None,
            bounding_box_xyz: None,
            threshold: 0.0,
        }
    }
}

impl OccupancyRequest {
    /// Create a new occupancy request with a text prompt
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            ..Default::default()
        }
    }

    /// Set the random seed for reproducibility
    pub fn with_seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set the grid resolution (power of 2)
    pub fn with_grid_resolution(mut self, resolution: u32) -> Self {
        // Clamp to reasonable powers of 2
        self.grid_resolution = resolution.clamp(8, 256);
        self
    }

    /// Set the guidance scale
    pub fn with_guidance_scale(mut self, scale: f32) -> Self {
        self.guidance_scale = scale.max(0.0);
        self
    }

    /// Set the top-p sampling parameter
    pub fn with_top_p(mut self, top_p: Option<f32>) -> Self {
        self.top_p = top_p.map(|p| p.clamp(0.0, 1.0));
        self
    }

    /// Set the bounding box aspect ratio
    pub fn with_bounding_box(mut self, bbox: [f32; 3]) -> Self {
        self.bounding_box_xyz = Some(bbox);
        self
    }

    /// Set the occupancy threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// Validate the request parameters
    pub fn validate(&self) -> Result<()> {
        if self.prompt.trim().is_empty() {
            return Err(RobocubeError::InvalidPrompt(
                "Prompt cannot be empty".to_string(),
            ));
        }
        if self.prompt.len() > 10000 {
            return Err(RobocubeError::InvalidPrompt(
                "Prompt exceeds maximum length of 10000 characters".to_string(),
            ));
        }
        if !self.grid_resolution.is_power_of_two() {
            return Err(RobocubeError::InvalidPrompt(format!(
                "Grid resolution {} is not a power of 2",
                self.grid_resolution
            )));
        }
        Ok(())
    }
}

/// Occupancy field result from the Cube3D server
///
/// Contains the raw occupancy values or thresholded binary voxels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OccupancyResult {
    /// Grid resolution (N x N x N)
    pub resolution: u32,

    /// Bounding box minimum [x, y, z]
    pub bbox_min: [f32; 3],

    /// Bounding box maximum [x, y, z]
    pub bbox_max: [f32; 3],

    /// Occupied voxel positions as [x, y, z] indices
    /// Only voxels with occupancy > threshold are included
    pub occupied_voxels: Vec<[u32; 3]>,

    /// Optional: raw occupancy logits for all grid points
    /// Shape: resolution^3 values in row-major order (x fastest, then y, then z)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logits: Option<Vec<f32>>,

    /// Generation metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenerationMetadata>,
}

impl OccupancyResult {
    /// Get the number of occupied voxels
    pub fn occupied_count(&self) -> usize {
        self.occupied_voxels.len()
    }

    /// Get the total number of grid cells
    pub fn total_cells(&self) -> usize {
        (self.resolution as usize).pow(3)
    }

    /// Get the occupancy ratio (0.0 to 1.0)
    pub fn occupancy_ratio(&self) -> f32 {
        self.occupied_count() as f32 / self.total_cells() as f32
    }

    /// Get the bounding box dimensions
    pub fn bbox_dimensions(&self) -> [f32; 3] {
        [
            self.bbox_max[0] - self.bbox_min[0],
            self.bbox_max[1] - self.bbox_min[1],
            self.bbox_max[2] - self.bbox_min[2],
        ]
    }

    /// Check if this result has raw logits
    pub fn has_logits(&self) -> bool {
        self.logits.is_some()
    }

    /// Validate the occupancy result
    pub fn validate(&self) -> Result<()> {
        if self.resolution == 0 {
            return Err(RobocubeError::ConversionError(
                "Resolution cannot be zero".to_string(),
            ));
        }

        // Validate voxel indices are within bounds
        for (i, voxel) in self.occupied_voxels.iter().enumerate() {
            for (axis, &coord) in voxel.iter().enumerate() {
                if coord >= self.resolution {
                    let axis_name = ["x", "y", "z"][axis];
                    return Err(RobocubeError::ConversionError(format!(
                        "Voxel {} has {} coordinate {} >= resolution {}",
                        i, axis_name, coord, self.resolution
                    )));
                }
            }
        }

        // Validate logits length if present
        if let Some(logits) = &self.logits {
            let expected_len = self.total_cells();
            if logits.len() != expected_len {
                return Err(RobocubeError::ConversionError(format!(
                    "Logits length {} doesn't match expected {} (resolution^3)",
                    logits.len(),
                    expected_len
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generation_request_new() {
        let request = GenerationRequest::new("A wooden chair");
        assert_eq!(request.prompt, "A wooden chair");
        assert!(request.seed.is_none());
        assert_eq!(request.ddim_steps, 50);
        assert_eq!(request.guidance_scale, 7.5);
    }

    #[test]
    fn test_generation_request_builder() {
        let request = GenerationRequest::new("A red mushroom")
            .with_seed(42)
            .with_ddim_steps(100)
            .with_guidance_scale(10.0)
            .with_bounding_box([1.0, 2.0, 1.5])
            .with_top_p(Some(0.9));

        assert_eq!(request.prompt, "A red mushroom");
        assert_eq!(request.seed, Some(42));
        assert_eq!(request.ddim_steps, 100);
        assert_eq!(request.guidance_scale, 10.0);
        assert_eq!(request.bounding_box_xyz, Some([1.0, 2.0, 1.5]));
        assert_eq!(request.top_p, Some(0.9));
    }

    #[test]
    fn test_generation_request_clamping() {
        let request = GenerationRequest::new("test")
            .with_ddim_steps(0) // Should clamp to 1
            .with_ddim_steps(9999) // Should clamp to 1000
            .with_guidance_scale(-5.0) // Should clamp to 0.0
            .with_top_p(Some(2.0)); // Should clamp to 1.0

        // Final values after chained calls
        assert_eq!(request.ddim_steps, 1000); // Last call wins, clamped
        assert_eq!(request.guidance_scale, 0.0);
        assert_eq!(request.top_p, Some(1.0));
    }

    #[test]
    fn test_generation_request_validation() {
        // Empty prompt should fail
        let empty = GenerationRequest::new("");
        assert!(empty.validate().is_err());

        // Whitespace-only prompt should fail
        let whitespace = GenerationRequest::new("   ");
        assert!(whitespace.validate().is_err());

        // Valid prompt should succeed
        let valid = GenerationRequest::new("A chair");
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_robocube_result_validation() {
        // Empty result should fail
        let empty = RobocubeResult::new();
        assert!(empty.validate().is_err());

        // Result with vertices but no faces should fail
        let no_faces = RobocubeResult {
            vertices: vec![[0.0, 0.0, 0.0]],
            faces: vec![],
            vertex_colors: None,
            vertex_normals: None,
            metadata: None,
        };
        assert!(no_faces.validate().is_err());

        // Valid result should succeed
        let valid = RobocubeResult {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            faces: vec![[0, 1, 2]],
            vertex_colors: None,
            vertex_normals: None,
            metadata: None,
        };
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_robocube_result_invalid_face_index() {
        let result = RobocubeResult {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            faces: vec![[0, 1, 5]], // Index 5 is out of bounds
            vertex_colors: None,
            vertex_normals: None,
            metadata: None,
        };
        assert!(result.validate().is_err());
    }

    #[test]
    fn test_robocube_result_mismatched_colors() {
        let result = RobocubeResult {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            faces: vec![[0, 1, 2]],
            vertex_colors: Some(vec![[1.0, 0.0, 0.0]]), // Only 1 color for 3 vertices
            vertex_normals: None,
            metadata: None,
        };
        assert!(result.validate().is_err());
    }

    #[test]
    fn test_server_status_states() {
        let ready = ServerStatus {
            status: "ready".to_string(),
            gpu_available: true,
            gpu_name: Some("NVIDIA RTX 4090".to_string()),
            model_loaded: true,
            model_version: Some("v0.5".to_string()),
            error: None,
            uptime_secs: Some(3600.0),
        };
        assert!(ready.is_ready());
        assert!(!ready.is_loading());
        assert!(!ready.is_error());

        let loading = ServerStatus {
            status: "loading".to_string(),
            gpu_available: true,
            gpu_name: Some("NVIDIA RTX 4090".to_string()),
            model_loaded: false,
            model_version: None,
            error: None,
            uptime_secs: Some(10.0),
        };
        assert!(!loading.is_ready());
        assert!(loading.is_loading());
        assert!(!loading.is_error());

        let error = ServerStatus {
            status: "error".to_string(),
            gpu_available: false,
            gpu_name: None,
            model_loaded: false,
            model_version: None,
            error: Some("CUDA not available".to_string()),
            uptime_secs: None,
        };
        assert!(!error.is_ready());
        assert!(!error.is_loading());
        assert!(error.is_error());
    }

    #[test]
    fn test_generation_request_serialization() {
        let request = GenerationRequest::new("A wooden chair")
            .with_seed(42)
            .with_bounding_box([1.0, 1.5, 1.0]);

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"prompt\":\"A wooden chair\""));
        assert!(json.contains("\"seed\":42"));
        assert!(json.contains("\"bounding_box_xyz\""));
    }

    #[test]
    fn test_generation_request_skip_none_serialization() {
        let request = GenerationRequest::new("test");

        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("\"seed\"")); // Should be omitted
        assert!(!json.contains("\"bounding_box_xyz\"")); // Should be omitted
        assert!(!json.contains("\"top_p\"")); // Should be omitted
    }

    #[test]
    fn test_robocube_result_helpers() {
        let result = RobocubeResult {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            faces: vec![[0, 1, 2]],
            vertex_colors: Some(vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
            vertex_normals: None,
            metadata: Some(GenerationMetadata {
                generation_time_secs: Some(5.5),
                seed_used: Some(42),
                model_version: Some("v0.5".to_string()),
            }),
        };

        assert_eq!(result.vertex_count(), 3);
        assert_eq!(result.face_count(), 1);
        assert!(result.has_vertex_colors());
        assert!(!result.has_vertex_normals());
    }

    // Occupancy field tests
    #[test]
    fn test_occupancy_request_new() {
        let request = OccupancyRequest::new("A wooden chair");
        assert_eq!(request.prompt, "A wooden chair");
        assert!(request.seed.is_none());
        assert_eq!(request.grid_resolution, 64);
        assert_eq!(request.guidance_scale, 3.0);
        assert_eq!(request.threshold, 0.0);
    }

    #[test]
    fn test_occupancy_request_builder() {
        let request = OccupancyRequest::new("A red mushroom")
            .with_seed(42)
            .with_grid_resolution(128)
            .with_guidance_scale(5.0)
            .with_threshold(0.5)
            .with_bounding_box([1.0, 2.0, 1.0]);

        assert_eq!(request.prompt, "A red mushroom");
        assert_eq!(request.seed, Some(42));
        assert_eq!(request.grid_resolution, 128);
        assert_eq!(request.guidance_scale, 5.0);
        assert_eq!(request.threshold, 0.5);
        assert_eq!(request.bounding_box_xyz, Some([1.0, 2.0, 1.0]));
    }

    #[test]
    fn test_occupancy_request_validation() {
        // Empty prompt should fail
        let empty = OccupancyRequest::new("");
        assert!(empty.validate().is_err());

        // Non-power-of-2 resolution should fail
        let mut bad_res = OccupancyRequest::new("test");
        bad_res.grid_resolution = 50;
        assert!(bad_res.validate().is_err());

        // Valid request should succeed
        let valid = OccupancyRequest::new("A chair").with_grid_resolution(64);
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_occupancy_result_helpers() {
        let result = OccupancyResult {
            resolution: 32,
            bbox_min: [-1.0, -1.0, -1.0],
            bbox_max: [1.0, 1.0, 1.0],
            occupied_voxels: vec![[0, 0, 0], [1, 1, 1], [2, 2, 2]],
            logits: None,
            metadata: None,
        };

        assert_eq!(result.occupied_count(), 3);
        assert_eq!(result.total_cells(), 32768); // 32^3
        assert!((result.occupancy_ratio() - 3.0 / 32768.0).abs() < 1e-6);
        assert_eq!(result.bbox_dimensions(), [2.0, 2.0, 2.0]);
        assert!(!result.has_logits());
    }

    #[test]
    fn test_occupancy_result_validation() {
        // Valid result
        let valid = OccupancyResult {
            resolution: 32,
            bbox_min: [-1.0, -1.0, -1.0],
            bbox_max: [1.0, 1.0, 1.0],
            occupied_voxels: vec![[0, 0, 0], [31, 31, 31]],
            logits: None,
            metadata: None,
        };
        assert!(valid.validate().is_ok());

        // Out of bounds voxel should fail
        let oob = OccupancyResult {
            resolution: 32,
            bbox_min: [-1.0, -1.0, -1.0],
            bbox_max: [1.0, 1.0, 1.0],
            occupied_voxels: vec![[32, 0, 0]], // 32 is out of bounds for resolution 32
            logits: None,
            metadata: None,
        };
        assert!(oob.validate().is_err());

        // Mismatched logits length should fail
        let bad_logits = OccupancyResult {
            resolution: 4,
            bbox_min: [-1.0, -1.0, -1.0],
            bbox_max: [1.0, 1.0, 1.0],
            occupied_voxels: vec![],
            logits: Some(vec![0.0; 32]), // Should be 64 (4^3)
            metadata: None,
        };
        assert!(bad_logits.validate().is_err());
    }

    #[test]
    fn test_occupancy_result_with_logits() {
        let resolution = 4;
        let total = (resolution as usize).pow(3);
        let result = OccupancyResult {
            resolution,
            bbox_min: [-1.0, -1.0, -1.0],
            bbox_max: [1.0, 1.0, 1.0],
            occupied_voxels: vec![],
            logits: Some(vec![0.0; total]),
            metadata: None,
        };

        assert!(result.has_logits());
        assert!(result.validate().is_ok());
    }
}
