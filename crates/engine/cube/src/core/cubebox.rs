//! CubeBox - A bounded voxel model with explicit dimensions
//!
//! CubeBox wraps a Cube with its actual voxel dimensions, preserving
//! the original model bounds that would otherwise be lost when loading
//! into a power-of-2 octree structure.

use crate::Cube;
use glam::IVec3;

/// A bounded voxel model with explicit dimensions.
///
/// CubeBox pairs a `Cube<T>` octree with the actual model dimensions,
/// allowing preservation of bounds information for physics, placement,
/// and rendering operations.
///
/// # Coordinate System
/// - The model is always positioned at origin (0,0,0) within the octree
/// - `size` represents the actual model dimensions in voxels
/// - `depth` determines the coordinate scale: size is in units of 2^depth
///
/// # Example
/// A 16x30x12 avatar model loaded as CubeBox:
/// - `size = (16, 30, 12)`
/// - `depth = 5` (32³ octree, minimum to contain 30)
/// - `octree_size() = 32`
#[derive(Debug, Clone, PartialEq)]
pub struct CubeBox<T> {
    /// The octree containing the voxel data
    pub cube: Cube<T>,
    /// Original model size in voxels (not power-of-2 aligned)
    pub size: IVec3,
    /// Octree depth - size is measured in units of 2^depth voxels
    pub depth: u32,
}

impl<T> CubeBox<T> {
    /// Create a new CubeBox with the given cube, size, and depth.
    ///
    /// # Arguments
    /// * `cube` - The octree data with model at origin
    /// * `size` - Original model dimensions in voxels
    /// * `depth` - Octree depth (octree size = 2^depth)
    ///
    /// # Panics
    /// Panics if any dimension of size exceeds 2^depth
    pub fn new(cube: Cube<T>, size: IVec3, depth: u32) -> Self {
        let octree_size = 1i32 << depth;
        assert!(
            size.x <= octree_size && size.y <= octree_size && size.z <= octree_size,
            "Size {:?} exceeds octree capacity {} (2^{})",
            size,
            octree_size,
            depth
        );
        Self { cube, size, depth }
    }

    /// Get the octree size (2^depth).
    ///
    /// This is the size of the containing power-of-2 cube.
    #[inline]
    pub fn octree_size(&self) -> i32 {
        1 << self.depth
    }

    /// Check if the model size fits within the octree.
    ///
    /// Returns true if all dimensions of size are <= octree_size.
    #[inline]
    pub fn fits_octree(&self) -> bool {
        let octree_size = self.octree_size();
        self.size.x <= octree_size && self.size.y <= octree_size && self.size.z <= octree_size
    }

    /// Get the model bounds as min/max coordinates.
    ///
    /// Returns (min, max) where min is always (0,0,0) and max is size.
    #[inline]
    pub fn bounds(&self) -> (IVec3, IVec3) {
        (IVec3::ZERO, self.size)
    }

    /// Get the minimum required depth to contain a given model size.
    ///
    /// Calculates the smallest depth such that 2^depth >= max(size).
    pub fn min_depth_for_size(size: IVec3) -> u32 {
        let max_dim = size.x.max(size.y).max(size.z);
        if max_dim <= 1 {
            0
        } else {
            // We need 2^depth >= max_dim
            // depth = ceil(log2(max_dim))
            32 - (max_dim - 1).leading_zeros()
        }
    }
}

impl<T: Clone + Default + PartialEq> CubeBox<T> {
    /// Place this box model into a target cube at the specified position.
    ///
    /// This method inserts the CubeBox's voxel data into a larger cube
    /// at the given position, with optional scaling.
    ///
    /// # Arguments
    /// * `target` - The cube to place into
    /// * `target_depth` - Depth of the target cube
    /// * `position` - Position in target cube coordinates (corner-based, origin at 0,0,0)
    /// * `scale` - Scale exponent: 0 = 1:1, 1 = 2x, 2 = 4x (negative values clamped to 0)
    ///
    /// # Returns
    /// New cube with the model placed (immutable operation)
    ///
    /// # Example
    /// ```ignore
    /// let avatar = load_vox_to_cubebox(bytes)?;
    /// let world = Cube::solid(0);
    /// // Place avatar at position (100, 0, 100) in world coordinates
    /// let world_with_avatar = avatar.place_in(&world, 8, IVec3::new(100, 0, 100), 0);
    /// ```
    pub fn place_in(
        &self,
        target: &Cube<T>,
        target_depth: u32,
        position: IVec3,
        scale: i32,
    ) -> Cube<T> {
        // Clamp scale to non-negative
        let effective_scale = scale.max(0) as u32;

        // Calculate the final scale for update_depth_tree
        // The model is at self.depth, target is at target_depth
        // Base scale difference + additional scale
        let depth_diff = target_depth.saturating_sub(self.depth);
        let final_scale = depth_diff + effective_scale;

        target.update_depth_tree(target_depth, position, final_scale, &self.cube)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn test_cubebox_new() {
        let cube = Cube::Solid(5u8);
        let size = IVec3::new(16, 30, 12);
        let depth = 5;

        let cubebox = CubeBox::new(cube, size, depth);

        assert_eq!(cubebox.size, size);
        assert_eq!(cubebox.depth, depth);
        assert_eq!(cubebox.octree_size(), 32);
    }

    #[test]
    fn test_cubebox_octree_size() {
        let cube = Cube::Solid(0u8);

        assert_eq!(CubeBox::new(cube.clone(), IVec3::ONE, 0).octree_size(), 1);
        assert_eq!(CubeBox::new(cube.clone(), IVec3::ONE, 1).octree_size(), 2);
        assert_eq!(CubeBox::new(cube.clone(), IVec3::ONE, 3).octree_size(), 8);
        assert_eq!(CubeBox::new(cube.clone(), IVec3::ONE, 5).octree_size(), 32);
    }

    #[test]
    fn test_cubebox_fits_octree() {
        let cube = Cube::Solid(0u8);

        // 8x8x8 fits in depth 3 (size 8)
        let fits = CubeBox::new(cube.clone(), IVec3::splat(8), 3);
        assert!(fits.fits_octree());

        // 16x30x12 fits in depth 5 (size 32)
        let fits = CubeBox::new(cube.clone(), IVec3::new(16, 30, 12), 5);
        assert!(fits.fits_octree());
    }

    #[test]
    fn test_cubebox_bounds() {
        let cube = Cube::Solid(0u8);
        let size = IVec3::new(16, 30, 12);
        let cubebox = CubeBox::new(cube, size, 5);

        let (min, max) = cubebox.bounds();
        assert_eq!(min, IVec3::ZERO);
        assert_eq!(max, size);
    }

    #[test]
    fn test_min_depth_for_size() {
        assert_eq!(CubeBox::<u8>::min_depth_for_size(IVec3::splat(1)), 0);
        assert_eq!(CubeBox::<u8>::min_depth_for_size(IVec3::splat(2)), 1);
        assert_eq!(CubeBox::<u8>::min_depth_for_size(IVec3::splat(8)), 3);
        assert_eq!(CubeBox::<u8>::min_depth_for_size(IVec3::splat(9)), 4);
        assert_eq!(CubeBox::<u8>::min_depth_for_size(IVec3::new(16, 30, 12)), 5);
        assert_eq!(CubeBox::<u8>::min_depth_for_size(IVec3::splat(32)), 5);
        assert_eq!(CubeBox::<u8>::min_depth_for_size(IVec3::splat(33)), 6);
    }

    #[test]
    fn test_place_in_simple() {
        // Create a small CubeBox with a solid cube
        let model_cube = Cube::Solid(5u8);
        let cubebox = CubeBox::new(model_cube, IVec3::splat(2), 1);

        // Create an empty target cube
        let target = Cube::Solid(0u8);

        // Place at origin with no scale
        let result = cubebox.place_in(&target, 3, IVec3::ZERO, 0);

        // The result should have the model placed
        assert!(!matches!(result, Cube::Solid(0)));
    }

    #[test]
    fn test_place_in_with_scale() {
        // Create a 2x2x2 model (depth 1)
        let children: [Rc<Cube<u8>>; 8] = std::array::from_fn(|i| Rc::new(Cube::Solid(i as u8)));
        let model_cube = Cube::Cubes(Box::new(children));
        let cubebox = CubeBox::new(model_cube, IVec3::splat(2), 1);

        // Create an empty target at depth 4 (16x16x16)
        let target = Cube::Solid(0u8);

        // Place with scale=1 (2x) at position (4, 4, 4)
        let result = cubebox.place_in(&target, 4, IVec3::splat(4), 1);

        // The model should be scaled up
        assert!(!matches!(result, Cube::Solid(0)));
    }

    #[test]
    #[should_panic(expected = "Size")]
    fn test_cubebox_size_exceeds_depth() {
        let cube = Cube::Solid(0u8);
        // Size 16 cannot fit in depth 3 (size 8)
        CubeBox::new(cube, IVec3::splat(16), 3);
    }
}
