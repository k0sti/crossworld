// Core coordinate types for octree navigation

use crate::IVec3Ext;
use glam::IVec3;

/// Coordinate for tracking position during traversal
///
/// Uses center-based coordinate system matching the [-1,1]³ raycast space:
/// - Root cube (depth=0) has pos = (0, 0, 0)
/// - Child positions offset by ±1 in each direction
/// - At depth d, positions range from -(2^d) to +(2^d) in steps of 2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CubeCoord {
    /// Position in octree space (center-based)
    /// Root: (0, 0, 0), children offset by ±1
    pub pos: IVec3,
    /// Current depth level (0 = root)
    pub depth: u32,
}

impl CubeCoord {
    /// Create a new coordinate at the given position and depth
    pub fn new(pos: IVec3, depth: u32) -> Self {
        Self { pos, depth }
    }

    /// Create root coordinate at (0, 0, 0)
    pub fn root(depth: u32) -> Self {
        Self {
            pos: IVec3::ZERO,
            depth,
        }
    }

    /// Create child coordinate for octant
    /// Child position = parent_pos * 2 + offset where offset ∈ {-1,+1}³
    pub fn child(&self, octant_idx: usize) -> Self {
        let offset = IVec3::from_octant_index(octant_idx);
        Self {
            pos: self.pos * 2 + offset,
            depth: self.depth + 1,
        }
    }
}

/// Axis-aligned bounding box in octree space
///
/// Represents a rectangular region of voxels at a specific depth.
/// The box is defined by its corner position and size in voxels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Box {
    /// Corner position (minimum coordinates) in octree space
    pub corner: CubeCoord,
    /// Size of the box in voxels (at the same depth as corner)
    /// Each component must be positive
    pub size: IVec3,
}

impl Box {
    /// Create a new box with the given corner and size
    ///
    /// # Arguments
    /// * `corner` - The minimum corner of the box (CubeCoord with position and depth)
    /// * `size` - The size of the box in voxels (x, y, z dimensions)
    ///
    /// # Example
    /// ```
    /// use cube::{Box, CubeCoord};
    /// use glam::IVec3;
    ///
    /// let corner = CubeCoord::new(IVec3::new(0, 0, 0), 3);
    /// let size = IVec3::new(4, 4, 4);
    /// let bbox = Box::new(corner, size);
    /// ```
    pub fn new(corner: CubeCoord, size: IVec3) -> Self {
        Self { corner, size }
    }

    /// Create a box from min and max coordinates at the same depth
    ///
    /// # Arguments
    /// * `min` - Minimum corner position
    /// * `max` - Maximum corner position (exclusive)
    /// * `depth` - Depth level for the box
    pub fn from_min_max(min: IVec3, max: IVec3, depth: u32) -> Self {
        Self {
            corner: CubeCoord::new(min, depth),
            size: max - min,
        }
    }

    /// Get the maximum corner position (exclusive)
    pub fn max(&self) -> IVec3 {
        self.corner.pos + self.size
    }

    /// Check if this box contains the given point at the same depth
    pub fn contains(&self, pos: IVec3, depth: u32) -> bool {
        if depth != self.corner.depth {
            return false;
        }
        let max = self.max();
        pos.x >= self.corner.pos.x
            && pos.x < max.x
            && pos.y >= self.corner.pos.y
            && pos.y < max.y
            && pos.z >= self.corner.pos.z
            && pos.z < max.z
    }

    /// Check if this box contains the given coordinate
    pub fn contains_coord(&self, coord: CubeCoord) -> bool {
        self.contains(coord.pos, coord.depth)
    }

    /// Get the volume of the box (number of voxels)
    pub fn volume(&self) -> i32 {
        self.size.x * self.size.y * self.size.z
    }

    /// Check if this box intersects with another box at the same depth
    pub fn intersects(&self, other: &Box) -> bool {
        if self.corner.depth != other.corner.depth {
            return false;
        }
        let self_max = self.max();
        let other_max = other.max();

        self.corner.pos.x < other_max.x
            && self_max.x > other.corner.pos.x
            && self.corner.pos.y < other_max.y
            && self_max.y > other.corner.pos.y
            && self.corner.pos.z < other_max.z
            && self_max.z > other.corner.pos.z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube_coord_new() {
        let coord = CubeCoord::new(IVec3::new(1, 2, 3), 5);
        assert_eq!(coord.pos, IVec3::new(1, 2, 3));
        assert_eq!(coord.depth, 5);
    }

    #[test]
    fn test_cube_coord_root() {
        let coord = CubeCoord::root(3);
        assert_eq!(coord.pos, IVec3::ZERO);
        assert_eq!(coord.depth, 3);
    }

    #[test]
    fn test_cube_coord_child() {
        let parent = CubeCoord::new(IVec3::new(0, 0, 0), 2);
        let child = parent.child(0); // octant 0: (-1, -1, -1)
        assert_eq!(child.pos, IVec3::new(-1, -1, -1));
        assert_eq!(child.depth, 3);

        let child7 = parent.child(7); // octant 7: (1, 1, 1)
        assert_eq!(child7.pos, IVec3::new(1, 1, 1));
        assert_eq!(child7.depth, 3);
    }

    #[test]
    fn test_box_new() {
        let corner = CubeCoord::new(IVec3::new(0, 0, 0), 3);
        let size = IVec3::new(4, 4, 4);
        let bbox = Box::new(corner, size);
        assert_eq!(bbox.corner.pos, IVec3::new(0, 0, 0));
        assert_eq!(bbox.size, IVec3::new(4, 4, 4));
    }

    #[test]
    fn test_box_from_min_max() {
        let bbox = Box::from_min_max(IVec3::new(0, 0, 0), IVec3::new(8, 8, 8), 3);
        assert_eq!(bbox.corner.pos, IVec3::new(0, 0, 0));
        assert_eq!(bbox.size, IVec3::new(8, 8, 8));
        assert_eq!(bbox.corner.depth, 3);
    }

    #[test]
    fn test_box_max() {
        let bbox = Box::new(
            CubeCoord::new(IVec3::new(2, 3, 4), 5),
            IVec3::new(10, 10, 10),
        );
        assert_eq!(bbox.max(), IVec3::new(12, 13, 14));
    }

    #[test]
    fn test_box_contains() {
        let bbox = Box::new(
            CubeCoord::new(IVec3::new(0, 0, 0), 3),
            IVec3::new(4, 4, 4),
        );

        assert!(bbox.contains(IVec3::new(0, 0, 0), 3));
        assert!(bbox.contains(IVec3::new(3, 3, 3), 3));
        assert!(!bbox.contains(IVec3::new(4, 4, 4), 3)); // Exclusive max
        assert!(!bbox.contains(IVec3::new(-1, 0, 0), 3));
        assert!(!bbox.contains(IVec3::new(0, 0, 0), 2)); // Different depth
    }

    #[test]
    fn test_box_contains_coord() {
        let bbox = Box::new(
            CubeCoord::new(IVec3::new(0, 0, 0), 3),
            IVec3::new(4, 4, 4),
        );

        assert!(bbox.contains_coord(CubeCoord::new(IVec3::new(2, 2, 2), 3)));
        assert!(!bbox.contains_coord(CubeCoord::new(IVec3::new(5, 5, 5), 3)));
    }

    #[test]
    fn test_box_volume() {
        let bbox = Box::new(
            CubeCoord::new(IVec3::new(0, 0, 0), 3),
            IVec3::new(4, 5, 6),
        );
        assert_eq!(bbox.volume(), 120);
    }

    #[test]
    fn test_box_intersects() {
        let bbox1 = Box::new(
            CubeCoord::new(IVec3::new(0, 0, 0), 3),
            IVec3::new(4, 4, 4),
        );
        let bbox2 = Box::new(
            CubeCoord::new(IVec3::new(2, 2, 2), 3),
            IVec3::new(4, 4, 4),
        );
        let bbox3 = Box::new(
            CubeCoord::new(IVec3::new(10, 10, 10), 3),
            IVec3::new(4, 4, 4),
        );

        assert!(bbox1.intersects(&bbox2));
        assert!(bbox2.intersects(&bbox1));
        assert!(!bbox1.intersects(&bbox3));
        assert!(!bbox3.intersects(&bbox1));
    }
}
