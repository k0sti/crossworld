use crate::core::Cube;
use crate::mesh::face::Face;
use crate::traversal::{
    traverse_octree, CubeCoord, NeighborGrid, NeighborView, OFFSET_BACK, OFFSET_DOWN, OFFSET_FRONT,
    OFFSET_LEFT, OFFSET_RIGHT, OFFSET_UP,
};
use glam::{IVec3, Vec3};

/// Region bounds for filtering octree traversal
///
/// Represents a rectangular region in the octree coordinate space.
/// Used to limit face traversal to only voxels within the bounded region.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegionBounds {
    /// Corner-based position at the given depth
    pub pos: IVec3,
    /// Current depth level
    pub depth: u32,
    /// Size in units of 2^depth
    pub size: IVec3,
}

impl RegionBounds {
    /// Create new region bounds
    pub fn new(pos: IVec3, depth: u32, size: IVec3) -> Self {
        Self { pos, depth, size }
    }

    /// Create bounds from local space AABB [0,1]³
    ///
    /// Converts a local-space AABB to octree coordinate bounds at the specified depth.
    ///
    /// # Arguments
    /// * `local_min` - Minimum corner in [0,1] space
    /// * `local_max` - Maximum corner in [0,1] space
    /// * `depth` - Octree depth for bounds resolution
    ///
    /// # Returns
    /// `Some(RegionBounds)` if the AABB intersects [0,1], `None` otherwise
    pub fn from_local_aabb(local_min: Vec3, local_max: Vec3, depth: u32) -> Option<Self> {
        // Quick rejection: AABB outside [0,1] bounds
        if local_max.x < 0.0 || local_min.x > 1.0 {
            return None;
        }
        if local_max.y < 0.0 || local_min.y > 1.0 {
            return None;
        }
        if local_max.z < 0.0 || local_min.z > 1.0 {
            return None;
        }

        // Clamp to [0,1] bounds
        let clamped_min = local_min.max(Vec3::ZERO);
        let clamped_max = local_max.min(Vec3::ONE);

        // Convert to octant coordinates at given depth
        let scale = (1 << depth) as f32;

        // Convert [0,1] coordinates to octant indices
        let min_octant = (clamped_min * scale).floor().as_ivec3();
        let max_octant = ((clamped_max * scale).ceil().as_ivec3() - IVec3::ONE).max(min_octant);

        // Size is difference + 1
        let size = (max_octant - min_octant + IVec3::ONE).max(IVec3::ONE);

        Some(Self {
            pos: min_octant,
            depth,
            size,
        })
    }

    /// Check if a coordinate is within this region
    ///
    /// Returns true if the coordinate's position is inside the bounded region
    /// at a compatible depth level.
    pub fn contains(&self, coord: &CubeCoord) -> bool {
        // Convert center-based CubeCoord to corner-based [0, 2^depth]
        let octree_size = 1 << coord.depth;
        let corner_pos: IVec3 = (coord.pos + IVec3::splat(octree_size - 1)) >> 1;

        let depth_diff = coord.depth as i32 - self.depth as i32;

        let (check_pos, check_size) = if depth_diff > 0 {
            // coord is deeper - scale region to match
            let scale = 1 << depth_diff;
            (self.pos * scale, self.size * scale)
        } else if depth_diff < 0 {
            // coord is shallower - scale coord position to match region depth
            let scale = 1 << (-depth_diff);
            let scaled_pos: IVec3 = corner_pos * scale;
            // A shallower voxel is contained if any part overlaps
            return scaled_pos.x >= self.pos.x
                && scaled_pos.x < self.pos.x + self.size.x
                && scaled_pos.y >= self.pos.y
                && scaled_pos.y < self.pos.y + self.size.y
                && scaled_pos.z >= self.pos.z
                && scaled_pos.z < self.pos.z + self.size.z;
        } else {
            (self.pos, self.size)
        };

        // Check each axis
        let in_x = corner_pos.x >= check_pos.x && corner_pos.x < check_pos.x + check_size.x;
        let in_y = corner_pos.y >= check_pos.y && corner_pos.y < check_pos.y + check_size.y;
        let in_z = corner_pos.z >= check_pos.z && corner_pos.z < check_pos.z + check_size.z;

        in_x && in_y && in_z
    }

    /// Check if the region might contain any voxels at or below the given coordinate
    ///
    /// Used for early termination during traversal - if a branch of the octree
    /// cannot possibly intersect the region, we can skip it entirely.
    pub fn might_contain_descendants(&self, coord: &CubeCoord) -> bool {
        // Convert center-based CubeCoord to corner-based [0, 2^depth]
        let octree_size = 1 << coord.depth;
        let corner_pos: IVec3 = (coord.pos + IVec3::splat(octree_size - 1)) >> 1;

        // Scale region bounds to coord's depth for comparison
        let depth_diff = coord.depth as i32 - self.depth as i32;

        if depth_diff >= 0 {
            // coord is at or deeper than region - use contains()
            self.contains(coord)
        } else {
            // coord is shallower than region
            // Scale region bounds up to coord's depth level
            let shift = (-depth_diff) as u32;

            // Region bounds at coord's depth level (using arithmetic shift for floor division)
            let region_min = self.pos >> shift;
            let region_max = (self.pos + self.size - IVec3::ONE) >> shift;

            // Check if coord's cell overlaps with scaled region
            let cell_pos = corner_pos;

            // Overlap test
            cell_pos.x >= region_min.x
                && cell_pos.x <= region_max.x
                && cell_pos.y >= region_min.y
                && cell_pos.y <= region_max.y
                && cell_pos.z >= region_min.z
                && cell_pos.z <= region_max.z
        }
    }

    /// Number of octants covered (1 to 8)
    pub fn octant_count(&self) -> usize {
        (self.size.x * self.size.y * self.size.z) as usize
    }
}

/// Information about a visible face
///
/// Represents a face where an empty voxel borders a solid voxel.
#[derive(Debug, Clone)]
pub struct FaceInfo {
    /// The face direction
    pub face: Face,
    /// Position of the face in world space [0,1]
    pub position: Vec3,
    /// Size of the voxel
    pub size: f32,
    /// Material ID of the solid voxel
    pub material_id: u8,
    /// Coordinate of the empty voxel from which the face is visible
    pub viewer_coord: CubeCoord,
}

// NOTE: visit_faces_outside was an experimental approach that looked from empty voxels
// toward solid voxels. It had issues with solid roots.
// #[allow(dead_code)]
// pub fn visit_faces_outside<F>(
//     root: &Cube<u8>,
//     mut visitor: F,
//     max_depth: u32,
//     border_materials: [u8; 4],
// ) where
//     F: FnMut(&FaceInfo),
// {
//     let grid = NeighborGrid::new(root, border_materials);

//     traverse_octree(
//         &grid,
//         &mut |view, coord, _subleaf| {
//             // Only process empty voxels - THIS IS THE PROBLEM
//             if (**view.center()).id() != 0 {
//                 return false;
//             }

//             // Clamp depth to max_depth to avoid underflow
//             let depth = coord.depth.min(max_depth);
//             let voxel_size = 1.0 / (1 << (max_depth - depth + 1)) as f32;
//             let base_pos = coord.pos.as_vec3() * voxel_size;

//             let mut should_subdivide = false;

//             // Check all 6 directions using Face::DIRECTIONS
//             for (face, dir_offset, offset_vec) in Face::DIRECTIONS {
//                 if let Some(neighbor_cube) = view.get(dir_offset) {
//                     // Check if neighbor is subdivided
//                     if !(**neighbor_cube).is_leaf() {
//                         should_subdivide = true;
//                         continue;
//                     }

//                     let neighbor_id = (**neighbor_cube).id();
//                     if neighbor_id == 0 {
//                         continue; // Skip empty neighbors
//                     }

//                     // Found a visible face!
//                     let face_position = base_pos + offset_vec * voxel_size;
//                     visitor(&FaceInfo {
//                         face,
//                         position: face_position,
//                         size: voxel_size,
//                         material_id: neighbor_id,
//                         viewer_coord: coord,
//                     });
//                 }
//             }

//             should_subdivide
//         },
//         max_depth,
//     );
// }

/// Visit all visible faces in the octree
///
/// A face is visible when a solid voxel borders an empty voxel.
/// This looks FROM solid voxels TOWARD empty voxels, which correctly handles
/// solid roots and generates boundary faces.
///
/// The callback receives information about each visible face.
///
/// This is the core face detection algorithm, separated from mesh building
/// for reusability. Can be used for mesh generation, surface area calculation,
/// ambient occlusion, light map baking, debug visualization, etc.
///
/// # Arguments
/// * `root` - The root cube of the octree
/// * `visitor` - Callback invoked for each visible face
/// * `border_materials` - Material IDs for the 4 border layers [y0, y1, y2, y3]
///
/// # Example
/// ```
/// use cube::{Cube, visit_faces};
///
/// let root = Cube::Solid(1);
/// visit_faces(&root, |face_info| {
///     println!("Face {:?} at {:?} with material {}",
///         face_info.face, face_info.position, face_info.material_id);
/// }, [0, 0, 0, 0]);
/// ```
pub fn visit_faces<F>(root: &Cube<u8>, mut visitor: F, border_materials: [u8; 4])
where
    F: FnMut(&FaceInfo),
{
    let grid = NeighborGrid::new(root, border_materials);

    // Traverse with depth 0 - we'll visit all octants as leaves
    traverse_octree(&grid, &mut |view, coord, _subleaf| {
        let voxel = view.center();
        if !voxel.is_leaf() {
            return true; // Always traverse branches in full visit
        }

        // Only process SOLID voxels (inverted logic from old implementation)
        let center_id = (**voxel).id();
        if center_id == 0 {
            return false; // Skip empty voxels
        }

        // Calculate voxel size from coord.depth
        // voxel_size = 1.0 / 2^coord.depth
        // At depth 1: size = 0.5, at depth 2: size = 0.25, etc.
        let voxel_size = 1.0 / (1 << coord.depth) as f32;

        // Convert from center-based coordinates to [0,1] world space
        // Center-based coords are in {-1, +1} steps, so we scale by half size
        // Formula: (pos - 1) * (size / 2) + 0.5
        // This maps -1 -> 0.0, 1 -> 0.5 (for size 0.5)
        let half_size = voxel_size * 0.5;
        let base_pos = (coord.pos.as_vec3() - Vec3::splat(1.0)) * half_size + Vec3::splat(0.5);

        // Debug print
        // println!(
        //     "DEBUG: depth={}, pos={}, size={}, half={}, base={}",
        //     coord.depth, coord.pos, voxel_size, half_size, base_pos
        // );

        let mut should_subdivide = false;

        // Check all 6 directions
        // Since we inverted the logic (solid looking at empty instead of empty looking at solid),
        // we need to use the opposite offsets and faces:
        // - Check LEFT neighbor (-X) → render LEFT face
        // - Check RIGHT neighbor (+X) → render RIGHT face
        // etc.
        use crate::traversal::{
            OFFSET_BACK, OFFSET_DOWN, OFFSET_FRONT, OFFSET_LEFT, OFFSET_RIGHT, OFFSET_UP,
        };

        let directions = [
            (Face::Left, OFFSET_LEFT, Vec3::new(-1.0, 0.0, 0.0)),
            (Face::Right, OFFSET_RIGHT, Vec3::new(1.0, 0.0, 0.0)),
            (Face::Bottom, OFFSET_DOWN, Vec3::new(0.0, -1.0, 0.0)),
            (Face::Top, OFFSET_UP, Vec3::new(0.0, 1.0, 0.0)),
            (Face::Back, OFFSET_BACK, Vec3::new(0.0, 0.0, -1.0)),
            (Face::Front, OFFSET_FRONT, Vec3::new(0.0, 0.0, 1.0)),
        ];

        for (face, dir_offset, _offset_vec) in directions {
            if let Some(neighbor_cube) = view.get(dir_offset) {
                // Check if neighbor is subdivided
                if !(**neighbor_cube).is_leaf() {
                    should_subdivide = true;
                    continue;
                }

                let neighbor_id = (**neighbor_cube).id();
                if neighbor_id != 0 {
                    continue; // Skip solid neighbors (no face between two solids)
                }

                // Found a visible face! Solid voxel bordering empty
                // The face is ON the solid voxel, facing toward the empty neighbor
                // We pass the voxel's position because Face::vertices handles the offsets
                visitor(&FaceInfo {
                    face,
                    position: base_pos,
                    size: voxel_size,
                    material_id: center_id, // Use the solid voxel's material
                    viewer_coord: coord,
                });
            } else {
                // Neighbor is None (outside grid bounds) - treat as empty and render face
                visitor(&FaceInfo {
                    face,
                    position: base_pos,
                    size: voxel_size,
                    material_id: center_id,
                    viewer_coord: coord,
                });
            }
        }

        should_subdivide
    });
}

/// Visit visible faces within a bounded region of the octree
///
/// Similar to `visit_faces`, but only processes voxels that fall within
/// the specified region bounds. This significantly reduces the number of
/// faces visited when only a small region of the octree is relevant
/// (e.g., for collision detection in a limited area).
///
/// # Arguments
/// * `root` - The root cube of the octree
/// * `bounds` - Region bounds to filter traversal
/// * `visitor` - Callback invoked for each visible face in the region
/// * `border_materials` - Material IDs for the 4 border layers [y0, y1, y2, y3]
///
/// # Performance
/// - Full traversal: O(n) faces for n voxels
/// - Bounded traversal: O(k) faces for k voxels in region
/// - Typical reduction: 70-90% fewer faces for small regions
///
/// # Example
/// ```
/// use cube::{Cube, visit_faces_in_region, RegionBounds, CubeCoord};
/// use glam::{IVec3, Vec3};
///
/// let root = Cube::Solid(1);
/// let bounds = RegionBounds::from_local_aabb(
///     Vec3::new(0.0, 0.0, 0.0),
///     Vec3::new(0.5, 0.5, 0.5),
///     2
/// ).unwrap();
///
/// visit_faces_in_region(&root, &bounds, |face_info| {
///     println!("Face {:?} at {:?}", face_info.face, face_info.position);
/// }, [0, 0, 0, 0]);
/// ```
pub fn visit_faces_in_region<F>(
    root: &Cube<u8>,
    bounds: &RegionBounds,
    mut visitor: F,
    border_materials: [u8; 4],
) where
    F: FnMut(&FaceInfo),
{
    let grid = NeighborGrid::new(root, border_materials);

    traverse_octree(&grid, &mut |view, coord, _subleaf| {
        // Early termination: skip branches that cannot contain the region
        if !bounds.might_contain_descendants(&coord) {
            return false;
        }

        let voxel = view.center();
        if !voxel.is_leaf() {
            return true; // Continue traversing branch if it might contain region
        }

        // Only process SOLID voxels
        let center_id = (**voxel).id();
        if center_id == 0 {
            return false;
        }

        // Only visit faces for voxels actually within the region
        if !bounds.contains(&coord) {
            // Not in region, but might have descendants in region
            // Continue traversing children
            return true;
        }

        // Calculate voxel position (same as visit_faces)
        let voxel_size = 1.0 / (1 << coord.depth) as f32;
        let half_size = voxel_size * 0.5;
        let base_pos = (coord.pos.as_vec3() - Vec3::splat(1.0)) * half_size + Vec3::splat(0.5);

        let mut should_subdivide = false;

        use crate::traversal::{
            OFFSET_BACK, OFFSET_DOWN, OFFSET_FRONT, OFFSET_LEFT, OFFSET_RIGHT, OFFSET_UP,
        };

        let directions = [
            (Face::Left, OFFSET_LEFT, Vec3::new(-1.0, 0.0, 0.0)),
            (Face::Right, OFFSET_RIGHT, Vec3::new(1.0, 0.0, 0.0)),
            (Face::Bottom, OFFSET_DOWN, Vec3::new(0.0, -1.0, 0.0)),
            (Face::Top, OFFSET_UP, Vec3::new(0.0, 1.0, 0.0)),
            (Face::Back, OFFSET_BACK, Vec3::new(0.0, 0.0, -1.0)),
            (Face::Front, OFFSET_FRONT, Vec3::new(0.0, 0.0, 1.0)),
        ];

        for (face, dir_offset, _offset_vec) in directions {
            if let Some(neighbor_cube) = view.get(dir_offset) {
                if !(**neighbor_cube).is_leaf() {
                    should_subdivide = true;
                    continue;
                }

                let neighbor_id = (**neighbor_cube).id();
                if neighbor_id != 0 {
                    continue;
                }

                // println!("DEBUG: Visited face {:?} at {:?}", face, base_pos);
                visitor(&FaceInfo {
                    face,
                    position: base_pos,
                    size: voxel_size,
                    material_id: center_id,
                    viewer_coord: coord,
                });
            } else {
                // println!("DEBUG: Visited border face {:?} at {:?}", face, base_pos);
                visitor(&FaceInfo {
                    face,
                    position: base_pos,
                    size: voxel_size,
                    material_id: center_id,
                    viewer_coord: coord,
                });
            }
        }

        should_subdivide
    });
}

/// Visit visible faces at and below a specific CubeCoord
///
/// This function navigates directly to the specified coordinate in the octree,
/// builds the appropriate NeighborGrid context, and then traverses faces from
/// that point downward. This is more efficient than `visit_faces_in_region`
/// when you need faces for a single specific octant.
///
/// # Arguments
/// * `root` - The root cube of the octree
/// * `target` - The target CubeCoord to visit
/// * `visitor` - Callback invoked for each visible face
/// * `border_materials` - Material IDs for the 4 border layers [y0, y1, y2, y3]
///
/// # Example
/// ```
/// use cube::{Cube, visit_faces_at_coord, CubeCoord};
/// use glam::IVec3;
///
/// let root = Cube::Solid(1);
/// let coord = CubeCoord::new(IVec3::new(-1, -1, -1), 1);
///
/// visit_faces_at_coord(&root, coord, |face_info| {
///     println!("Face {:?} at {:?}", face_info.face, face_info.position);
/// }, [0, 0, 0, 0]);
/// ```
pub fn visit_faces_at_coord<F>(
    root: &Cube<u8>,
    target: CubeCoord,
    mut visitor: F,
    border_materials: [u8; 4],
) where
    F: FnMut(&FaceInfo),
{
    if target.depth == 0 {
        // Depth 0 means visit the entire root - just use visit_faces
        visit_faces(root, visitor, border_materials);
        return;
    }

    // Build the path of octant indices from root to target
    let path = coord_to_path(target);

    // Start with the root grid
    let root_grid = NeighborGrid::new(root, border_materials);

    // Navigate to the target by building child grids along the path
    visit_faces_at_path(&root_grid, &path, 0, target, &mut visitor);
}

/// Convert a CubeCoord to a path of octant indices from root
fn coord_to_path(coord: CubeCoord) -> Vec<usize> {
    use crate::IVec3Ext;

    let mut path = Vec::with_capacity(coord.depth as usize);

    // Center-based coordinates at depth d use values in {-(2^d-1), ..., 2^d-1} with step 2
    // At depth 1: {-1, +1}
    // At depth 2: {-3, -1, +1, +3}
    // At depth 3: {-7, -5, -3, -1, +1, +3, +5, +7}
    //
    // At each depth level, we need to determine which octant the position falls into.
    // The pattern is: at level i, the bit that determines the octant is at position (depth - i)
    // when we look at the transformed position.
    //
    // Transform: convert center-based to 0-based by adding (2^depth - 1) and dividing by 2
    // This gives us a value in 0..2^depth
    let scale = (1 << coord.depth) - 1;
    let pos_0based = (coord.pos + IVec3::splat(scale)) / 2;

    // Now extract octant indices from most significant to least significant bit
    for level in 1..=coord.depth {
        let shift = coord.depth - level;
        let octant_bits = (pos_0based >> shift) & 1;
        let octant_idx = octant_bits.to_octant_index();
        path.push(octant_idx);
    }

    path
}

/// Navigate to target along path and visit faces
fn visit_faces_at_path<F>(
    grid: &NeighborGrid,
    path: &[usize],
    path_idx: usize,
    target: CubeCoord,
    visitor: &mut F,
) where
    F: FnMut(&FaceInfo),
{
    use crate::IVec3Ext;

    if path_idx >= path.len() {
        // We've reached the target depth - this shouldn't happen
        // as we handle the final step in the else branch
        return;
    }

    let octant_idx = path[path_idx];

    // Convert octant index to grid position
    let octant_pos_01 = IVec3::from_octant_index(octant_idx);
    let octant_pos = octant_pos_01 * 2 - IVec3::ONE;
    let grid_x = (octant_pos.x + 3) / 2;
    let grid_y = (octant_pos.y + 3) / 2;
    let grid_z = (octant_pos.z + 3) / 2;
    let grid_idx = NeighborGrid::xyz_to_index(grid_x, grid_y, grid_z);

    let view = NeighborView::new(grid, grid_idx);

    if path_idx == path.len() - 1 {
        // We're at the target - now traverse faces from here
        let coord = target;
        traverse_faces_from_view(view, coord, visitor);
    } else {
        // Need to go deeper - create child grid and recurse
        let child_grid = view.create_child_grid();
        visit_faces_at_path(&child_grid, path, path_idx + 1, target, visitor);
    }
}

/// Traverse faces starting from a specific view, recursing into children
fn traverse_faces_from_view<F>(view: NeighborView, coord: CubeCoord, visitor: &mut F)
where
    F: FnMut(&FaceInfo),
{
    use crate::IVec3Ext;

    let voxel = view.center();

    if !voxel.is_leaf() {
        // Branch node - recurse into children
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
            traverse_faces_from_view(child_view, child_coord, visitor);
        }
        return;
    }

    // Leaf node - check for visible faces
    let center_id = (**voxel).id();
    if center_id == 0 {
        return; // Skip empty voxels
    }

    // Calculate voxel position
    let voxel_size = 1.0 / (1 << coord.depth) as f32;
    let half_size = voxel_size * 0.5;
    let base_pos = (coord.pos.as_vec3() - Vec3::splat(1.0)) * half_size + Vec3::splat(0.5);

    let mut should_subdivide = false;

    let directions = [
        (Face::Left, OFFSET_LEFT),
        (Face::Right, OFFSET_RIGHT),
        (Face::Bottom, OFFSET_DOWN),
        (Face::Top, OFFSET_UP),
        (Face::Back, OFFSET_BACK),
        (Face::Front, OFFSET_FRONT),
    ];

    for (face, dir_offset) in directions {
        if let Some(neighbor_cube) = view.get(dir_offset) {
            if !(**neighbor_cube).is_leaf() {
                should_subdivide = true;
                continue;
            }

            let neighbor_id = (**neighbor_cube).id();
            if neighbor_id != 0 {
                continue; // Skip solid neighbors
            }

            visitor(&FaceInfo {
                face,
                position: base_pos,
                size: voxel_size,
                material_id: center_id,
                viewer_coord: coord,
            });
        } else {
            // Neighbor is None (outside grid bounds) - treat as empty
            visitor(&FaceInfo {
                face,
                position: base_pos,
                size: voxel_size,
                material_id: center_id,
                viewer_coord: coord,
            });
        }
    }

    // If we need to subdivide due to neighbor resolution mismatch
    if should_subdivide {
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
            traverse_faces_from_view(child_view, child_coord, visitor);
        }
    }
}

/// Information about a visited voxel
#[derive(Debug, Clone)]
pub struct VoxelInfo {
    pub position: Vec3,
    pub size: f32,
    pub material_id: u8,
}

/// Visit all solid voxels within a region
///
/// This is useful for physics collision detection where we need to know about
/// all solid matter, not just surface faces (e.g. for deep penetration).
pub fn visit_voxels_in_region<F>(
    root: &Cube<u8>,
    bounds: &RegionBounds,
    mut visitor: F,
    border_materials: [u8; 4],
) where
    F: FnMut(&VoxelInfo),
{
    let grid = NeighborGrid::new(root, border_materials);

    traverse_octree(&grid, &mut |view, coord, _subleaf| {
        // Early termination: skip branches that cannot contain the region
        if !bounds.might_contain_descendants(&coord) {
            return false;
        }

        let voxel = view.center();
        if !voxel.is_leaf() {
            return true; // Continue traversing branch if it might contain region
        }

        // Only process SOLID voxels
        let center_id = (**voxel).id();
        if center_id == 0 {
            return false;
        }

        // Calculate voxel position
        let voxel_size = 1.0 / (1 << coord.depth) as f32;
        let half_size = voxel_size * 0.5;
        let base_pos = (coord.pos.as_vec3() - Vec3::splat(1.0)) * half_size + Vec3::splat(0.5);

        visitor(&VoxelInfo {
            position: base_pos,
            size: voxel_size,
            material_id: center_id,
        });

        false // Stop traversing this branch (it's a leaf)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IVec3Ext;

    #[test]
    fn test_region_bounds_from_local_aabb() {
        // Corner region at depth 1
        let bounds =
            RegionBounds::from_local_aabb(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.4, 0.4, 0.4), 1)
                .unwrap();

        assert_eq!(bounds.octant_count(), 1);
        assert_eq!(bounds.depth, 1);
    }

    #[test]
    fn test_region_bounds_spanning() {
        // Region spanning all octants
        let bounds =
            RegionBounds::from_local_aabb(Vec3::splat(0.25), Vec3::splat(0.75), 1).unwrap();

        assert_eq!(bounds.octant_count(), 8);
    }

    #[test]
    fn test_region_bounds_outside() {
        // Region outside [0,1] cube
        let bounds = RegionBounds::from_local_aabb(Vec3::splat(2.0), Vec3::splat(3.0), 1);

        assert!(bounds.is_none());
    }

    #[test]
    fn test_region_bounds_contains() {
        let bounds =
            RegionBounds::from_local_aabb(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.4, 0.4, 0.4), 1)
                .unwrap();

        // Coordinate in bottom-left-back corner should be contained
        let inside = CubeCoord::new(IVec3::new(-1, -1, -1), 1);
        assert!(bounds.contains(&inside));

        // Coordinate in opposite corner should not be contained
        let outside = CubeCoord::new(IVec3::new(1, 1, 1), 1);
        assert!(!bounds.contains(&outside));
    }

    #[test]
    fn test_visit_faces_in_region_reduces_count() {
        // Create a subdivided cube for a fair comparison
        // (solid root gets expanded during traversal)
        let root = Cube::tabulate(|_| Cube::Solid(1));

        // Count all faces (each of 8 octants contributes faces)
        let mut full_count = 0;
        visit_faces(&root, |_| full_count += 1, [0, 0, 0, 0]);

        // Count faces in corner region (should be fewer)
        // This region covers only the bottom-left-back octant
        let bounds =
            RegionBounds::from_local_aabb(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.4, 0.4, 0.4), 1)
                .unwrap();

        let mut region_count = 0;
        visit_faces_in_region(&root, &bounds, |_| region_count += 1, [0, 0, 0, 0]);

        // The subdivided solid cube produces many internal faces + 6 outer faces
        // The corner region should only have the outer faces of that octant (3 faces)
        assert!(
            region_count < full_count,
            "Region count {} should be < full count {}",
            region_count,
            full_count
        );
        // Corner octant has 3 exposed outer faces
        assert!(
            region_count <= 6,
            "Corner region should have at most 6 faces, got {}",
            region_count
        );
    }

    #[test]
    fn test_visit_faces_in_region_subdivided() {
        // Create a subdivided cube
        let root = Cube::tabulate(|i| {
            if i < 4 {
                Cube::Solid(1) // Bottom 4 octants solid
            } else {
                Cube::Solid(0) // Top 4 octants empty
            }
        });

        // Count faces in bottom corner
        let bounds =
            RegionBounds::from_local_aabb(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.4, 0.4, 0.4), 2)
                .unwrap();

        let mut faces = Vec::new();
        visit_faces_in_region(&root, &bounds, |f| faces.push(f.clone()), [0, 0, 0, 0]);

        // Should have found some faces
        assert!(!faces.is_empty(), "Should find faces in region");
    }
    #[test]
    fn test_visit_faces_in_region_ground_collision() {
        // Create a root cube of air
        let root = Cube::Solid(0);

        // Expand once with ground at bottom (material 1) and air at top (material 0)
        // border_materials: [y0, y1, y2, y3]
        // y0, y1 are bottom half (ground), y2, y3 are top half (air)
        let border_materials = [1, 1, 0, 0];
        let expanded = Cube::expand_once(&root, border_materials);

        // The expanded cube has depth 2 (4x4x4 voxels)
        // Center 2x2x2 is the original root (air)
        // Bottom layer (y=0) is ground (material 1)

        // Define a region that covers the entire bottom-left-back octant (Octant 0)
        // This octant contains both ground (outer) and air (inner corner)
        // So we should see faces between them.
        let bounds = RegionBounds::from_local_aabb(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5), // Covers Octant 0
            2,                        // Depth of the expanded cube
        )
        .unwrap();

        let mut faces_found = 0;
        visit_faces_in_region(
            &expanded,
            &bounds,
            |info| {
                // We expect to see faces from the ground voxel at (0,0,0)
                if info.material_id == 1 {
                    faces_found += 1;
                }
            },
            border_materials,
        );

        assert!(faces_found > 0, "Should find ground faces in the region");
    }

    #[test]
    fn test_visit_faces_at_coord_matches_region() {
        // Test that visit_faces_at_coord produces the same results as
        // visit_faces_in_region when querying a single octant

        // Create a more complex cube with mixed content
        let root = Cube::tabulate(|i| {
            if i % 2 == 0 {
                Cube::Solid(1) // Alternate solid
            } else {
                Cube::Solid(0) // Alternate empty
            }
        });

        let border_materials = [0, 0, 0, 0];

        // Test all 8 depth-1 octants
        for octant_idx in 0..8 {
            // Create CubeCoord for this octant
            let octant_pos_01 = IVec3::from_octant_index(octant_idx);
            let octant_pos = octant_pos_01 * 2 - IVec3::ONE;
            let coord = CubeCoord::new(octant_pos, 1);

            // Collect faces using visit_faces_at_coord
            let mut coord_faces: Vec<FaceInfo> = Vec::new();
            visit_faces_at_coord(
                &root,
                coord,
                |f| coord_faces.push(f.clone()),
                border_materials,
            );

            // Create equivalent region bounds for the same octant
            // Convert octant to local AABB [0,1]
            let local_min = octant_pos_01.as_vec3() * 0.5;
            let local_max = local_min + Vec3::splat(0.5);
            let bounds = RegionBounds::from_local_aabb(local_min, local_max, 1).unwrap();

            // Collect faces using visit_faces_in_region
            let mut region_faces: Vec<FaceInfo> = Vec::new();
            visit_faces_in_region(
                &root,
                &bounds,
                |f| region_faces.push(f.clone()),
                border_materials,
            );

            // Compare results - same count
            assert_eq!(
                coord_faces.len(),
                region_faces.len(),
                "Octant {}: coord_faces={} != region_faces={}",
                octant_idx,
                coord_faces.len(),
                region_faces.len()
            );

            // Sort both by position and face for comparison
            let sort_key = |f: &FaceInfo| {
                (
                    (f.position.x * 1000.0) as i32,
                    (f.position.y * 1000.0) as i32,
                    (f.position.z * 1000.0) as i32,
                    f.face as u8,
                )
            };
            coord_faces.sort_by_key(sort_key);
            region_faces.sort_by_key(sort_key);

            // Compare each face
            for (i, (cf, rf)) in coord_faces.iter().zip(region_faces.iter()).enumerate() {
                assert_eq!(
                    cf.face, rf.face,
                    "Octant {}, face {}: face mismatch {:?} vs {:?}",
                    octant_idx, i, cf.face, rf.face
                );
                assert!(
                    (cf.position - rf.position).length() < 0.001,
                    "Octant {}, face {}: position mismatch {:?} vs {:?}",
                    octant_idx,
                    i,
                    cf.position,
                    rf.position
                );
                assert_eq!(
                    cf.size, rf.size,
                    "Octant {}, face {}: size mismatch {} vs {}",
                    octant_idx, i, cf.size, rf.size
                );
                assert_eq!(
                    cf.material_id, rf.material_id,
                    "Octant {}, face {}: material mismatch {} vs {}",
                    octant_idx, i, cf.material_id, rf.material_id
                );
            }
        }
    }

    #[test]
    fn test_visit_faces_at_coord_deeper() {
        // Test with a deeper octree structure
        let root = Cube::tabulate(|i| {
            if i < 4 {
                // Bottom 4 octants are subdivided
                Cube::tabulate(|j| {
                    if j % 3 == 0 {
                        Cube::Solid(2)
                    } else {
                        Cube::Solid(0)
                    }
                })
            } else {
                // Top 4 octants are solid
                Cube::Solid(1)
            }
        });

        let border_materials = [0, 0, 0, 0];

        // Test a depth-2 coordinate (inside the subdivided bottom octants)
        // Octant 0 at depth 1 is at position (-1, -1, -1)
        // Its child octant 0 at depth 2 is at position (-3, -3, -3)
        let coord = CubeCoord::new(IVec3::new(-3, -3, -3), 2);

        let mut coord_faces: Vec<FaceInfo> = Vec::new();
        visit_faces_at_coord(
            &root,
            coord,
            |f| coord_faces.push(f.clone()),
            border_materials,
        );

        // Create equivalent region bounds
        // Depth 2 means 4x4x4 grid, this coord is in octant (0,0,0)
        let bounds = RegionBounds::from_local_aabb(Vec3::ZERO, Vec3::splat(0.25), 2).unwrap();

        let mut region_faces: Vec<FaceInfo> = Vec::new();
        visit_faces_in_region(
            &root,
            &bounds,
            |f| region_faces.push(f.clone()),
            border_materials,
        );

        assert_eq!(
            coord_faces.len(),
            region_faces.len(),
            "Deep coord: coord_faces={} != region_faces={}",
            coord_faces.len(),
            region_faces.len()
        );
    }

    #[test]
    fn test_visit_faces_at_coord_solid_root() {
        // Test with solid root (should still work correctly)
        let root = Cube::Solid(5);
        let border_materials = [0, 0, 0, 0];

        // Test depth-1 coordinate
        let coord = CubeCoord::new(IVec3::new(-1, -1, -1), 1);

        let mut coord_faces: Vec<FaceInfo> = Vec::new();
        visit_faces_at_coord(
            &root,
            coord,
            |f| coord_faces.push(f.clone()),
            border_materials,
        );

        // A solid octant at depth 1 with all-empty borders should have 3 outer faces
        // (the corner octant exposes 3 faces to the outside)
        assert!(
            coord_faces.len() >= 3,
            "Solid octant should have at least 3 faces, got {}",
            coord_faces.len()
        );

        // Verify all faces have correct material
        for face in &coord_faces {
            assert_eq!(
                face.material_id, 5,
                "Face should have material 5, got {}",
                face.material_id
            );
        }
    }
}
