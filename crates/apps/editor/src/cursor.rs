//! Cursor system for voxel editing
//!
//! Provides the 3D cursor that follows mouse raycast and determines
//! where voxels will be placed or removed. The cursor is a CubeBox
//! with configurable size and face alignment.
//!
//! # Coordinate System
//!
//! Cursor coordinates are origin-centric, matching CubeGrid:
//! - At scale `s`, valid coords are `[-size/2, size/2)` where `size = 1 << s`
//! - Cursor position maps directly to CubeGrid position at the same scale
//! - No world_scale conversion needed

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
            FocusMode::Far => [1.0, 1.0, 0.0],  // Yellow for placement (more visible)
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

/// Cursor coordinate in origin-centric space
///
/// Coordinates are origin-centric: `[-size/2, size/2)` where `size = 1 << scale`.
/// This matches CubeGrid's coordinate system directly.
///
/// Scale can be negative for sub-voxel precision (cursor smaller than 1 grid unit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorCoord {
    /// Position in origin-centric space (matches CubeGrid coordinates)
    pub pos: IVec3,
    /// Scale level: size = 1 << scale, coords in [-size/2, size/2)
    /// Can be negative for sub-voxel sizes.
    pub scale: i32,
}

impl Default for CursorCoord {
    fn default() -> Self {
        Self {
            pos: IVec3::ZERO,
            scale: 0, // Default to scale 0 (1x1x1 - single voxel)
        }
    }
}

impl CursorCoord {
    /// Create a new cursor coordinate
    pub fn new(pos: IVec3, scale: i32) -> Self {
        Self { pos, scale }
    }

    /// Get the grid size at this scale (1 << scale for non-negative, fraction for negative)
    pub fn size(&self) -> i32 {
        if self.scale >= 0 {
            1i32 << self.scale
        } else {
            // For negative scale, size is still 1 (minimum displayable)
            1
        }
    }

    /// Get the cursor size as a float (handles negative scales)
    pub fn size_f32(&self) -> f32 {
        2.0_f32.powi(self.scale)
    }

    /// Get voxel size as fraction of world (1.0 / size)
    pub fn voxel_fraction(&self) -> f32 {
        1.0 / self.size_f32()
    }
}

/// 3D cursor for voxel editing (CubeBox-based)
///
/// Uses origin-centric coordinates that map directly to CubeGrid.
#[derive(Debug, Clone)]
pub struct CubeCursor {
    /// Current coordinate in origin-centric space
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

    /// Create a cursor with a specific scale
    pub fn with_scale(scale: i32) -> Self {
        Self {
            coord: CursorCoord::new(IVec3::ZERO, scale),
            ..Self::default()
        }
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

    /// Increase cursor scale (larger grid, more voxels)
    pub fn increase_scale(&mut self) {
        self.coord.scale += 1;
    }

    /// Decrease cursor scale (smaller grid, fewer voxels)
    /// Scale can go negative for sub-voxel precision.
    pub fn decrease_scale(&mut self) {
        self.coord.scale -= 1;
    }

    /// Set cursor scale directly
    pub fn set_scale(&mut self, scale: i32) {
        self.coord.scale = scale;
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
    /// * `cube_scale` - The world scale of the root cube (edge size in world units)
    pub fn world_size(&self, cube_scale: f32) -> Vec3 {
        // Each voxel is cube_scale / grid_size in world units
        // Use size_f32 to handle negative scales properly
        let voxel_size = cube_scale / self.coord.size_f32();
        Vec3::new(
            self.size.x as f32 * voxel_size,
            self.size.y as f32 * voxel_size,
            self.size.z as f32 * voxel_size,
        )
    }

    /// Get cursor corner position in world space (min corner of the cursor box)
    ///
    /// Origin-centric: coord (0,0,0) is at world origin, coords map linearly
    ///
    /// # Arguments
    /// * `cube_scale` - The world scale of the root cube (edge size in world units)
    pub fn world_corner(&self, cube_scale: f32) -> Vec3 {
        // Each voxel is cube_scale / grid_size in world units
        // Origin-centric: coord 0 maps to world 0
        // Use size_f32 to handle negative scales properly
        let voxel_size = cube_scale / self.coord.size_f32();
        Vec3::new(
            self.coord.pos.x as f32 * voxel_size,
            self.coord.pos.y as f32 * voxel_size,
            self.coord.pos.z as f32 * voxel_size,
        )
    }

    /// Get cursor center position in world space
    ///
    /// # Arguments
    /// * `cube_scale` - The world scale of the root cube (edge size in world units)
    pub fn world_center(&self, cube_scale: f32) -> Vec3 {
        let corner = self.world_corner(cube_scale);
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
    /// * `voxel_coord` - Integer voxel coordinate of the hit (origin-centric)
    /// * `scale` - Scale level for the cursor
    pub fn update_from_raycast_with_face(
        &mut self,
        hit_position: Vec3,
        face: Axis,
        voxel_coord: IVec3,
        scale: i32,
    ) {
        self.valid = true;
        self.hit_face = Some(face);
        self.hit_position = hit_position;
        self.coord.scale = scale;

        let base_pos = match self.focus_mode {
            FocusMode::Near => voxel_coord,
            FocusMode::Far => voxel_coord + face.to_ivec3(),
        };

        self.coord.pos = self.calculate_aligned_position(base_pos, face);
    }

    /// Legacy: Update cursor position based on raycast result (Vec3 normal)
    pub fn update_from_raycast(
        &mut self,
        hit_position: Vec3,
        face_normal: Vec3,
        voxel_coord: IVec3,
    ) {
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

        self.update_from_raycast_with_face(hit_position, face, voxel_coord, self.coord.scale);
    }

    /// Update cursor position using the new coord selection logic
    ///
    /// This method uses the EditorHit's select_coord_at_scale to properly handle
    /// far/near mode based on whether the hit face is at a boundary of the cursor scale.
    ///
    /// # Arguments
    /// * `hit_position` - World position of the raycast hit
    /// * `face` - The face that was hit
    /// * `selected_coord` - The voxel coordinate selected by select_coord_at_scale (origin-centric)
    /// * `scale` - The scale level for the cursor
    /// * `is_boundary` - Whether the hit was at a boundary face
    pub fn update_from_selected_coord(
        &mut self,
        hit_position: Vec3,
        face: Axis,
        selected_coord: IVec3,
        scale: i32,
        is_boundary: bool,
    ) {
        self.valid = true;
        self.hit_face = Some(face);
        self.hit_position = hit_position;
        self.coord.scale = scale;

        // The selected_coord already accounts for far/near mode
        // Apply alignment based on face
        self.coord.pos = self.calculate_aligned_position(selected_coord, face);

        // Store whether this was a boundary hit for potential UI display
        let _ = is_boundary; // Currently unused but available for future use
    }

    /// Update cursor position while preserving the current scale
    ///
    /// Similar to update_from_selected_coord but does not change the cursor's scale.
    /// The selected_coord should already be calculated at the cursor's current scale.
    ///
    /// # Arguments
    /// * `hit_position` - World position of the raycast hit
    /// * `face` - The face that was hit
    /// * `selected_coord` - The voxel coordinate selected at cursor's current scale (origin-centric)
    /// * `is_boundary` - Whether the hit was at a boundary face
    pub fn update_from_selected_coord_preserve_scale(
        &mut self,
        hit_position: Vec3,
        face: Axis,
        selected_coord: IVec3,
        is_boundary: bool,
    ) {
        self.valid = true;
        self.hit_face = Some(face);
        self.hit_position = hit_position;
        // Don't change self.coord.scale - preserve current cursor scale

        // The selected_coord already accounts for far/near mode
        // Apply alignment based on face
        self.coord.pos = self.calculate_aligned_position(selected_coord, face);

        // Store whether this was a boundary hit for potential UI display
        let _ = is_boundary; // Currently unused but available for future use
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
        assert_eq!(cursor.coord.scale, 0); // Default scale is 0 (single voxel)
        assert_eq!(cursor.size, IVec3::ONE);
        assert!(!cursor.valid);
        assert_eq!(cursor.focus_mode, FocusMode::Far);
        assert_eq!(cursor.align, CursorAlign::Corner);
    }

    #[test]
    fn test_cursor_with_scale() {
        let cursor = CubeCursor::with_scale(6);
        assert_eq!(cursor.coord.scale, 6);
        assert_eq!(cursor.coord.size(), 64); // 2^6 = 64
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
        cursor.coord.pos = IVec3::new(-3, 2, 0); // Origin-centric coords
        cursor.size = IVec3::ONE;

        let (min, max) = cursor.bounds();
        assert_eq!(min, IVec3::new(-3, 2, 0));
        assert_eq!(max, IVec3::new(-2, 3, 1));
    }

    #[test]
    fn test_wireframe_colors() {
        assert_eq!(FocusMode::Near.wireframe_color(), [1.0, 0.3, 0.3]);
        assert_eq!(FocusMode::Far.wireframe_color(), [1.0, 1.0, 0.0]); // Yellow for visibility
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
        assert_eq!(coord.scale, 4);
        assert_eq!(coord.size(), 16); // 2^4 = 16
        assert_eq!(coord.voxel_fraction(), 1.0 / 16.0);

        let coord2 = CursorCoord::new(IVec3::ZERO, 0);
        assert_eq!(coord2.size(), 1); // 2^0 = 1
        assert_eq!(coord2.voxel_fraction(), 1.0);

        let coord3 = CursorCoord::new(IVec3::ZERO, 2);
        assert_eq!(coord3.size(), 4); // 2^2 = 4
    }

    #[test]
    fn test_world_corner_origin_centric() {
        let mut cursor = CubeCursor::with_scale(4); // 16x16x16 grid
        cursor.coord.pos = IVec3::ZERO;

        // At scale 4, cube_scale 16.0: each voxel is 1.0 unit
        // Coord (0,0,0) should be at world (0,0,0)
        let corner = cursor.world_corner(16.0);
        assert!((corner.x - 0.0).abs() < 0.001);
        assert!((corner.y - 0.0).abs() < 0.001);
        assert!((corner.z - 0.0).abs() < 0.001);

        // Coord (-8,0,0) should be at world (-8,0,0)
        cursor.coord.pos = IVec3::new(-8, 0, 0);
        let corner = cursor.world_corner(16.0);
        assert!((corner.x - (-8.0)).abs() < 0.001);
    }

    #[test]
    fn test_world_size() {
        let mut cursor = CubeCursor::with_scale(4); // 16x16x16 grid
        cursor.size = IVec3::splat(2);

        // At scale 4, cube_scale 16.0: each voxel is 1.0 unit
        // Size 2 voxels = 2.0 units
        let size = cursor.world_size(16.0);
        assert!((size.x - 2.0).abs() < 0.001);
        assert!((size.y - 2.0).abs() < 0.001);
        assert!((size.z - 2.0).abs() < 0.001);
    }
}
