//! CubeGrid - An infinite-size grid of cubes with automatic expansion
//!
//! CubeGrid wraps a root Cube with a scale parameter that indicates the cube's
//! edge length as 2^scale. When voxels are placed outside the current bounds,
//! the grid automatically expands to accommodate them.

use super::Cube;
use glam::IVec3;
use std::rc::Rc;

/// An infinite-size grid of cubes with automatic expansion.
///
/// The grid contains a root cube and a scale parameter. The cube's edge length
/// is `2^scale` units. When `set_cube` is called with coordinates outside the
/// current bounds, the grid expands by wrapping the root cube with empty borders,
/// incrementing the scale.
///
/// # Coordinate System
///
/// Coordinates are in corner-based space `[0, 2^(depth + scale))` where:
/// - `depth` is the internal octree depth (typically EDIT_DEPTH = 4 for 16x16x16 voxels)
/// - `scale` is the grid's scale level
///
/// # Example
///
/// ```
/// use cube::CubeGrid;
/// use glam::IVec3;
///
/// // Create a new grid with a solid cube
/// let mut grid = CubeGrid::new();
///
/// // Place a voxel at (0, 0, 0) at depth 4
/// grid = grid.set_cube(IVec3::new(0, 0, 0), 4, 42);
///
/// // Place a voxel outside bounds - grid will automatically expand
/// grid = grid.set_cube(IVec3::new(20, 0, 0), 4, 43);
/// assert!(grid.scale() > 0); // Scale increased due to expansion
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CubeGrid {
    /// The root cube data
    root: Rc<Cube<u8>>,
    /// Scale parameter: cube edge length is 2^scale
    scale: u32,
}

impl Default for CubeGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl CubeGrid {
    /// Create a new CubeGrid with an empty (solid 0) root cube at scale 0.
    pub fn new() -> Self {
        Self {
            root: Rc::new(Cube::Solid(0)),
            scale: 0,
        }
    }

    /// Create a new CubeGrid with a solid material at scale 0.
    pub fn solid(material: u8) -> Self {
        Self {
            root: Rc::new(Cube::Solid(material)),
            scale: 0,
        }
    }

    /// Create a CubeGrid from an existing cube at scale 0.
    pub fn from_cube(cube: Cube<u8>) -> Self {
        Self {
            root: Rc::new(cube),
            scale: 0,
        }
    }

    /// Create a CubeGrid from an existing cube with a specific scale.
    pub fn from_cube_with_scale(cube: Cube<u8>, scale: u32) -> Self {
        Self {
            root: Rc::new(cube),
            scale,
        }
    }

    /// Get a reference to the root cube.
    pub fn root(&self) -> &Cube<u8> {
        &self.root
    }

    /// Get an Rc reference to the root cube.
    pub fn root_rc(&self) -> Rc<Cube<u8>> {
        Rc::clone(&self.root)
    }

    /// Get the current scale. Cube edge length is 2^scale.
    pub fn scale(&self) -> u32 {
        self.scale
    }

    /// Get the cube scale factor (2^scale).
    pub fn scale_factor(&self) -> f32 {
        (1u32 << self.scale) as f32
    }

    /// Get the effective depth for a given base depth.
    ///
    /// The effective depth is `base_depth + 2 * scale` because each expansion
    /// adds 2 octree levels.
    pub fn effective_depth(&self, base_depth: u32) -> u32 {
        base_depth + 2 * self.scale
    }

    /// Get the maximum coordinate value at the effective depth.
    ///
    /// For a given base_depth, coordinates are valid in `[0, 2^effective_depth)`.
    pub fn max_coord(&self, base_depth: u32) -> i32 {
        1i32 << self.effective_depth(base_depth)
    }

    /// Check if a coordinate is within bounds at the effective depth.
    pub fn in_bounds(&self, pos: IVec3, base_depth: u32) -> bool {
        let max = self.max_coord(base_depth);
        pos.x >= 0 && pos.x < max && pos.y >= 0 && pos.y < max && pos.z >= 0 && pos.z < max
    }

    /// Set a voxel at the given position and depth, expanding the grid if needed.
    ///
    /// If the position is outside the current bounds, the grid will be expanded
    /// by calling `Cube::expand_once` with empty (material 0) borders. This adds
    /// 2 octree levels and increments the scale by 1.
    ///
    /// # Arguments
    /// * `pos` - Position in corner-based coordinates `[0, 2^effective_depth)`
    /// * `base_depth` - Base octree depth (e.g., EDIT_DEPTH = 4)
    /// * `material` - Material value to set (0-255)
    ///
    /// # Returns
    /// A new CubeGrid with the voxel set (immutable operation)
    pub fn set_cube(self, pos: IVec3, base_depth: u32, material: u8) -> Self {
        let effective_depth = self.effective_depth(base_depth);
        let max_coord = 1i32 << effective_depth;

        // Check if position is within current bounds
        let in_bounds = pos.x >= 0
            && pos.x < max_coord
            && pos.y >= 0
            && pos.y < max_coord
            && pos.z >= 0
            && pos.z < max_coord;

        if in_bounds {
            // Normal case: position is within bounds
            let new_root = self
                .root
                .set_voxel(pos.x, pos.y, pos.z, effective_depth, material);
            Self {
                root: Rc::new(new_root),
                scale: self.scale,
            }
        } else {
            // Position is outside bounds - expand the grid
            self.expand_and_set(pos, base_depth, material)
        }
    }

    /// Expand the grid and set a voxel at the given position.
    ///
    /// This method handles the case where the position is outside current bounds.
    /// It will expand the grid one or more times until the position fits.
    fn expand_and_set(self, pos: IVec3, base_depth: u32, material: u8) -> Self {
        let mut current = self;
        let mut current_pos = pos;

        // Expand until the position is within bounds
        loop {
            let effective_depth = current.effective_depth(base_depth);
            let max_coord = 1i32 << effective_depth;

            // Check if we need to expand
            let in_bounds = current_pos.x >= 0
                && current_pos.x < max_coord
                && current_pos.y >= 0
                && current_pos.y < max_coord
                && current_pos.z >= 0
                && current_pos.z < max_coord;

            if in_bounds {
                // Position is now within bounds - set the voxel
                let new_root = current.root.set_voxel(
                    current_pos.x,
                    current_pos.y,
                    current_pos.z,
                    effective_depth,
                    material,
                );
                return Self {
                    root: Rc::new(new_root),
                    scale: current.scale,
                };
            }

            // Expand the grid: wrap with empty borders
            // expand_once creates a 4x4x4 grid (2 octree levels) with original in center 2x2x2
            let expanded = Cube::expand_once(&current.root, [0, 0, 0, 0]);

            // After expansion, coordinates shift by 2^effective_depth to center the original
            // New coordinate space is [0, 2^(effective_depth+2))
            // Original [0, 2^effective_depth) maps to [2^effective_depth, 2^(effective_depth+1))
            let offset = 1i32 << effective_depth;
            current_pos += IVec3::splat(offset);

            current = Self {
                root: Rc::new(expanded),
                scale: current.scale + 1,
            };
        }
    }

    /// Get a voxel material at the given position.
    ///
    /// Returns 0 (empty) if the position is outside bounds.
    pub fn get_cube(&self, pos: IVec3, base_depth: u32) -> u8 {
        let effective_depth = self.effective_depth(base_depth);
        let max_coord = 1i32 << effective_depth;

        // Check bounds
        if pos.x < 0
            || pos.x >= max_coord
            || pos.y < 0
            || pos.y >= max_coord
            || pos.z < 0
            || pos.z >= max_coord
        {
            return 0;
        }

        self.root.get_id(effective_depth, pos)
    }

    /// Set the scale directly (used for loading saved grids).
    ///
    /// Note: This does not modify the cube data. Use with caution.
    pub fn with_scale(mut self, scale: u32) -> Self {
        self.scale = scale;
        self
    }

    /// Replace the root cube (used for loading saved grids).
    pub fn with_root(mut self, root: Cube<u8>) -> Self {
        self.root = Rc::new(root);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_grid() {
        let grid = CubeGrid::new();
        assert_eq!(grid.scale(), 0);
        assert_eq!(grid.scale_factor(), 1.0);
    }

    #[test]
    fn test_solid_grid() {
        let grid = CubeGrid::solid(42);
        assert_eq!(grid.scale(), 0);
        assert!(matches!(grid.root(), Cube::Solid(42)));
    }

    #[test]
    fn test_effective_depth() {
        let grid = CubeGrid::new();
        assert_eq!(grid.effective_depth(4), 4);

        // Simulate one expansion (scale = 1)
        let grid = CubeGrid::new().with_scale(1);
        assert_eq!(grid.effective_depth(4), 6); // 4 + 2*1 = 6
    }

    #[test]
    fn test_max_coord() {
        let grid = CubeGrid::new();
        assert_eq!(grid.max_coord(4), 16); // 2^4 = 16

        let grid = CubeGrid::new().with_scale(1);
        assert_eq!(grid.max_coord(4), 64); // 2^6 = 64
    }

    #[test]
    fn test_in_bounds() {
        let grid = CubeGrid::new();
        assert!(grid.in_bounds(IVec3::new(0, 0, 0), 4));
        assert!(grid.in_bounds(IVec3::new(15, 15, 15), 4));
        assert!(!grid.in_bounds(IVec3::new(16, 0, 0), 4));
        assert!(!grid.in_bounds(IVec3::new(-1, 0, 0), 4));
    }

    #[test]
    fn test_set_cube_in_bounds() {
        let grid = CubeGrid::new();
        let grid = grid.set_cube(IVec3::new(0, 0, 0), 4, 42);

        assert_eq!(grid.scale(), 0); // No expansion needed
        assert_eq!(grid.get_cube(IVec3::new(0, 0, 0), 4), 42);
    }

    #[test]
    fn test_set_cube_out_of_bounds_expands() {
        let grid = CubeGrid::new();

        // Position 20 is out of bounds for depth 4 (max = 16)
        let grid = grid.set_cube(IVec3::new(20, 0, 0), 4, 42);

        // Should have expanded
        assert!(grid.scale() >= 1);

        // The voxel should be accessible at the adjusted position
        // After expansion, original (20, 0, 0) -> (20 + 16, 16, 16) = (36, 16, 16)
        let effective_depth = grid.effective_depth(4);
        assert!(effective_depth >= 6); // At least 6 after one expansion
    }

    #[test]
    fn test_set_cube_negative_coords_expand() {
        let grid = CubeGrid::new();

        // Negative position requires expansion
        let grid = grid.set_cube(IVec3::new(-1, 0, 0), 4, 42);

        // Should have expanded
        assert!(grid.scale() >= 1);
    }

    #[test]
    fn test_multiple_expansions() {
        let grid = CubeGrid::new();

        // Position 100 requires multiple expansions at depth 4
        // depth 4: max 16, depth 6: max 64, depth 8: max 256
        let grid = grid.set_cube(IVec3::new(100, 0, 0), 4, 42);

        // Should have expanded at least twice
        assert!(grid.scale() >= 2);
    }

    #[test]
    fn test_get_cube_out_of_bounds() {
        let grid = CubeGrid::new();
        assert_eq!(grid.get_cube(IVec3::new(100, 0, 0), 4), 0); // Returns 0 for out of bounds
    }

    #[test]
    fn test_from_cube() {
        let cube = Cube::Solid(128);
        let grid = CubeGrid::from_cube(cube);
        assert_eq!(grid.scale(), 0);
        assert!(matches!(grid.root(), Cube::Solid(128)));
    }

    #[test]
    fn test_from_cube_with_scale() {
        let cube = Cube::Solid(128);
        let grid = CubeGrid::from_cube_with_scale(cube, 2);
        assert_eq!(grid.scale(), 2);
        assert_eq!(grid.scale_factor(), 4.0);
    }
}
