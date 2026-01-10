use bevy::prelude::*;

/// Editor configuration resource
#[derive(Resource)]
#[allow(dead_code)]
pub struct EditorConfig {
    /// Maximum octree depth for voxel operations
    pub max_depth: u32,
    /// Whether to show grid overlay
    pub show_grid: bool,
    /// Grid size
    pub grid_size: u32,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            max_depth: 6,
            show_grid: true,
            grid_size: 16,
        }
    }
}
