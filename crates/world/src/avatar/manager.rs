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
}

impl Default for AvatarManager {
    fn default() -> Self {
        Self::new()
    }
}
