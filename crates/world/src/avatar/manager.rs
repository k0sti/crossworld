use cube::{Cube, glam::Vec3};

/// Manages avatar loading from .vox files
#[allow(dead_code)]
pub struct AvatarManager {
    base_model: Cube<i32>,
}

#[allow(dead_code)]
impl AvatarManager {
    /// Create a new avatar manager with an empty cube
    pub fn new() -> Self {
        Self {
            base_model: Cube::solid(0),
        }
    }

    /// Load avatar from .vox file bytes
    pub fn load_from_vox(&mut self, bytes: &[u8]) -> Result<(), String> {
        // Use cube crate's vox loader with centered alignment
        let align = Vec3::splat(0.5); // Center the model
        self.base_model = cube::load_vox_to_cube(bytes, align)?;
        Ok(())
    }

    /// Get the base model
    pub fn get_base_model(&self) -> &Cube<i32> {
        &self.base_model
    }
}

impl Default for AvatarManager {
    fn default() -> Self {
        Self::new()
    }
}
