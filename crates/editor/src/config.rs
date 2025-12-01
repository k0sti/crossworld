use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Editor configuration
#[derive(Resource, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    /// Maximum depth of the cube (determines size: 2^max_depth)
    pub max_depth: u32,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            max_depth: 4, // 2^4 = 16x16x16
        }
    }
}

impl EditorConfig {
    /// Get the cube size (2^max_depth)
    pub fn cube_size(&self) -> u32 {
        1 << self.max_depth
    }
}
