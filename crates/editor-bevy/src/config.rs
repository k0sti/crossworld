use bevy::prelude::*;

/// Editor configuration resource
#[derive(Resource)]
pub struct EditorConfig {
    /// Maximum octree depth for voxel operations
    #[allow(dead_code)]
    pub max_depth: u32,
    /// Whether to show grid overlay
    #[allow(dead_code)]
    pub show_grid: bool,
    /// Grid size
    #[allow(dead_code)]
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
