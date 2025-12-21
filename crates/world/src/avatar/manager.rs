use cube::{load_vox_to_cubebox, Cube, CubeBox};
use glam::IVec3;

/// Manages avatar loading from .vox files
#[allow(dead_code)]
pub struct AvatarManager {
    /// The loaded avatar model with preserved bounds
    model: Option<CubeBox<u8>>,
}

#[allow(dead_code)]
impl AvatarManager {
    /// Create a new avatar manager with no avatar loaded
    pub fn new() -> Self {
        Self { model: None }
    }

    /// Load avatar from .vox file bytes
    pub fn load_from_vox(&mut self, bytes: &[u8]) -> Result<(), String> {
        // Use cube crate's vox loader with CubeBox (model at origin)
        self.model = Some(load_vox_to_cubebox(bytes)?);
        Ok(())
    }

    /// Get the loaded CubeBox if available
    pub fn get_cubebox(&self) -> Option<&CubeBox<u8>> {
        self.model.as_ref()
    }

    /// Get the base model cube if loaded
    pub fn get_base_model(&self) -> Option<&Cube<u8>> {
        self.model.as_ref().map(|cb| &cb.cube)
    }

    /// Get the avatar bounds (model size in voxels)
    pub fn get_avatar_bounds(&self) -> Option<IVec3> {
        self.model.as_ref().map(|cb| cb.size)
    }
}

impl Default for AvatarManager {
    fn default() -> Self {
        Self::new()
    }
}
