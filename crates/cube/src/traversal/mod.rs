// Neighbor-aware octree traversal

use crate::IVec3Ext;
use glam::IVec3;

pub mod neighbor_grid;
pub mod visit_faces;

// Re-export main types and functions from submodules
pub use neighbor_grid::{
    CubeCoord, NeighborGrid, NeighborView, OFFSET_BACK, OFFSET_DOWN, OFFSET_FRONT, OFFSET_LEFT,
    OFFSET_RIGHT, OFFSET_UP,
};
pub use visit_faces::{visit_faces, FaceInfo};

// traverse_octree and TraversalVisitor are defined in this file and exported directly

/// Visitor function type for octree traversal
///
/// Called only for leaf nodes (Solid, Quad, Layers, or at depth==0).
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
/// use cube::{Cube, NeighborGrid, traverse_octree, OFFSET_LEFT};
///
/// let root = Cube::Solid(33);
/// let border_materials = [33, 33, 0, 0]; // Ground at bottom, sky at top
/// let grid = NeighborGrid::new(&root, border_materials);
///
/// traverse_octree(&grid, &mut |view, coord, subleaf| {
///     if let Some(left) = view.get(OFFSET_LEFT) {
///         // Process with left neighbor
///     }
///     subleaf  // Subdivide if this is a subleaf, otherwise false
/// }, 3);
/// ```
pub fn traverse_octree(grid: &NeighborGrid, visitor: TraversalVisitor, max_depth: u32) {
    // Traverse the center 2x2x2 octants
    for octant_idx in 0..8 {
        let octant_pos = IVec3::from_octant_index(octant_idx);

        // Convert center-based {-1,+1} to grid coordinates [1,2]
        // Formula: grid = (center_based + 3) / 2, where -1 → 1, +1 → 2
        let grid_x = (octant_pos.x + 3) / 2;
        let grid_y = (octant_pos.y + 3) / 2;
        let grid_z = (octant_pos.z + 3) / 2;
        let grid_idx = NeighborGrid::xyz_to_index(grid_x, grid_y, grid_z);

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
    use crate::Cube;

    let voxel = view.center();

    // At maximum depth, this is a final leaf - cannot subdivide further
    if coord.depth == 0 {
        visitor(view, coord, subleaf);
        return;
    }

    match &**voxel {
        Cube::Solid(_) | Cube::Quad { .. } | Cube::Layers { .. } => {
            // Leaf node at depth > 0 - can be subdivided
            let should_subdivide = visitor(view, coord, subleaf); // subleaf = true

            if should_subdivide {
                // Subdivide leaf and traverse children
                // create_child_grid will replicate the solid value into 8 children
                let child_grid = view.create_child_grid();

                for octant_idx in 0..8 {
                    let octant_pos = IVec3::from_octant_index(octant_idx);

                    // Convert center-based {-1,+1} to grid coordinates [1,2]
                    let grid_x = (octant_pos.x + 3) / 2;
                    let grid_y = (octant_pos.y + 3) / 2;
                    let grid_z = (octant_pos.z + 3) / 2;
                    let child_grid_idx = NeighborGrid::xyz_to_index(grid_x, grid_y, grid_z);

                    let child_view = NeighborView::new(&child_grid, child_grid_idx);
                    let child_coord = coord.child(octant_idx);
                    traverse_recursive(child_view, child_coord, visitor, true);
                }
            }
        }
        Cube::Cubes(_children) => {
            // Branch node - traverse children without calling visitor
            let child_grid = view.create_child_grid();

            for octant_idx in 0..8 {
                let octant_pos = IVec3::from_octant_index(octant_idx);

                // Convert center-based {-1,+1} to grid coordinates [1,2]
                let grid_x = (octant_pos.x + 3) / 2;
                let grid_y = (octant_pos.y + 3) / 2;
                let grid_z = (octant_pos.z + 3) / 2;
                let child_grid_idx = NeighborGrid::xyz_to_index(grid_x, grid_y, grid_z);

                let child_view = NeighborView::new(&child_grid, child_grid_idx);
                let child_coord = coord.child(octant_idx);
                traverse_recursive(child_view, child_coord, visitor, false);
            }
        }
    }
}
