use crate::{Cube, IVec3Ext};
use glam::IVec3;
use std::rc::Rc;

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
    /// * `ground_value` - Material ID for ground voxels (default: 33)
    /// * `sky_value` - Material ID for sky voxels (default: 0)
    pub fn new(root: &Cube<i32>, ground_value: i32, sky_value: i32) -> Self {
        let mut voxels: [Rc<Cube<i32>>; 64] = std::array::from_fn(|i| {
            let (x, y, z) = Self::index_to_xyz(i);

            // Determine if this is a border voxel
            let is_border = x == 0 || x == 3 || y == 0 || y == 3 || z == 0 || z == 3;

            if is_border {
                // Border voxels: ground below center plane, sky above/at center
                // Center plane is at y=1,2 (the middle two layers)
                if y < 2 {
                    Rc::new(Cube::Solid(ground_value))
                } else {
                    Rc::new(Cube::Solid(sky_value))
                }
            } else {
                // Non-border: placeholder, will be filled next
                Rc::new(Cube::Solid(sky_value))
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

    /// Convert linear index to 3D coordinates
    #[inline]
    pub fn index_to_xyz(index: usize) -> (i32, i32, i32) {
        let z = (index / 16) as i32;
        let rem = index % 16;
        let y = (rem / 4) as i32;
        let x = (rem % 4) as i32;
        (x, y, z)
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
    #[inline]
    pub fn get(&self, dx: i32, dy: i32, dz: i32) -> Option<&Rc<Cube<i32>>> {
        self.grid.get_neighbor(self.center_index, dx, dy, dz)
    }

    // Named accessors for direct neighbors

    #[inline]
    pub fn left(&self) -> Option<&Rc<Cube<i32>>> {
        self.get(-1, 0, 0)
    }

    #[inline]
    pub fn right(&self) -> Option<&Rc<Cube<i32>>> {
        self.get(1, 0, 0)
    }

    #[inline]
    pub fn down(&self) -> Option<&Rc<Cube<i32>>> {
        self.get(0, -1, 0)
    }

    #[inline]
    pub fn up(&self) -> Option<&Rc<Cube<i32>>> {
        self.get(0, 1, 0)
    }

    #[inline]
    pub fn back(&self) -> Option<&Rc<Cube<i32>>> {
        self.get(0, 0, -1)
    }

    #[inline]
    pub fn front(&self) -> Option<&Rc<Cube<i32>>> {
        self.get(0, 0, 1)
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
/// Arguments:
/// - `view`: View of the current voxel and its neighbors
/// - `coord`: Current position and depth in the octree
pub type TraversalVisitor<'a> = &'a mut dyn FnMut(NeighborView, CubeCoord);

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
/// let root = Cube::Solid(33);
/// let grid = NeighborGrid::new(&root, 33, 0);
///
/// traverse_with_neighbors(&grid, &mut |view, coord| {
///     if let Some(left) = view.left() {
///         // Process with left neighbor
///     }
/// }, 3);
/// ```
pub fn traverse_with_neighbors(
    grid: &NeighborGrid,
    visitor: TraversalVisitor,
    max_depth: u32,
) {
    // Traverse the center 2x2x2 octants
    for octant_idx in 0..8 {
        let octant_pos = IVec3::from_octant_index(octant_idx);
        let grid_idx = NeighborGrid::xyz_to_index(
            octant_pos.x + 1,
            octant_pos.y + 1,
            octant_pos.z + 1,
        );

        let coord = CubeCoord::new(octant_pos, max_depth);
        traverse_recursive(grid, grid_idx, coord, visitor, max_depth);
    }
}

/// Internal recursive traversal function
#[allow(clippy::only_used_in_recursion)]
fn traverse_recursive(
    grid: &NeighborGrid,
    grid_idx: usize,
    coord: CubeCoord,
    visitor: TraversalVisitor,
    max_depth: u32,
) {
    let voxel = &grid.voxels[grid_idx];

    // Create view and visit this voxel
    let view = NeighborView::new(grid, grid_idx);
    visitor(view, coord);

    // Base case: reached target depth or voxel is solid
    if coord.depth == 0 {
        return;
    }

    match &**voxel {
        Cube::Solid(_) => {
            // Solid voxel - no need to descend further
        }
        Cube::Cubes(children) => {
            // Branch: create new 4x4x4 grid for next level
            let child_grid = create_child_grid(grid, grid_idx, children);

            // Recursively traverse each child octant
            for octant_idx in 0..8 {
                let octant_pos = IVec3::from_octant_index(octant_idx);
                let child_grid_idx = NeighborGrid::xyz_to_index(
                    octant_pos.x + 1,
                    octant_pos.y + 1,
                    octant_pos.z + 1,
                );

                let child_coord = coord.child(octant_idx);
                traverse_recursive(&child_grid, child_grid_idx, child_coord, visitor, max_depth);
            }
        }
        _ => {
            // Planes/Slices: treat as solid for now
        }
    }
}

/// Create child grid for next recursion level
///
/// Takes the 8 children of the current voxel and places them in the center 2x2x2.
/// Fills the border with neighbors from the parent grid.
fn create_child_grid(
    parent_grid: &NeighborGrid,
    parent_idx: usize,
    children: &[Rc<Cube<i32>>; 8],
) -> NeighborGrid {
    let voxels: [Rc<Cube<i32>>; 64] = std::array::from_fn(|i| {
        let (x, y, z) = NeighborGrid::index_to_xyz(i);

        // Check if this is a center voxel (will be filled by children)
        if (1..=2).contains(&x) && (1..=2).contains(&y) && (1..=2).contains(&z) {
            // Map grid position back to octant index
            let octant_pos = IVec3::new(x - 1, y - 1, z - 1);
            let octant_idx = octant_pos.to_octant_index();
            return children[octant_idx].clone();
        }

        // Border voxel: sample from parent grid's neighbors

        // Map child border position to parent neighbor direction
        // x: [0,3] -> [-1, 0, 0, 1] relative to parent
        // Each child border face maps to a parent neighbor's child face
        let dx = match x {
            0 => -1,
            3 => 1,
            _ => 0,
        };
        let dy = match y {
            0 => -1,
            3 => 1,
            _ => 0,
        };
        let dz = match z {
            0 => -1,
            3 => 1,
            _ => 0,
        };

        // Get the neighbor voxel from parent grid
        if let Some(neighbor) = parent_grid.get_neighbor(parent_idx, dx, dy, dz) {
            // Get the appropriate child from the neighbor
            // The child on the facing side
            let child_x = if dx < 0 { 1 } else if dx > 0 { 0 } else { (x - 1).clamp(0, 1) };
            let child_y = if dy < 0 { 1 } else if dy > 0 { 0 } else { (y - 1).clamp(0, 1) };
            let child_z = if dz < 0 { 1 } else if dz > 0 { 0 } else { (z - 1).clamp(0, 1) };

            let child_octant = IVec3::new(child_x, child_y, child_z).to_octant_index();

            match &**neighbor {
                Cube::Cubes(neighbor_children) => neighbor_children[child_octant].clone(),
                Cube::Solid(v) => Rc::new(Cube::Solid(*v)),
                _ => Rc::new(Cube::Solid(0)),
            }
        } else {
            // No neighbor (boundary): use default
            Rc::new(Cube::Solid(0))
        }
    });

    NeighborGrid { voxels }
}

/// Initialize and traverse octree with generative function
///
/// This is a convenience function that creates the initial grid from a root cube
/// and traverses it with neighbor context.
///
/// # Arguments
/// * `root` - Root octree cube
/// * `depth` - Depth to traverse
/// * `visitor` - Function called for each voxel with its neighbors
pub fn traverse_octree_with_neighbors<F>(
    root: &Cube<i32>,
    depth: u32,
    mut visitor: F,
)
where
    F: FnMut(NeighborView, CubeCoord),
{
    let grid = NeighborGrid::new(root, 33, 0);
    traverse_with_neighbors(&grid, &mut visitor, depth);
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
        let grid = NeighborGrid::new(&root, 33, 0);

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
        let grid = NeighborGrid::new(&root, 33, 0);

        // Center voxel (octant 0 is at grid position 1,1,1)
        let center_idx = NeighborGrid::xyz_to_index(1, 1, 1);
        let view = NeighborView::new(&grid, center_idx);

        assert_eq!(view.center().id(), 0); // Octant 0

        // Right neighbor should be octant 4 (x+1)
        assert_eq!(view.right().unwrap().id(), 4);

        // Up neighbor should be octant 2 (y+1)
        assert_eq!(view.up().unwrap().id(), 2);

        // Front neighbor should be octant 1 (z+1)
        assert_eq!(view.front().unwrap().id(), 1);
    }
}
