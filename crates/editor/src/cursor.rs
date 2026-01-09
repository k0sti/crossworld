//! Cursor system for voxel editing
//!
//! Provides the 3D cursor that follows mouse raycast and determines
//! where voxels will be placed or removed. The cursor is a CubeBox
//! with configurable size and face alignment.

use cube::Axis;
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

/// Alignment mode for cursor relative to hit face
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorAlign {
    /// Cursor corner touches the hit point
    #[default]
    Corner,
    /// Cursor center is at the hit point
    Center,
    /// Cursor edge is aligned with the hit face
    Edge,
}

impl CursorAlign {
    /// Cycle through alignment modes
    pub fn next(&self) -> Self {
        match self {
            CursorAlign::Corner => CursorAlign::Center,
            CursorAlign::Center => CursorAlign::Edge,
            CursorAlign::Edge => CursorAlign::Corner,
        }
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            CursorAlign::Corner => "Corner",
            CursorAlign::Center => "Center",
            CursorAlign::Edge => "Edge",
        }
    }
}

/// Cursor coordinate in octree space
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorCoord {
    /// Position in octree space
    pub pos: IVec3,
    /// Depth level (determines voxel size)
    pub depth: u32,
}

impl Default for CursorCoord {
    fn default() -> Self {
        Self {
            pos: IVec3::ZERO,
            depth: 0, // Default to 0 (whole cube)
        }
    }
}

impl CursorCoord {
    /// Create a new cursor coordinate
    pub fn new(pos: IVec3, depth: u32) -> Self {
        Self { pos, depth }
    }

    /// Get the voxel size at this depth (2^(max_depth - depth))
    pub fn voxel_size(&self, max_depth: u32) -> u32 {
        1 << (max_depth.saturating_sub(self.depth))
    }
}

/// 3D cursor for voxel editing (CubeBox-based)
#[derive(Debug, Clone)]
pub struct CubeCursor {
    /// Current coordinate in octree space
    pub coord: CursorCoord,
    /// Size of cursor in voxels (x, y, z)
    pub size: IVec3,
    /// Whether cursor is currently valid (raycast hit something)
    pub valid: bool,
    /// Current focus mode (Near for removal, Far for placement)
    pub focus_mode: FocusMode,
    /// Alignment mode for cursor placement
    pub align: CursorAlign,
    /// The face that was hit by raycast
    pub hit_face: Option<Axis>,
    /// World position of the hit point
    pub hit_position: Vec3,
}

impl Default for CubeCursor {
    fn default() -> Self {
        Self {
            coord: CursorCoord::default(),
            size: IVec3::ONE,
            valid: false,
            focus_mode: FocusMode::default(),
            align: CursorAlign::default(),
            hit_face: None,
            hit_position: Vec3::ZERO,
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
        self.size = (self.size + IVec3::ONE).min(IVec3::splat(16));
    }

    /// Decrease cursor size (min 1)
    pub fn decrease_size(&mut self) {
        self.size = (self.size - IVec3::ONE).max(IVec3::ONE);
    }

    /// Set cursor size uniformly
    pub fn set_size(&mut self, size: i32) {
        let clamped = size.clamp(1, 16);
        self.size = IVec3::splat(clamped);
    }

    /// Toggle between Near and Far focus modes
    pub fn toggle_mode(&mut self) {
        self.focus_mode = self.focus_mode.toggle();
    }

    /// Cycle through alignment modes
    pub fn cycle_align(&mut self) {
        self.align = self.align.next();
    }

    /// Increase cursor depth (finer voxels, max 7)
    pub fn increase_depth(&mut self, max_depth: u32) {
        if self.coord.depth < max_depth {
            self.coord.depth += 1;
        }
    }

    /// Decrease cursor depth (coarser voxels, min 0)
    pub fn decrease_depth(&mut self) {
        self.coord.depth = self.coord.depth.saturating_sub(1);
    }

    /// Set cursor depth directly
    pub fn set_depth(&mut self, depth: u32, max_depth: u32) {
        self.coord.depth = depth.min(max_depth);
    }

    /// Get the wireframe color based on current focus mode
    pub fn wireframe_color(&self) -> [f32; 3] {
        self.focus_mode.wireframe_color()
    }

    /// Get cursor bounds as min and max voxel coordinates
    pub fn bounds(&self) -> (IVec3, IVec3) {
        let min = self.coord.pos;
        let max = self.coord.pos + self.size;
        (min, max)
    }

    /// Get cursor size as Vec3 in voxel units (not world space)
    pub fn render_size(&self) -> Vec3 {
        Vec3::new(self.size.x as f32, self.size.y as f32, self.size.z as f32)
    }

    /// Get cursor position as Vec3 in voxel units (not world space)
    pub fn render_position(&self) -> Vec3 {
        Vec3::new(
            self.coord.pos.x as f32,
            self.coord.pos.y as f32,
            self.coord.pos.z as f32,
        )
    }

    /// Get cursor size in world space
    ///
    /// # Arguments
    /// * `cube_scale` - The world scale of the root cube (edge size)
    pub fn world_size(&self, cube_scale: f32) -> Vec3 {
        let voxel_size = cube_scale / (1 << self.coord.depth) as f32;
        Vec3::new(
            self.size.x as f32 * voxel_size,
            self.size.y as f32 * voxel_size,
            self.size.z as f32 * voxel_size,
        )
    }

    /// Get cursor corner position in world space (min corner of the cursor box)
    ///
    /// The cube is centered at cube_position, so voxel (0,0,0) corner is at cube_position - cube_scale/2
    ///
    /// # Arguments
    /// * `cube_position` - The center position of the root cube in world space
    /// * `cube_scale` - The world scale of the root cube (edge size)
    pub fn world_corner(&self, cube_position: Vec3, cube_scale: f32) -> Vec3 {
        let voxel_size = cube_scale / (1 << self.coord.depth) as f32;
        let cube_corner = cube_position - Vec3::splat(cube_scale * 0.5);
        cube_corner + Vec3::new(
            self.coord.pos.x as f32 * voxel_size,
            self.coord.pos.y as f32 * voxel_size,
            self.coord.pos.z as f32 * voxel_size,
        )
    }

    /// Get cursor center position in world space
    ///
    /// # Arguments
    /// * `cube_position` - The center position of the root cube in world space
    /// * `cube_scale` - The world scale of the root cube (edge size)
    pub fn world_center(&self, cube_position: Vec3, cube_scale: f32) -> Vec3 {
        let corner = self.world_corner(cube_position, cube_scale);
        let size = self.world_size(cube_scale);
        corner + size * 0.5
    }

    /// Calculate aligned position based on hit face and alignment mode
    fn calculate_aligned_position(&self, base_pos: IVec3, face: Axis) -> IVec3 {
        match self.align {
            CursorAlign::Corner => {
                // Corner touches hit point - offset based on face normal
                let offset = match face {
                    Axis::PosX => IVec3::new(0, 0, 0),
                    Axis::NegX => IVec3::new(1 - self.size.x, 0, 0),
                    Axis::PosY => IVec3::new(0, 0, 0),
                    Axis::NegY => IVec3::new(0, 1 - self.size.y, 0),
                    Axis::PosZ => IVec3::new(0, 0, 0),
                    Axis::NegZ => IVec3::new(0, 0, 1 - self.size.z),
                };
                base_pos + offset
            }
            CursorAlign::Center => {
                // Center at hit point
                base_pos - self.size / 2
            }
            CursorAlign::Edge => {
                // Edge aligned with hit face
                let half = self.size / 2;
                match face {
                    Axis::PosX | Axis::NegX => {
                        IVec3::new(base_pos.x, base_pos.y - half.y, base_pos.z - half.z)
                    }
                    Axis::PosY | Axis::NegY => {
                        IVec3::new(base_pos.x - half.x, base_pos.y, base_pos.z - half.z)
                    }
                    Axis::PosZ | Axis::NegZ => {
                        IVec3::new(base_pos.x - half.x, base_pos.y - half.y, base_pos.z)
                    }
                }
            }
        }
    }

    /// Update cursor position based on raycast result
    ///
    /// # Arguments
    /// * `hit_position` - World position of the raycast hit
    /// * `face` - The face that was hit
    /// * `voxel_coord` - Integer voxel coordinate of the hit
    /// * `depth` - Depth level for the cursor
    pub fn update_from_raycast_with_face(
        &mut self,
        hit_position: Vec3,
        face: Axis,
        voxel_coord: IVec3,
        depth: u32,
    ) {
        self.valid = true;
        self.hit_face = Some(face);
        self.hit_position = hit_position;
        self.coord.depth = depth;

        let base_pos = match self.focus_mode {
            FocusMode::Near => voxel_coord,
            FocusMode::Far => voxel_coord + face.to_ivec3(),
        };

        self.coord.pos = self.calculate_aligned_position(base_pos, face);
    }

    /// Legacy: Update cursor position based on raycast result (Vec3 normal)
    pub fn update_from_raycast(&mut self, hit_position: Vec3, face_normal: Vec3, voxel_coord: IVec3) {
        // Convert Vec3 normal to Axis
        let face = if face_normal.x > 0.5 {
            Axis::PosX
        } else if face_normal.x < -0.5 {
            Axis::NegX
        } else if face_normal.y > 0.5 {
            Axis::PosY
        } else if face_normal.y < -0.5 {
            Axis::NegY
        } else if face_normal.z > 0.5 {
            Axis::PosZ
        } else {
            Axis::NegZ
        };

        self.update_from_raycast_with_face(hit_position, face, voxel_coord, self.coord.depth);
    }

    /// Mark cursor as invalid (no raycast hit)
    pub fn invalidate(&mut self) {
        self.valid = false;
        self.hit_face = None;
    }

    /// Get the voxel coordinate for editing operations
    pub fn voxel_coord(&self) -> IVec3 {
        self.coord.pos
    }

    /// Legacy: Get position as Vec3
    pub fn position(&self) -> Vec3 {
        self.render_position()
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
        assert_eq!(cursor.coord.pos, IVec3::ZERO);
        assert_eq!(cursor.size, IVec3::ONE);
        assert!(!cursor.valid);
        assert_eq!(cursor.focus_mode, FocusMode::Far);
        assert_eq!(cursor.align, CursorAlign::Corner);
    }

    #[test]
    fn test_cursor_size_bounds() {
        let mut cursor = CubeCursor::new();

        // Test max size
        cursor.size = IVec3::splat(15);
        cursor.increase_size();
        assert_eq!(cursor.size, IVec3::splat(16));
        cursor.increase_size();
        assert_eq!(cursor.size, IVec3::splat(16)); // Should stay at max

        // Test min size
        cursor.size = IVec3::splat(2);
        cursor.decrease_size();
        assert_eq!(cursor.size, IVec3::ONE);
        cursor.decrease_size();
        assert_eq!(cursor.size, IVec3::ONE); // Should stay at min
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
        cursor.coord.pos = IVec3::new(5, 5, 5);
        cursor.size = IVec3::ONE;

        let (min, max) = cursor.bounds();
        assert_eq!(min, IVec3::new(5, 5, 5));
        assert_eq!(max, IVec3::new(6, 6, 6));
    }

    #[test]
    fn test_wireframe_colors() {
        assert_eq!(FocusMode::Near.wireframe_color(), [1.0, 0.3, 0.3]);
        assert_eq!(FocusMode::Far.wireframe_color(), [0.3, 1.0, 0.3]);
    }

    #[test]
    fn test_cursor_align_cycle() {
        assert_eq!(CursorAlign::Corner.next(), CursorAlign::Center);
        assert_eq!(CursorAlign::Center.next(), CursorAlign::Edge);
        assert_eq!(CursorAlign::Edge.next(), CursorAlign::Corner);
    }

    #[test]
    fn test_cursor_coord() {
        let coord = CursorCoord::new(IVec3::new(1, 2, 3), 4);
        assert_eq!(coord.pos, IVec3::new(1, 2, 3));
        assert_eq!(coord.depth, 4);
        assert_eq!(coord.voxel_size(4), 1);
        assert_eq!(coord.voxel_size(5), 2);
    }
}
