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
