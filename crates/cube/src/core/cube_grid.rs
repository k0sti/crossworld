//! CubeGrid - An infinite-size grid of cubes with automatic expansion
//!
//! CubeGrid wraps a root Cube with a scale parameter that indicates the cube's
//! edge length as 2^scale. When voxels are placed outside the current bounds,
//! the grid automatically expands to accommodate them.
//!
//! # Coordinate System
//!
//! Coordinates are origin-centric: `[-size/2, size/2)` where `size = 1 << scale`.
//!
//! ```text
//! scale=0: size=1,  extents=[0, 1)      → single voxel at coord 0
//! scale=1: size=2,  extents=[-1, 1)    → 2³=8 voxels, coords {-1, 0}
//! scale=2: size=4,  extents=[-2, 2)    → 4³=64 voxels, coords {-2,-1,0,1}
//! scale=3: size=8,  extents=[-4, 4)    → 8³=512 voxels, coords {-4..3}
//! scale=4: size=16, extents=[-8, 8)    → 16³ voxels, coords {-8..7}
//! ```

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
/// Coordinates are origin-centric in the range `[-size/2, size/2)` where
/// `size = 1 << scale`. This means coordinates are symmetric around the origin.
///
/// # Example
///
/// ```
/// use cube::CubeGrid;
/// use glam::IVec3;
///
/// // Create a new grid at scale 4 (16x16x16, coords [-8, 8))
/// let grid = CubeGrid::new().with_scale(4);
///
/// // Place a voxel at origin
/// let grid = grid.set_cube(IVec3::new(0, 0, 0), 42);
/// assert_eq!(grid.get_cube(IVec3::new(0, 0, 0)), 42);
///
/// // Place a voxel outside bounds - grid will automatically expand
/// let grid = grid.set_cube(IVec3::new(10, 0, 0), 43);
/// assert!(grid.scale() > 4); // Scale increased due to expansion
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CubeGrid {
    /// The root cube data
    root: Rc<Cube<u8>>,
    /// Scale parameter: cube edge length is 2^scale, coords in [-size/2, size/2)
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

    /// Get the grid size (edge length = 2^scale).
    pub fn size(&self) -> i32 {
        1i32 << self.scale
    }

    /// Get the half size (size / 2). Coordinates range from -half_size to half_size-1.
    pub fn half_size(&self) -> i32 {
        self.size() / 2
    }

    /// Get the minimum valid coordinate (-half_size, or 0 for scale 0).
    pub fn min_coord(&self) -> i32 {
        if self.scale == 0 {
            0
        } else {
            -self.half_size()
        }
    }

    /// Get the maximum valid coordinate (exclusive: half_size, or 1 for scale 0).
    pub fn max_coord(&self) -> i32 {
        if self.scale == 0 {
            1
        } else {
            self.half_size()
        }
    }

    /// Check if a coordinate is within bounds.
    ///
    /// For scale 0: valid coord is [0, 1)
    /// For scale > 0: valid coords are [-size/2, size/2)
    pub fn in_bounds(&self, pos: IVec3) -> bool {
        let min = self.min_coord();
        let max = self.max_coord();
        pos.x >= min && pos.x < max && pos.y >= min && pos.y < max && pos.z >= min && pos.z < max
    }

    /// Convert origin-centric coordinates to internal octree coordinates.
    ///
    /// Origin-centric: [-size/2, size/2) for scale > 0, [0, 1) for scale 0
    /// Octree internal: [0, size)
    fn to_octree_coord(&self, pos: IVec3) -> IVec3 {
        if self.scale == 0 {
            pos // No offset for scale 0
        } else {
            pos + IVec3::splat(self.half_size())
        }
    }

    /// Expand the grid once, doubling its size.
    ///
    /// After expansion, all existing coordinates remain valid.
    /// The new coordinate range doubles symmetrically around the origin.
    pub fn expand(&self) -> Self {
        let expanded = Cube::expand_once(&self.root, [0, 0, 0, 0]);
        Self {
            root: Rc::new(expanded),
            scale: self.scale + 1,
        }
    }

    /// Set a voxel at the given position, expanding the grid if needed.
    ///
    /// # Arguments
    /// * `pos` - Position in origin-centric coordinates
    /// * `material` - Material value to set (0-255)
    ///
    /// # Returns
    /// A new CubeGrid with the voxel set (immutable operation)
    pub fn set_cube(mut self, pos: IVec3, material: u8) -> Self {
        // Expand until position is within bounds
        while !self.in_bounds(pos) {
            self = self.expand();
        }

        // Convert to octree coordinates and set
        let octree_pos = self.to_octree_coord(pos);
        let new_root = self.root.set_voxel(
            octree_pos.x,
            octree_pos.y,
            octree_pos.z,
            self.scale,
            material,
        );

        Self {
            root: Rc::new(new_root),
            scale: self.scale,
        }
    }

    /// Get a voxel material at the given position.
    ///
    /// Returns 0 (empty) if the position is outside bounds.
    pub fn get_cube(&self, pos: IVec3) -> u8 {
        if !self.in_bounds(pos) {
            return 0;
        }

        let octree_pos = self.to_octree_coord(pos);
        self.root.get_id(self.scale, octree_pos)
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

    // === Deprecated methods for backwards compatibility ===

    /// Get the cube scale factor (2^scale).
    #[deprecated(note = "Use size() instead")]
    pub fn scale_factor(&self) -> f32 {
        self.size() as f32
    }

    /// Get the effective depth for a given base depth.
    ///
    /// The effective depth is `base_depth + 2 * scale` because each expansion
    /// adds 2 octree levels.
    #[deprecated(note = "Use scale() directly - depth is now internal")]
    pub fn effective_depth(&self, base_depth: u32) -> u32 {
        base_depth + 2 * self.scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_grid() {
        let grid = CubeGrid::new();
        assert_eq!(grid.scale(), 0);
        assert_eq!(grid.size(), 1);
    }

    #[test]
    fn test_solid_grid() {
        let grid = CubeGrid::solid(42);
        assert_eq!(grid.scale(), 0);
        assert!(matches!(grid.root(), Cube::Solid(42)));
    }

    #[test]
    fn test_size_and_bounds() {
        // Scale 0: size=1, coords [0, 1)
        let grid = CubeGrid::new();
        assert_eq!(grid.size(), 1);
        assert_eq!(grid.half_size(), 0);
        assert_eq!(grid.min_coord(), 0);
        assert_eq!(grid.max_coord(), 1);

        // Scale 1: size=2, coords [-1, 1)
        let grid = CubeGrid::new().with_scale(1);
        assert_eq!(grid.size(), 2);
        assert_eq!(grid.half_size(), 1);
        assert_eq!(grid.min_coord(), -1);
        assert_eq!(grid.max_coord(), 1);

        // Scale 2: size=4, coords [-2, 2)
        let grid = CubeGrid::new().with_scale(2);
        assert_eq!(grid.size(), 4);
        assert_eq!(grid.half_size(), 2);
        assert_eq!(grid.min_coord(), -2);
        assert_eq!(grid.max_coord(), 2);

        // Scale 4: size=16, coords [-8, 8)
        let grid = CubeGrid::new().with_scale(4);
        assert_eq!(grid.size(), 16);
        assert_eq!(grid.half_size(), 8);
        assert_eq!(grid.min_coord(), -8);
        assert_eq!(grid.max_coord(), 8);
    }

    #[test]
    fn test_in_bounds_scale_0() {
        let grid = CubeGrid::new(); // scale 0: coords [0, 1)
        assert!(grid.in_bounds(IVec3::new(0, 0, 0)));
        assert!(!grid.in_bounds(IVec3::new(1, 0, 0)));
        assert!(!grid.in_bounds(IVec3::new(-1, 0, 0)));
    }

    #[test]
    fn test_in_bounds_scale_4() {
        let grid = CubeGrid::new().with_scale(4); // scale 4: coords [-8, 8)
        assert!(grid.in_bounds(IVec3::new(0, 0, 0)));
        assert!(grid.in_bounds(IVec3::new(-8, -8, -8)));
        assert!(grid.in_bounds(IVec3::new(7, 7, 7)));
        assert!(!grid.in_bounds(IVec3::new(8, 0, 0)));
        assert!(!grid.in_bounds(IVec3::new(-9, 0, 0)));
    }

    #[test]
    fn test_set_get_cube_at_origin() {
        let grid = CubeGrid::new().with_scale(4);
        let grid = grid.set_cube(IVec3::new(0, 0, 0), 42);

        assert_eq!(grid.scale(), 4); // No expansion needed
        assert_eq!(grid.get_cube(IVec3::new(0, 0, 0)), 42);
    }

    #[test]
    fn test_set_get_cube_negative_coords() {
        let grid = CubeGrid::new().with_scale(4);
        let grid = grid.set_cube(IVec3::new(-5, -3, -7), 42);

        assert_eq!(grid.scale(), 4); // No expansion needed
        assert_eq!(grid.get_cube(IVec3::new(-5, -3, -7)), 42);
    }

    #[test]
    fn test_set_cube_expands_when_needed() {
        let grid = CubeGrid::new().with_scale(4); // coords [-8, 8)

        // Position 10 is out of bounds
        let grid = grid.set_cube(IVec3::new(10, 0, 0), 42);

        // Should have expanded to scale 5 (coords [-16, 16))
        assert!(grid.scale() >= 5);
        assert_eq!(grid.get_cube(IVec3::new(10, 0, 0)), 42);
    }

    #[test]
    fn test_set_cube_expands_negative() {
        let grid = CubeGrid::new().with_scale(4); // coords [-8, 8)

        // Position -10 is out of bounds
        let grid = grid.set_cube(IVec3::new(-10, 0, 0), 42);

        // Should have expanded
        assert!(grid.scale() >= 5);
        assert_eq!(grid.get_cube(IVec3::new(-10, 0, 0)), 42);
    }

    #[test]
    fn test_multiple_expansions() {
        let grid = CubeGrid::new().with_scale(4); // coords [-8, 8)

        // Position 100 requires multiple expansions
        // scale 5: [-16, 16), scale 6: [-32, 32), scale 7: [-64, 64), scale 8: [-128, 128)
        let grid = grid.set_cube(IVec3::new(100, 0, 0), 42);

        assert!(grid.scale() >= 8);
        assert_eq!(grid.get_cube(IVec3::new(100, 0, 0)), 42);
    }

    #[test]
    fn test_expansion_preserves_existing_data() {
        let grid = CubeGrid::new().with_scale(4);
        let grid = grid.set_cube(IVec3::new(0, 0, 0), 42);
        let grid = grid.set_cube(IVec3::new(-5, 3, 2), 43);

        // Expand by setting voxel outside bounds
        let grid = grid.set_cube(IVec3::new(100, 0, 0), 44);

        // Original voxels should still be accessible at same coords
        assert_eq!(grid.get_cube(IVec3::new(0, 0, 0)), 42);
        assert_eq!(grid.get_cube(IVec3::new(-5, 3, 2)), 43);
        assert_eq!(grid.get_cube(IVec3::new(100, 0, 0)), 44);
    }

    #[test]
    fn test_get_cube_out_of_bounds() {
        let grid = CubeGrid::new().with_scale(4);
        assert_eq!(grid.get_cube(IVec3::new(100, 0, 0)), 0); // Returns 0 for out of bounds
        assert_eq!(grid.get_cube(IVec3::new(-100, 0, 0)), 0);
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
        assert_eq!(grid.size(), 4);
    }

    #[test]
    fn test_expand() {
        let grid = CubeGrid::new().with_scale(4);
        assert_eq!(grid.scale(), 4);
        assert_eq!(grid.size(), 16);

        let expanded = grid.expand();
        assert_eq!(expanded.scale(), 5);
        assert_eq!(expanded.size(), 32);
    }

    #[test]
    fn test_to_octree_coord() {
        let grid = CubeGrid::new().with_scale(4);

        // At scale 4, half_size = 8
        // Origin-centric pos (-5, 3, 0) -> octree (3, 11, 8)
        let pos = IVec3::new(-5, 3, 0);
        let octree = grid.to_octree_coord(pos);
        assert_eq!(octree, IVec3::new(3, 11, 8));

        // Origin maps to center of octree
        let origin = grid.to_octree_coord(IVec3::ZERO);
        assert_eq!(origin, IVec3::splat(8));

        // Corner coordinates
        let min_corner = grid.to_octree_coord(IVec3::splat(-8));
        assert_eq!(min_corner, IVec3::ZERO);
    }
}
