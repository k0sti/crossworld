//! Cursor system for voxel editing
//!
//! Provides the 3D cursor that follows mouse raycast and determines
//! where voxels will be placed or removed.

use glam::{IVec3, Vec3};

/// Focus mode determines where cursor appears relative to raycast hit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusMode {
    /// Cursor at hit position (for removing voxels)
    Near,
    /// Cursor at hit position + face normal (for placing voxels)
    #[default]
    Far,
}

impl FocusMode {
    /// Toggle between Near and Far modes
    pub fn toggle(&self) -> Self {
        match self {
            FocusMode::Near => FocusMode::Far,
            FocusMode::Far => FocusMode::Near,
        }
    }

    /// Get the wireframe color for this mode
    /// Returns [r, g, b] in 0.0-1.0 range
    pub fn wireframe_color(&self) -> [f32; 3] {
        match self {
            FocusMode::Near => [1.0, 0.3, 0.3], // Red for removal
            FocusMode::Far => [0.3, 1.0, 0.3],  // Green for placement
        }
    }
}

/// 3D cursor for voxel editing
#[derive(Debug, Clone)]
pub struct CubeCursor {
    /// Current position in world space (voxel coordinates)
    pub position: Vec3,
    /// Size of cursor (1-16 voxels)
    pub size: u32,
    /// Whether cursor is currently valid (raycast hit something)
    pub valid: bool,
    /// Current focus mode (Near for removal, Far for placement)
    pub focus_mode: FocusMode,
}

impl Default for CubeCursor {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            size: 1,
            valid: false,
            focus_mode: FocusMode::default(),
        }
    }
}

impl CubeCursor {
    /// Create a new cursor with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Increase cursor size (max 16)
    pub fn increase_size(&mut self) {
        self.size = (self.size + 1).min(16);
    }

    /// Decrease cursor size (min 1)
    pub fn decrease_size(&mut self) {
        self.size = (self.size - 1).max(1);
    }

    /// Toggle between Near and Far focus modes
    pub fn toggle_mode(&mut self) {
        self.focus_mode = self.focus_mode.toggle();
    }

    /// Get the wireframe color based on current focus mode
    pub fn wireframe_color(&self) -> [f32; 3] {
        self.focus_mode.wireframe_color()
    }

    /// Get cursor bounds as min and max voxel coordinates
    pub fn bounds(&self) -> (IVec3, IVec3) {
        let half_size = (self.size / 2) as i32;
        let center = IVec3::new(
            self.position.x.floor() as i32,
            self.position.y.floor() as i32,
            self.position.z.floor() as i32,
        );

        let min = center - IVec3::splat(half_size);
        let max = center + IVec3::splat(half_size.max(1) - 1) + IVec3::ONE;

        (min, max)
    }

    /// Get cursor size as Vec3 for rendering
    pub fn render_size(&self) -> Vec3 {
        Vec3::splat(self.size as f32)
    }

    /// Get cursor center position for wireframe rendering
    /// The wireframe should be centered on the cursor position
    pub fn render_center(&self) -> Vec3 {
        let half_size = self.size as f32 / 2.0;
        self.position + Vec3::splat(half_size)
    }

    /// Update cursor position based on raycast result
    ///
    /// # Arguments
    /// * `hit_position` - World position of the raycast hit
    /// * `face_normal` - Normal of the face that was hit
    /// * `voxel_coord` - Integer voxel coordinate of the hit
    pub fn update_from_raycast(&mut self, hit_position: Vec3, face_normal: Vec3, voxel_coord: IVec3) {
        self.valid = true;
        self.position = match self.focus_mode {
            FocusMode::Near => {
                // Cursor at hit voxel (for removing)
                Vec3::new(
                    voxel_coord.x as f32,
                    voxel_coord.y as f32,
                    voxel_coord.z as f32,
                )
            }
            FocusMode::Far => {
                // Cursor at hit position + face normal (for placing)
                let placement_pos = hit_position + face_normal;
                Vec3::new(
                    placement_pos.x.floor(),
                    placement_pos.y.floor(),
                    placement_pos.z.floor(),
                )
            }
        };
    }

    /// Mark cursor as invalid (no raycast hit)
    pub fn invalidate(&mut self) {
        self.valid = false;
    }

    /// Get the voxel coordinate for editing operations
    pub fn voxel_coord(&self) -> IVec3 {
        IVec3::new(
            self.position.x.floor() as i32,
            self.position.y.floor() as i32,
            self.position.z.floor() as i32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_mode_toggle() {
        assert_eq!(FocusMode::Near.toggle(), FocusMode::Far);
        assert_eq!(FocusMode::Far.toggle(), FocusMode::Near);
    }

    #[test]
    fn test_focus_mode_default() {
        assert_eq!(FocusMode::default(), FocusMode::Far);
    }

    #[test]
    fn test_cursor_default() {
        let cursor = CubeCursor::default();
        assert_eq!(cursor.position, Vec3::ZERO);
        assert_eq!(cursor.size, 1);
        assert!(!cursor.valid);
        assert_eq!(cursor.focus_mode, FocusMode::Far);
    }

    #[test]
    fn test_cursor_size_bounds() {
        let mut cursor = CubeCursor::new();

        // Test max size
        cursor.size = 15;
        cursor.increase_size();
        assert_eq!(cursor.size, 16);
        cursor.increase_size();
        assert_eq!(cursor.size, 16); // Should stay at max

        // Test min size
        cursor.size = 2;
        cursor.decrease_size();
        assert_eq!(cursor.size, 1);
        cursor.decrease_size();
        assert_eq!(cursor.size, 1); // Should stay at min
    }

    #[test]
    fn test_cursor_toggle_mode() {
        let mut cursor = CubeCursor::new();
        assert_eq!(cursor.focus_mode, FocusMode::Far);

        cursor.toggle_mode();
        assert_eq!(cursor.focus_mode, FocusMode::Near);

        cursor.toggle_mode();
        assert_eq!(cursor.focus_mode, FocusMode::Far);
    }

    #[test]
    fn test_cursor_bounds() {
        let mut cursor = CubeCursor::new();
        cursor.position = Vec3::new(5.0, 5.0, 5.0);
        cursor.size = 1;

        let (min, max) = cursor.bounds();
        assert_eq!(min, IVec3::new(5, 5, 5));
        assert_eq!(max, IVec3::new(6, 6, 6));
    }

    #[test]
    fn test_wireframe_colors() {
        assert_eq!(FocusMode::Near.wireframe_color(), [1.0, 0.3, 0.3]);
        assert_eq!(FocusMode::Far.wireframe_color(), [0.3, 1.0, 0.3]);
    }
}
