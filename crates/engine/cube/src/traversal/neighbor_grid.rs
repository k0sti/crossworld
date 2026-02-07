use crate::{Cube, IVec3Ext};
use glam::IVec3;
use std::rc::Rc;

/// Index multiplier for converting 3D position to linear index: x + y*4 + z*16
const INDEX_MUL: IVec3 = IVec3::new(1, 4, 16);

/// Neighbor direction offsets
pub const OFFSET_LEFT: i32 = -1;
pub const OFFSET_RIGHT: i32 = 1;
pub const OFFSET_DOWN: i32 = -4;
pub const OFFSET_UP: i32 = 4;
pub const OFFSET_BACK: i32 = -16;
pub const OFFSET_FRONT: i32 = 16;

/// 4x4x4 neighbor grid for octree traversal with boundary conditions
///
/// Layout:
/// - Outer border (shell): initialized as ground (33) or sky (0)
/// - Inner 2x2x2 center: the 8 octants of the root voxel
///
/// Index calculation: index = x + y*4 + z*16
/// Neighbor offsets:
/// - -x: index - 1
/// - +x: index + 1
/// - -y: index - 4
/// - +y: index + 4
/// - -z: index - 16
/// - +z: index + 16
pub struct NeighborGrid {
    /// 4x4x4 = 64 voxels
    pub voxels: [Rc<Cube<u8>>; 64],
}

impl NeighborGrid {
    /// Create a new neighbor grid with initialized borders
    ///
    /// # Arguments
    /// * `root` - The root octree cube whose 8 children will be placed in the center 2x2x2
    /// * `border_materials` - Array of 4 material IDs for border voxels at each Y layer [y0, y1, y2, y3]
    ///   - For world: [hard_rock, water, air, air] or similar
    ///   - For avatars: [0, 0, 0, 0] for empty borders
    ///
    /// Note: Uses similar logic to `Cube::expand_once` but with different output structure
    /// (flat array vs hierarchical octree) for efficient neighbor lookups during traversal.
    pub fn new(root: &Cube<u8>, border_materials: [u8; 4]) -> Self {
        let mut voxels: [Rc<Cube<u8>>; 64] = std::array::from_fn(|i| {
            let (_x, y, _z) = Self::index_to_xyz(i);

            // All voxels at Y layer y use border_materials[y]
            // This ensures grid(y, _, _) = border_materials[y]
            // Border voxels keep this material, non-border voxels will be overwritten
            // by root octants in the next step
            Rc::new(Cube::Solid(border_materials[y as usize]))
        });

        // Fill center 2x2x2 with root octants
        // Center coordinates: x=[1,2], y=[1,2], z=[1,2]
        // Map octant index (0-7) to grid position
        for octant_idx in 0..8 {
            // from_octant_index returns binary coords 0/1
            let octant_pos_01 = IVec3::from_octant_index(octant_idx);

            // Convert binary {0,1} to grid coordinates {1,2}
            let grid_x = octant_pos_01.x + 1;
            let grid_y = octant_pos_01.y + 1;
            let grid_z = octant_pos_01.z + 1;
            let grid_idx = Self::xyz_to_index(grid_x, grid_y, grid_z);

            // Get octant from root, or use solid if root is not branching
            let octant = match root {
                Cube::Cubes(children) => children[octant_idx].clone(),
                Cube::Solid(v) => Rc::new(Cube::Solid(*v)),
                _ => Rc::new(Cube::Solid(0)),
            };

            voxels[grid_idx] = octant;
        }

        Self { voxels }
    }

    /// Convert 3D coordinates to linear index
    #[inline]
    pub fn xyz_to_index(x: i32, y: i32, z: i32) -> usize {
        (x + y * 4 + z * 16) as usize
    }

    /// Convert linear index to 3D position vector
    #[inline]
    pub fn index_to_pos(index: usize) -> IVec3 {
        let z = (index / 16) as i32;
        let rem = index % 16;
        let y = (rem / 4) as i32;
        let x = (rem % 4) as i32;
        IVec3::new(x, y, z)
    }

    /// Convert linear index to 3D coordinates (convenience wrapper around index_to_pos)
    #[inline]
    pub fn index_to_xyz(index: usize) -> (i32, i32, i32) {
        let pos = Self::index_to_pos(index);
        (pos.x, pos.y, pos.z)
    }

    /// Convert 3D position to linear index using dot product
    #[inline]
    pub fn pos_to_index(pos: IVec3) -> usize {
        pos.dot(INDEX_MUL) as usize
    }

    /// Get neighbor at offset from current index
    /// Returns None if out of bounds
    #[inline]
    pub fn get_neighbor(&self, index: usize, dx: i32, dy: i32, dz: i32) -> Option<&Rc<Cube<u8>>> {
        let (x, y, z) = Self::index_to_xyz(index);
        let nx = x + dx;
        let ny = y + dy;
        let nz = z + dz;

        if (0..4).contains(&nx) && (0..4).contains(&ny) && (0..4).contains(&nz) {
            Some(&self.voxels[Self::xyz_to_index(nx, ny, nz)])
        } else {
            None
        }
    }
}

/// View into a neighbor grid centered on a specific voxel
///
/// Provides convenient access to neighboring voxels with directional names
#[derive(Copy, Clone)]
pub struct NeighborView<'a> {
    grid: &'a NeighborGrid,
    center_index: usize,
}

impl<'a> NeighborView<'a> {
    pub fn new(grid: &'a NeighborGrid, center_index: usize) -> Self {
        Self { grid, center_index }
    }

    /// Get the center voxel
    #[inline]
    pub fn center(&self) -> &Rc<Cube<u8>> {
        &self.grid.voxels[self.center_index]
    }

    /// Get neighbor by offset
    /// Use constants: OFFSET_LEFT, OFFSET_RIGHT, OFFSET_DOWN, OFFSET_UP, OFFSET_BACK, OFFSET_FRONT
    #[inline]
    pub fn get(&self, offset: i32) -> Option<&Rc<Cube<u8>>> {
        let neighbor_idx = self.center_index as i32 + offset;
        if (0..64).contains(&neighbor_idx) {
            Some(&self.grid.voxels[neighbor_idx as usize])
        } else {
            None
        }
    }

    /// Get neighbor voxel by offset (preferred name for `get`)
    ///
    /// Use constants: OFFSET_LEFT, OFFSET_RIGHT, OFFSET_DOWN, OFFSET_UP, OFFSET_BACK, OFFSET_FRONT
    ///
    /// # Example
    /// ```
    /// use cube::{OFFSET_LEFT, OFFSET_UP};
    /// # use cube::{Cube, NeighborGrid, NeighborView};
    /// # let root = Cube::Solid(1);
    /// # let grid = NeighborGrid::new(&root, [0, 0, 0, 0]);
    /// # let view = NeighborView::new(&grid, 21);
    ///
    /// if let Some(left_neighbor) = view.neighbor(OFFSET_LEFT) {
    ///     // Process left neighbor
    /// }
    /// ```
    #[inline]
    pub fn neighbor(&self, offset: i32) -> Option<&Rc<Cube<u8>>> {
        self.get(offset)
    }

    /// Create a child grid one level deeper
    ///
    /// Uses IVec3 and dot product for compact index calculations.
    ///
    /// Formula for position p in 0..4:
    /// - parent_offset = (p + 1) / 2 - 1  (gives -1, 0, 0, 1)
    /// - child_pos = (p + 1) % 2          (gives 1, 0, 1, 0)
    pub fn create_child_grid(&self) -> NeighborGrid {
        let mut voxels: [Rc<Cube<u8>>; 64] = std::array::from_fn(|_| Rc::new(Cube::Solid(0)));

        for (i, voxel) in voxels.iter_mut().enumerate() {
            let pos = NeighborGrid::index_to_pos(i);

            // Calculate parent offset and child position
            // parent_offset = (p + 1) / 2 - 1  (gives -1, 0, 0, 1)
            // child_pos = (p + 1) % 2          (gives 1, 0, 1, 0)
            let parent_offset = (pos + 1) / 2 - 1;
            let child_pos = (pos + 1) % 2;

            // Calculate parent index
            let parent_idx = self.center_index as i32 + parent_offset.dot(INDEX_MUL);
            let parent = &self.grid.voxels[parent_idx as usize];

            // Calculate child octant index: x + y*2 + z*4
            let child_octant = (child_pos.x | (child_pos.y << 1) | (child_pos.z << 2)) as usize;

            // Extract child or replicate if solid
            *voxel = match &**parent {
                Cube::Cubes(children) => children[child_octant].clone(),
                Cube::Solid(v) => Rc::new(Cube::Solid(*v)),
                _ => Rc::new(Cube::Solid(0)),
            };
        }

        NeighborGrid { voxels }
    }
}

/// Coordinate for tracking position during traversal
///
/// Uses center-based coordinate system matching the [-1,1]³ raycast space:
/// - Root cube (depth=0) has pos = (0, 0, 0)
/// - Child positions offset by ±1 in each direction
/// - At depth d, positions range from -(2^d) to +(2^d) in steps of 2
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CubeCoord {
    /// Position in octree space (center-based)
    /// Root: (0, 0, 0), children offset by ±1
    pub pos: IVec3,
    /// Current depth level (0 = root)
    pub depth: u32,
}

impl CubeCoord {
    pub fn new(pos: IVec3, depth: u32) -> Self {
        Self { pos, depth }
    }

    /// Create child coordinate for octant
    /// Child position = parent_pos * 2 + offset where offset ∈ {-1,+1}³
    pub fn child(&self, octant_idx: usize) -> Self {
        // from_octant_index now returns 0/1 coords, convert to center-based -1/+1
        let offset_01 = IVec3::from_octant_index(octant_idx);
        let offset = offset_01 * 2 - IVec3::ONE;
        Self {
            pos: self.pos * 2 + offset,
            depth: self.depth + 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_conversion() {
        // Test corner cases
        assert_eq!(NeighborGrid::xyz_to_index(0, 0, 0), 0);
        assert_eq!(NeighborGrid::xyz_to_index(3, 3, 3), 63);
        assert_eq!(NeighborGrid::xyz_to_index(1, 1, 1), 21);

        // Test round-trip
        for i in 0..64 {
            let (x, y, z) = NeighborGrid::index_to_xyz(i);
            assert_eq!(NeighborGrid::xyz_to_index(x, y, z), i);
        }
    }

    #[test]
    fn test_neighbor_offsets() {
        let grid_idx = NeighborGrid::xyz_to_index(2, 2, 2); // Center

        // Test neighbor index calculations
        let (x, y, z) = NeighborGrid::index_to_xyz(grid_idx);
        assert_eq!((x, y, z), (2, 2, 2));

        // Left: x-1
        assert_eq!(NeighborGrid::xyz_to_index(x - 1, y, z), grid_idx - 1);
        // Right: x+1
        assert_eq!(NeighborGrid::xyz_to_index(x + 1, y, z), grid_idx + 1);
        // Down: y-1
        assert_eq!(NeighborGrid::xyz_to_index(x, y - 1, z), grid_idx - 4);
        // Up: y+1
        assert_eq!(NeighborGrid::xyz_to_index(x, y + 1, z), grid_idx + 4);
        // Back: z-1
        assert_eq!(NeighborGrid::xyz_to_index(x, y, z - 1), grid_idx - 16);
        // Front: z+1
        assert_eq!(NeighborGrid::xyz_to_index(x, y, z + 1), grid_idx + 16);
    }

    #[test]
    fn test_neighbor_grid_init() {
        let root = Cube::Solid(42);
        let border_materials = [33, 33, 0, 0]; // Ground at bottom, sky at top
        let grid = NeighborGrid::new(&root, border_materials);

        // Check center voxels are from root
        for octant_idx in 0..8 {
            // from_octant_index now returns 0/1 coords, convert to center-based -1/+1
            let pos_01 = IVec3::from_octant_index(octant_idx);
            let pos = pos_01 * 2 - IVec3::ONE;
            // Convert center-based {-1,+1} to grid coordinates [1,2]
            let idx = NeighborGrid::xyz_to_index((pos.x + 3) / 2, (pos.y + 3) / 2, (pos.z + 3) / 2);
            assert_eq!(grid.voxels[idx].id(), 42);
        }

        // Check borders
        let corner = &grid.voxels[NeighborGrid::xyz_to_index(0, 0, 0)];
        assert_eq!(corner.id(), 33); // Ground

        let top_corner = &grid.voxels[NeighborGrid::xyz_to_index(0, 3, 0)];
        assert_eq!(top_corner.id(), 0); // Sky
    }

    #[test]
    fn test_neighbor_view() {
        let root = Cube::tabulate(|i| Cube::Solid(i as u8));
        let border_materials = [33, 33, 0, 0]; // Ground at bottom, sky at top
        let grid = NeighborGrid::new(&root, border_materials);

        // Center voxel (octant 0 is at grid position 1,1,1)
        let center_idx = NeighborGrid::xyz_to_index(1, 1, 1);
        let view = NeighborView::new(&grid, center_idx);

        assert_eq!(view.center().id(), 0); // Octant 0: (0,0,0) → index 0

        // Right neighbor: (1,0,0) → index 1 (x+1)
        assert_eq!(view.get(OFFSET_RIGHT).unwrap().id(), 1);

        // Up neighbor: (0,1,0) → index 2 (y+1)
        assert_eq!(view.get(OFFSET_UP).unwrap().id(), 2);

        // Front neighbor: (0,0,1) → index 4 (z+1)
        assert_eq!(view.get(OFFSET_FRONT).unwrap().id(), 4);
    }
}
