// Neighbor-aware octree traversal

use crate::{Cube, IVec3Ext};
use glam::IVec3;
use std::rc::Rc;

pub mod neighbor_grid;
pub mod visit_faces;

// Re-export main types and functions from submodules
pub use neighbor_grid::{
    CubeCoord, NeighborGrid, NeighborView, OFFSET_BACK, OFFSET_DOWN, OFFSET_FRONT, OFFSET_LEFT,
    OFFSET_RIGHT, OFFSET_UP,
};
pub use visit_faces::{
    visit_faces, visit_faces_at_coord, visit_faces_in_region, visit_voxels_in_region, FaceInfo,
    RegionBounds, VoxelInfo,
};

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
/// });
/// ```
pub fn traverse_octree(grid: &NeighborGrid, visitor: TraversalVisitor) {
    // Traverse the center 2x2x2 octants
    for octant_idx in 0..8 {
        // from_octant_index now returns 0/1 coords, convert to center-based -1/+1
        let octant_pos_01 = IVec3::from_octant_index(octant_idx);
        let octant_pos = octant_pos_01 * 2 - IVec3::ONE;

        // Convert center-based {-1,+1} to grid coordinates [1,2]
        // Formula: grid = (center_based + 3) / 2, where -1 → 1, +1 → 2
        let grid_x = (octant_pos.x + 3) / 2;
        let grid_y = (octant_pos.y + 3) / 2;
        let grid_z = (octant_pos.z + 3) / 2;
        let grid_idx = NeighborGrid::xyz_to_index(grid_x, grid_y, grid_z);

        let view = NeighborView::new(grid, grid_idx);
        let coord = CubeCoord::new(octant_pos, 1);
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

    match &**voxel {
        Cube::Solid(_) | Cube::Quad { .. } | Cube::Layers { .. } => {
            // Leaf node at depth > 0 - can be subdivided
            let should_subdivide = visitor(view, coord, subleaf); // subleaf = true

            if should_subdivide {
                // Subdivide leaf and traverse children
                // create_child_grid will replicate the solid value into 8 children
                let child_grid = view.create_child_grid();

                for octant_idx in 0..8 {
                    // from_octant_index now returns 0/1 coords, convert to center-based -1/+1
                    let octant_pos_01 = IVec3::from_octant_index(octant_idx);
                    let octant_pos = octant_pos_01 * 2 - IVec3::ONE;

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
            // Branch node - call visitor to allow pruning
            let should_traverse = visitor(view, coord, false);

            if should_traverse {
                let child_grid = view.create_child_grid();

                for octant_idx in 0..8 {
                    // from_octant_index now returns 0/1 coords, convert to center-based -1/+1
                    let octant_pos_01 = IVec3::from_octant_index(octant_idx);
                    let octant_pos = octant_pos_01 * 2 - IVec3::ONE;

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
}

/// Traverse a rectangular region of the octree at a specific depth
///
/// This function efficiently traverses all voxels in a rectangular region,
/// building shared neighbor context once and reusing it across adjacent cells.
///
/// # Arguments
/// * `root` - The root cube of the octree
/// * `start` - Starting corner of the region (corner-based coordinates at given depth)
/// * `size` - Size of the region in each dimension (must be > 0)
/// * `depth` - Depth level to traverse at
/// * `visitor` - Callback function for each voxel
/// * `border_materials` - Material IDs for border voxels [y0, y1, y2, y3]
///
/// # Coordinate System
/// Uses corner-based coordinates in [0, 2^depth) range, not center-based.
/// - At depth 1: valid positions are 0, 1
/// - At depth 2: valid positions are 0, 1, 2, 3
/// - etc.
///
/// # Performance
/// - Builds region grid once: O(region_size + border)
/// - Shared neighbor context for the entire region
/// - Cache-friendly Z-Y-X iteration order
/// - Early termination via visitor return value
///
/// # Example
/// ```
/// use cube::{Cube, traverse_region, CubeCoord};
/// use glam::IVec3;
///
/// let root = Cube::Solid(1);
/// let mut count = 0;
///
/// traverse_region(
///     &root,
///     IVec3::ZERO,      // Start at corner (0,0,0)
///     IVec3::splat(2),  // 2x2x2 region
///     2,                // At depth 2
///     &mut |view, coord, subleaf| {
///         count += 1;
///         false // Don't subdivide
///     },
///     [0, 0, 0, 0],
/// );
///
/// assert_eq!(count, 8); // Visited 2x2x2 = 8 voxels
/// ```
pub fn traverse_region(
    root: &Cube<u8>,
    start: IVec3,
    size: IVec3,
    depth: u32,
    visitor: TraversalVisitor,
    border_materials: [u8; 4],
) {
    // Validate inputs
    if depth == 0 {
        // At depth 0, there's only the root - just call visitor once
        let grid = NeighborGrid::new(root, border_materials);
        let view = NeighborView::new(&grid, NeighborGrid::xyz_to_index(1, 1, 1));
        let coord = CubeCoord::new(IVec3::ZERO, 0);
        visitor(view, coord, false);
        return;
    }

    if size.x <= 0 || size.y <= 0 || size.z <= 0 {
        return; // Empty region
    }

    // Build the region grid - a grid that covers the entire region plus borders
    let region_grid = build_region_grid(root, start, size, depth, border_materials);
    let grid_size = size + IVec3::splat(2);

    // Iterate over all cells in the region
    for z in 0..size.z {
        for y in 0..size.y {
            for x in 0..size.x {
                let local_pos = IVec3::new(x, y, z);
                let corner_pos = start + local_pos;

                // Convert corner-based to center-based coordinates
                let octree_size = 1 << depth;
                let center_pos = corner_pos * 2 - IVec3::splat(octree_size - 1);
                let coord = CubeCoord::new(center_pos, depth);

                // Get view from region grid (offset by 1 for border)
                let grid_pos = local_pos + IVec3::ONE;

                // Extract a 4x4x4 NeighborGrid for this cell
                let neighbor_grid = extract_neighbor_grid(&region_grid, grid_pos, grid_size);

                // Create view centered on the cell (grid indices 1,1,1 in the 4x4x4 window)
                let view = NeighborView::new(&neighbor_grid, NeighborGrid::xyz_to_index(1, 1, 1));

                // Call visitor
                let voxel = view.center();
                let is_leaf = voxel.is_leaf();
                let should_subdivide = visitor(view, coord, is_leaf);

                if should_subdivide && is_leaf {
                    // Need to subdivide this leaf and traverse children
                    traverse_region_children(view, coord, visitor);
                } else if !is_leaf && should_subdivide {
                    // Branch node - traverse children if visitor says so
                    traverse_region_children(view, coord, visitor);
                }
            }
        }
    }
}

/// Build a grid covering the region plus 1-cell border for neighbor lookups
fn build_region_grid(
    root: &Cube<u8>,
    start: IVec3,
    size: IVec3,
    depth: u32,
    border_materials: [u8; 4],
) -> Vec<Rc<Cube<u8>>> {
    // Grid dimensions: size + 2 (1 border on each side)
    let grid_size = size + IVec3::splat(2);
    let total_cells = (grid_size.x * grid_size.y * grid_size.z) as usize;
    let mut grid = Vec::with_capacity(total_cells);

    // Pre-compute the octree size at this depth
    let octree_size = 1 << depth;

    // Fill the grid
    for z in 0..grid_size.z {
        for y in 0..grid_size.y {
            for x in 0..grid_size.x {
                // Convert grid position to corner-based octree position
                // Grid position 0 is border, 1..size+1 is region, size+1 is border
                let corner_pos = start + IVec3::new(x, y, z) - IVec3::ONE;

                // Check if position is outside the octree bounds [0, 2^depth)
                if corner_pos.x < 0
                    || corner_pos.y < 0
                    || corner_pos.z < 0
                    || corner_pos.x >= octree_size
                    || corner_pos.y >= octree_size
                    || corner_pos.z >= octree_size
                {
                    // Outside bounds - use border material based on Y position
                    // Map Y to [0,3] range for border materials
                    let y_normalized = if corner_pos.y < 0 {
                        0
                    } else if corner_pos.y >= octree_size {
                        3
                    } else {
                        // Map [0, octree_size) to [0, 4)
                        ((corner_pos.y * 4) / octree_size).min(3) as usize
                    };
                    grid.push(Rc::new(Cube::Solid(border_materials[y_normalized])));
                } else {
                    // Inside bounds - navigate to the cube at this position
                    let cube = get_cube_at_corner_pos(root, corner_pos, depth);
                    grid.push(Rc::new(cube.clone()));
                }
            }
        }
    }

    grid
}

/// Get cube at a corner-based position by navigating the octree
///
/// Corner positions are in [0, 2^depth) range.
/// This function correctly navigates using octant indices rather than
/// using Cube::get which relies on center-based coordinate bit extraction.
fn get_cube_at_corner_pos(root: &Cube<u8>, corner_pos: IVec3, depth: u32) -> &Cube<u8> {
    if depth == 0 {
        return root;
    }

    // Navigate down the tree using corner-based position bits
    // At each level, extract the bit at (depth - level - 1) position
    let mut current = root;

    for level in 0..depth {
        let shift = depth - level - 1;
        let octant_bits = (corner_pos >> shift) & 1;
        let octant_idx = octant_bits.to_octant_index();

        match current {
            Cube::Cubes(children) => {
                current = &children[octant_idx];
            }
            // If we hit a leaf before reaching target depth, return it
            _ => return current,
        }
    }

    current
}

/// Convert 3D position to linear index in region grid
#[inline]
fn region_grid_index(pos: IVec3, grid_size: IVec3) -> usize {
    (pos.x + pos.y * grid_size.x + pos.z * grid_size.x * grid_size.y) as usize
}

/// Extract a 4x4x4 NeighborGrid from the region grid centered on given position
fn extract_neighbor_grid(
    region_grid: &[Rc<Cube<u8>>],
    center_pos: IVec3,
    grid_size: IVec3,
) -> NeighborGrid {
    let mut voxels: [Rc<Cube<u8>>; 64] = std::array::from_fn(|_| Rc::new(Cube::Solid(0)));

    for (i, voxel) in voxels.iter_mut().enumerate() {
        let ng_pos = NeighborGrid::index_to_pos(i);
        // NeighborGrid uses [0,3] coordinates, center at [1,2]
        // Map to region grid: ng_pos - 1 gives offset from center
        let offset = ng_pos - IVec3::ONE;
        let region_pos = center_pos + offset;

        if region_pos.x >= 0
            && region_pos.y >= 0
            && region_pos.z >= 0
            && region_pos.x < grid_size.x
            && region_pos.y < grid_size.y
            && region_pos.z < grid_size.z
        {
            let idx = region_grid_index(region_pos, grid_size);
            *voxel = region_grid[idx].clone();
        }
    }

    NeighborGrid { voxels }
}

/// Traverse children of a cell that needs subdivision
fn traverse_region_children(view: NeighborView, coord: CubeCoord, visitor: TraversalVisitor) {
    let child_grid = view.create_child_grid();

    for octant_idx in 0..8 {
        let octant_pos_01 = IVec3::from_octant_index(octant_idx);
        let octant_pos = octant_pos_01 * 2 - IVec3::ONE;

        let grid_x = (octant_pos.x + 3) / 2;
        let grid_y = (octant_pos.y + 3) / 2;
        let grid_z = (octant_pos.z + 3) / 2;
        let child_grid_idx = NeighborGrid::xyz_to_index(grid_x, grid_y, grid_z);

        let child_view = NeighborView::new(&child_grid, child_grid_idx);
        let child_coord = coord.child(octant_idx);
        traverse_recursive(child_view, child_coord, visitor, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traverse_region_basic() {
        // Test basic region traversal
        let root = Cube::Solid(1);
        let mut visited_coords: Vec<CubeCoord> = Vec::new();

        traverse_region(
            &root,
            IVec3::ZERO,
            IVec3::splat(2),
            2,
            &mut |_view, coord, _subleaf| {
                visited_coords.push(coord);
                false // Don't subdivide
            },
            [0, 0, 0, 0],
        );

        // Should visit 2x2x2 = 8 cells at depth 2
        assert_eq!(
            visited_coords.len(),
            8,
            "Should visit 8 cells, got {}",
            visited_coords.len()
        );

        // Verify all coords are at depth 2
        for coord in &visited_coords {
            assert_eq!(coord.depth, 2);
        }
    }

    #[test]
    fn test_traverse_region_single_cell() {
        // Test traversing a single cell
        let root = Cube::Solid(5);
        let mut visited = Vec::new();

        traverse_region(
            &root,
            IVec3::new(1, 1, 1),
            IVec3::ONE,
            2,
            &mut |view, coord, _subleaf| {
                let material = view.center().id();
                visited.push((coord, material));
                false
            },
            [0, 0, 0, 0],
        );

        assert_eq!(visited.len(), 1);
        assert_eq!(visited[0].1, 5); // Material should be 5
    }

    #[test]
    fn test_traverse_region_with_subdivision() {
        // Test with a subdivided cube
        let root = Cube::tabulate(|i| {
            if i % 2 == 0 {
                Cube::Solid(1)
            } else {
                Cube::Solid(0)
            }
        });

        let mut visited_coords: Vec<CubeCoord> = Vec::new();
        let mut materials: Vec<u8> = Vec::new();

        traverse_region(
            &root,
            IVec3::ZERO,
            IVec3::splat(2),
            1,
            &mut |view, coord, _subleaf| {
                visited_coords.push(coord);
                materials.push(view.center().id());
                false // Don't subdivide further
            },
            [0, 0, 0, 0],
        );

        // At depth 1, we have 2x2x2 = 8 cells
        assert_eq!(visited_coords.len(), 8);

        // Check alternating materials (0, 2, 4, 6 are solid 1; 1, 3, 5, 7 are empty 0)
        let solid_count = materials.iter().filter(|&&m| m == 1).count();
        let empty_count = materials.iter().filter(|&&m| m == 0).count();
        assert_eq!(solid_count, 4);
        assert_eq!(empty_count, 4);
    }

    #[test]
    fn test_traverse_region_matches_individual() {
        // Test that traverse_region produces same results as individual coord visits
        let root = Cube::tabulate(|i| {
            if i < 4 {
                Cube::tabulate(|j| Cube::Solid((i * 8 + j) as u8))
            } else {
                Cube::Solid(i as u8)
            }
        });

        let border_materials = [0, 0, 0, 0];
        let start = IVec3::ZERO;
        let size = IVec3::splat(2);
        let depth = 1u32;

        // Collect results from traverse_region
        let mut region_results: Vec<(IVec3, u8)> = Vec::new();
        traverse_region(
            &root,
            start,
            size,
            depth,
            &mut |view, coord, _subleaf| {
                region_results.push((coord.pos, view.center().id()));
                false
            },
            border_materials,
        );

        // Collect results by visiting each coord individually using proper navigation
        // Note: Cube::get() doesn't work correctly for center-based coordinates,
        // so we use get_cube_at_corner_pos instead
        let mut individual_results: Vec<(IVec3, u8)> = Vec::new();
        let octree_size = 1 << depth;
        for z in 0..size.z {
            for y in 0..size.y {
                for x in 0..size.x {
                    let corner_pos = start + IVec3::new(x, y, z);
                    let center_pos = corner_pos * 2 - IVec3::splat(octree_size - 1);
                    // Use proper navigation for corner-based coordinates
                    let cube = get_cube_at_corner_pos(&root, corner_pos, depth);
                    individual_results.push((center_pos, cube.id()));
                }
            }
        }

        // Sort both for comparison
        region_results.sort_by_key(|(pos, _)| (pos.z, pos.y, pos.x));
        individual_results.sort_by_key(|(pos, _)| (pos.z, pos.y, pos.x));

        assert_eq!(
            region_results.len(),
            individual_results.len(),
            "Result count mismatch"
        );

        for (r, i) in region_results.iter().zip(individual_results.iter()) {
            assert_eq!(r.0, i.0, "Position mismatch: {:?} vs {:?}", r.0, i.0);
            assert_eq!(r.1, i.1, "Material mismatch at {:?}: {} vs {}", r.0, r.1, i.1);
        }
    }

    #[test]
    fn test_traverse_region_neighbor_access() {
        // Test that neighbors are correctly accessible
        let root = Cube::tabulate(|i| Cube::Solid(i as u8));

        let mut neighbor_checks = 0;
        let mut neighbor_found = 0;

        traverse_region(
            &root,
            IVec3::ZERO,
            IVec3::splat(2),
            1,
            &mut |view, _coord, _subleaf| {
                // Check all 6 neighbors
                for offset in [
                    OFFSET_LEFT,
                    OFFSET_RIGHT,
                    OFFSET_DOWN,
                    OFFSET_UP,
                    OFFSET_BACK,
                    OFFSET_FRONT,
                ] {
                    neighbor_checks += 1;
                    if view.get(offset).is_some() {
                        neighbor_found += 1;
                    }
                }
                false
            },
            [0, 0, 0, 0],
        );

        // 8 cells * 6 directions = 48 checks
        assert_eq!(neighbor_checks, 48);
        // All neighbors should be found (we have borders)
        assert_eq!(neighbor_found, 48);
    }

    #[test]
    fn test_traverse_region_border_materials() {
        // Test that border materials are correctly applied
        let root = Cube::Solid(0); // Empty root
        let border_materials = [10, 20, 30, 40]; // Different materials for each Y layer

        let mut found_border = false;

        traverse_region(
            &root,
            IVec3::ZERO,
            IVec3::ONE, // Single cell at origin
            2,
            &mut |view, _coord, _subleaf| {
                // Check the -Y neighbor (should be border material)
                if let Some(neighbor) = view.get(OFFSET_DOWN) {
                    if neighbor.id() == 10 {
                        found_border = true;
                    }
                }
                false
            },
            border_materials,
        );

        assert!(found_border, "Should find border material in -Y direction");
    }

    #[test]
    fn test_traverse_region_depth_0() {
        // Test depth 0 case
        let root = Cube::Solid(99);
        let mut count = 0;

        traverse_region(
            &root,
            IVec3::ZERO,
            IVec3::ONE,
            0,
            &mut |view, coord, _subleaf| {
                count += 1;
                assert_eq!(coord.depth, 0);
                assert_eq!(view.center().id(), 99);
                false
            },
            [0, 0, 0, 0],
        );

        assert_eq!(count, 1);
    }

    #[test]
    fn test_traverse_region_empty_size() {
        // Test empty region
        let root = Cube::Solid(1);
        let mut count = 0;

        traverse_region(
            &root,
            IVec3::ZERO,
            IVec3::ZERO, // Empty size
            2,
            &mut |_view, _coord, _subleaf| {
                count += 1;
                false
            },
            [0, 0, 0, 0],
        );

        assert_eq!(count, 0); // Should not visit any cells
    }

}
