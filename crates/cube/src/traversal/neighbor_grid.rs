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
    pub voxels: [Rc<Cube<i32>>; 64],
}

impl NeighborGrid {
    /// Create a new neighbor grid with initialized borders
    ///
    /// # Arguments
    /// * `root` - The root octree cube whose 8 children will be placed in the center 2x2x2
    /// * `border_materials` - Array of 4 material IDs for border voxels at each Y layer [y0, y1, y2, y3]
    ///   - For world: [hard_rock, water, air, air] or similar
    ///   - For avatars: [0, 0, 0, 0] for empty borders
    pub fn new(root: &Cube<i32>, border_materials: [i32; 4]) -> Self {
        let mut voxels: [Rc<Cube<i32>>; 64] = std::array::from_fn(|i| {
            let (x, y, z) = Self::index_to_xyz(i);

            // Determine if this is a border voxel
            let is_border = x == 0 || x == 3 || y == 0 || y == 3 || z == 0 || z == 3;

            if is_border {
                // Border voxels use material from corresponding Y layer
                Rc::new(Cube::Solid(border_materials[y as usize]))
            } else {
                // Non-border: placeholder, will be filled next
                // Use top layer material as default
                Rc::new(Cube::Solid(border_materials[3]))
            }
        });

        // Fill center 2x2x2 with root octants
        // Center coordinates: x=[1,2], y=[1,2], z=[1,2]
        // Map octant index (0-7) to grid position
        for octant_idx in 0..8 {
            let octant_pos = IVec3::from_octant_index(octant_idx);
            // Offset by 1 to center in the 4x4x4 grid
            let grid_x = octant_pos.x + 1;
            let grid_y = octant_pos.y + 1;
            let grid_z = octant_pos.z + 1;
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
    pub fn get_neighbor(&self, index: usize, dx: i32, dy: i32, dz: i32) -> Option<&Rc<Cube<i32>>> {
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
    pub fn center(&self) -> &Rc<Cube<i32>> {
        &self.grid.voxels[self.center_index]
    }

    /// Get neighbor by offset
    /// Use constants: OFFSET_LEFT, OFFSET_RIGHT, OFFSET_DOWN, OFFSET_UP, OFFSET_BACK, OFFSET_FRONT
    #[inline]
    pub fn get(&self, offset: i32) -> Option<&Rc<Cube<i32>>> {
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
    /// use crossworld_cube::{OFFSET_LEFT, OFFSET_UP};
    /// # use crossworld_cube::{Cube, NeighborGrid, NeighborView};
    /// # let root = Cube::Solid(1);
    /// # let grid = NeighborGrid::new(&root, [0, 0, 0, 0]);
    /// # let view = NeighborView::new(&grid, 21);
    ///
    /// if let Some(left_neighbor) = view.neighbor(OFFSET_LEFT) {
    ///     // Process left neighbor
    /// }
    /// ```
    #[inline]
    pub fn neighbor(&self, offset: i32) -> Option<&Rc<Cube<i32>>> {
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
        let voxels: [Rc<Cube<i32>>; 64] = std::array::from_fn(|i| {
            let pos = NeighborGrid::index_to_pos(i);

            // Calculate parent offset and child position using formula
            let parent_offset = (pos + 1) / 2 - 1;
            let child_pos = (pos + 1) % 2;

            // Calculate parent index using dot product
            let parent_idx = self.center_index as i32 + parent_offset.dot(INDEX_MUL);

            assert!((0..64).contains(&parent_idx));
            let parent = &self.grid.voxels[parent_idx as usize];

            // Calculate child octant index using dot product with (4, 2, 1)
            let child_octant = child_pos.dot(IVec3::new(4, 2, 1)) as usize;

            // Extract child or replicate if solid
            match &**parent {
                Cube::Cubes(children) => children[child_octant].clone(),
                Cube::Solid(v) => Rc::new(Cube::Solid(*v)),
                _ => Rc::new(Cube::Solid(0)),
            }
        });

        NeighborGrid { voxels }
    }
}

/// Coordinate for tracking position during traversal
#[derive(Debug, Clone, Copy)]
pub struct CubeCoord {
    /// Position in octree space
    pub pos: IVec3,
    /// Current depth level (0 = leaf)
    pub depth: u32,
}

impl CubeCoord {
    pub fn new(pos: IVec3, depth: u32) -> Self {
        Self { pos, depth }
    }

    /// Create child coordinate for octant
    pub fn child(&self, octant_idx: usize) -> Self {
        let offset = IVec3::from_octant_index(octant_idx);
        Self {
            pos: (self.pos << 1) + offset,
            depth: self.depth - 1,
        }
    }
}

/// Visitor function type for octree traversal
///
/// Called only for leaf nodes (Solid, Planes, Slices, or at depth==0).
///
/// Arguments:
/// - `view`: View of the current voxel and its neighbors
/// - `coord`: Current position and depth in the octree
/// - `subleaf`: True if this leaf can be subdivided (depth > 0), false if at maximum depth
///
/// Returns:
/// - `true`: Subdivide this leaf and traverse its children (only applies if subleaf==true)
/// - `false`: Do not subdivide
pub type TraversalVisitor<'a> = &'a mut dyn FnMut(NeighborView, CubeCoord, bool) -> bool;

/// Recursively traverse octree with neighbor context
///
/// This function provides each voxel with access to its 26 neighbors (or fewer at boundaries).
/// The traversal maintains a 4x4x4 sliding window of voxels as it descends the octree.
///
/// # Arguments
/// * `grid` - Current 4x4x4 neighbor grid
/// * `visitor` - Callback function receiving each voxel with its neighbors and position
/// * `max_depth` - Maximum depth to traverse (stops at depth 0)
///
/// # Example
/// ```
/// use crossworld_cube::{Cube, NeighborGrid, traverse_with_neighbors, OFFSET_LEFT};
///
/// let root = Cube::Solid(33);
/// let border_materials = [33, 33, 0, 0]; // Ground at bottom, sky at top
/// let grid = NeighborGrid::new(&root, border_materials);
///
/// traverse_with_neighbors(&grid, &mut |view, coord, subleaf| {
///     if let Some(left) = view.get(OFFSET_LEFT) {
///         // Process with left neighbor
///     }
///     subleaf  // Subdivide if this is a subleaf, otherwise false
/// }, 3);
/// ```
pub fn traverse_octree(grid: &NeighborGrid, visitor: TraversalVisitor, max_depth: u32) {
    // Traverse the center 2x2x2 octants
    // Center offset in grid: (1,1,1) = 1 + 1*4 + 1*16 = 21
    const CENTER_OFFSET: usize = 21;

    for octant_idx in 0..8 {
        let octant_pos = IVec3::from_octant_index(octant_idx);
        let grid_idx = CENTER_OFFSET + octant_pos.dot(INDEX_MUL) as usize;
        let view = NeighborView::new(grid, grid_idx);
        let coord = CubeCoord::new(octant_pos, max_depth);
        traverse_recursive(view, coord, visitor, false);
    }
}

/// Internal recursive traversal function
fn traverse_recursive(
    view: NeighborView,
    coord: CubeCoord,
    visitor: TraversalVisitor,
    subleaf: bool,
) {
    let voxel = view.center();

    // At maximum depth, this is a final leaf - cannot subdivide further
    if coord.depth == 0 {
        visitor(view, coord, subleaf);
        return;
    }

    match &**voxel {
        Cube::Solid(_) | Cube::Planes { .. } | Cube::Slices { .. } => {
            // Leaf node at depth > 0 - can be subdivided
            let should_subdivide = visitor(view, coord, subleaf); // subleaf = true

            if should_subdivide {
                // Subdivide leaf and traverse children
                // create_child_grid will replicate the solid value into 8 children
                let child_grid = view.create_child_grid();
                const CENTER_OFFSET: usize = 21;
                for octant_idx in 0..8 {
                    let octant_pos = IVec3::from_octant_index(octant_idx);
                    let child_grid_idx = CENTER_OFFSET + octant_pos.dot(INDEX_MUL) as usize;
                    let child_view = NeighborView::new(&child_grid, child_grid_idx);
                    let child_coord = coord.child(octant_idx);
                    traverse_recursive(child_view, child_coord, visitor, true);
                }
            }
        }
        Cube::Cubes(_children) => {
            // Branch node - traverse children without calling visitor
            let child_grid = view.create_child_grid();
            const CENTER_OFFSET: usize = 21;
            for octant_idx in 0..8 {
                let octant_pos = IVec3::from_octant_index(octant_idx);
                let child_grid_idx = CENTER_OFFSET + octant_pos.dot(INDEX_MUL) as usize;
                let child_view = NeighborView::new(&child_grid, child_grid_idx);
                let child_coord = coord.child(octant_idx);
                traverse_recursive(child_view, child_coord, visitor, false);
            }
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
            let pos = IVec3::from_octant_index(octant_idx);
            let idx = NeighborGrid::xyz_to_index(pos.x + 1, pos.y + 1, pos.z + 1);
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
        let root = Cube::tabulate(|i| Cube::Solid(i as i32));
        let border_materials = [33, 33, 0, 0]; // Ground at bottom, sky at top
        let grid = NeighborGrid::new(&root, border_materials);

        // Center voxel (octant 0 is at grid position 1,1,1)
        let center_idx = NeighborGrid::xyz_to_index(1, 1, 1);
        let view = NeighborView::new(&grid, center_idx);

        assert_eq!(view.center().id(), 0); // Octant 0

        // Right neighbor should be octant 4 (x+1)
        assert_eq!(view.get(OFFSET_RIGHT).unwrap().id(), 4);

        // Up neighbor should be octant 2 (y+1)
        assert_eq!(view.get(OFFSET_UP).unwrap().id(), 2);

        // Front neighbor should be octant 1 (z+1)
        assert_eq!(view.get(OFFSET_FRONT).unwrap().id(), 1);
    }
}
