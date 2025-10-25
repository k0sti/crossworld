use super::mesher::VoxelMesher;
use super::voxel_model::{VoxelModel, VoxelPalette};
use crate::GeometryData;
use std::collections::HashMap;

/// Cached mesh data (vertices, indices, normals, colors)
type MeshData = (Vec<f32>, Vec<u32>, Vec<f32>, Vec<f32>);

/// Manages avatar generation and caching
pub struct AvatarManager {
    base_model: VoxelModel,
    base_palette: VoxelPalette,
    user_meshes: HashMap<String, MeshData>,
}

impl AvatarManager {
    /// Create a new avatar manager with a simple humanoid base model
    pub fn new() -> Self {
        let base_model = VoxelModel::create_simple_humanoid();
        let base_palette = base_model.palette.clone();

        Self {
            base_model,
            base_palette,
            user_meshes: HashMap::new(),
        }
    }

    /// Create a new avatar manager with a custom base model
    #[allow(dead_code)]
    pub fn with_model(model: VoxelModel) -> Self {
        let base_palette = model.palette.clone();

        Self {
            base_model: model,
            base_palette,
            user_meshes: HashMap::new(),
        }
    }

    /// Generate avatar geometry for a specific user
    pub fn generate_avatar_geometry(&mut self, user_npub: &str) -> GeometryData {
        // Check cache first
        if let Some((verts, indices, norms, cols)) = self.user_meshes.get(user_npub) {
            return GeometryData::new(verts.clone(), indices.clone(), norms.clone(), cols.clone());
        }

        // Generate customized palette for this user
        let user_palette = self.base_palette.customize_for_user(user_npub);

        // Generate mesh with customized colors
        let mesher = VoxelMesher::new(&self.base_model);
        let (vertices, indices, normals, colors) = mesher.generate_mesh(&user_palette);

        // Cache for future use
        self.user_meshes.insert(
            user_npub.to_string(),
            (
                vertices.clone(),
                indices.clone(),
                normals.clone(),
                colors.clone(),
            ),
        );

        GeometryData::new(vertices, indices, normals, colors)
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.user_meshes.clear();
    }

    /// Get number of cached avatars
    pub fn cache_size(&self) -> usize {
        self.user_meshes.len()
    }

    /// Set voxel in base model
    pub fn set_voxel(&mut self, x: u8, y: u8, z: u8, color_index: u8) {
        self.base_model.set_voxel(x, y, z, color_index);
        // Clear cache to force regeneration
        self.clear_cache();
    }

    /// Remove voxel from base model
    pub fn remove_voxel(&mut self, x: u8, y: u8, z: u8) {
        self.base_model.remove_voxel(x, y, z);
        // Clear cache to force regeneration
        self.clear_cache();
    }

    /// Get the base model (for editing purposes)
    #[allow(dead_code)]
    pub fn get_base_model(&self) -> &VoxelModel {
        &self.base_model
    }

    /// Replace the entire base model
    #[allow(dead_code)]
    pub fn set_base_model(&mut self, model: VoxelModel) {
        self.base_palette = model.palette.clone();
        self.base_model = model;
        self.clear_cache();
    }
}

impl Default for AvatarManager {
    fn default() -> Self {
        Self::new()
    }
}
