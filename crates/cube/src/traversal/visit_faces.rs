use crate::core::Cube;
use crate::mesh::face::Face;
use crate::traversal::{traverse_octree, CubeCoord, NeighborGrid};
use glam::{IVec3, Vec3};

/// Region bounds for filtering octree traversal
///
/// Represents a rectangular region in the octree coordinate space.
/// Used to limit face traversal to only voxels within the bounded region.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegionBounds {
    /// Base coordinate (minimum corner in center-based coords)
    pub coord: CubeCoord,
    /// Size in each dimension (1 or 2 per axis, representing octant spans)
    pub size: IVec3,
}

impl RegionBounds {
    /// Create new region bounds
    pub fn new(coord: CubeCoord, size: IVec3) -> Self {
        Self { coord, size }
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

        // Size is difference + 1, clamped to valid range
        let size = (max_octant - min_octant + IVec3::ONE).clamp(IVec3::ONE, IVec3::splat(2));

        // Convert from [0, 2^d) coordinates to center-based [-2^d, 2^d) coordinates
        let center_offset: i32 = (1 << depth) - 1;
        let center_based_pos = min_octant * 2 - IVec3::splat(center_offset);

        Some(Self {
            coord: CubeCoord::new(center_based_pos, depth),
            size,
        })
    }

    /// Check if a coordinate is within this region
    ///
    /// Returns true if the coordinate's position is inside the bounded region
    /// at a compatible depth level.
    pub fn contains(&self, coord: &CubeCoord) -> bool {
        // Handle different depth levels
        // If coord is at a different depth, we need to scale appropriately
        let depth_diff = coord.depth as i32 - self.coord.depth as i32;

        let (check_pos, check_size) = if depth_diff > 0 {
            // coord is deeper - scale region to match
            let scale = 1 << depth_diff;
            (self.coord.pos * scale, self.size * scale)
        } else if depth_diff < 0 {
            // coord is shallower - scale coord position to match region depth
            let scale = 1 << (-depth_diff);
            let scaled_pos = coord.pos * scale;
            // A shallower voxel is contained if any part overlaps
            // For simplicity, check if the center would be in range
            return scaled_pos.x >= self.coord.pos.x
                && scaled_pos.x < self.coord.pos.x + self.size.x * 2
                && scaled_pos.y >= self.coord.pos.y
                && scaled_pos.y < self.coord.pos.y + self.size.y * 2
                && scaled_pos.z >= self.coord.pos.z
                && scaled_pos.z < self.coord.pos.z + self.size.z * 2;
        } else {
            (self.coord.pos, self.size)
        };

        let pos = coord.pos;

        // Check each axis - in center-based coords, size of N means span of N*2 units
        let in_x = pos.x >= check_pos.x && pos.x < check_pos.x + check_size.x * 2;
        let in_y = pos.y >= check_pos.y && pos.y < check_pos.y + check_size.y * 2;
        let in_z = pos.z >= check_pos.z && pos.z < check_pos.z + check_size.z * 2;

        in_x && in_y && in_z
    }

    /// Check if the region might contain any voxels at or below the given coordinate
    ///
    /// Used for early termination during traversal - if a branch of the octree
    /// cannot possibly intersect the region, we can skip it entirely.
    pub fn might_contain_descendants(&self, coord: &CubeCoord) -> bool {
        // Calculate the bounds of this coord's subtree
        // At depth d, the subtree spans positions in [-1, 1] relative to coord.pos
        // when scaled to depth d+1, this becomes [-2, 2]

        // For a voxel at coord, its children will have positions:
        // coord.pos * 2 + offset where offset is in {-1, +1}
        // So the range is [coord.pos * 2 - 1, coord.pos * 2 + 1]

        // Scale region bounds to coord's depth for comparison
        let depth_diff = coord.depth as i32 - self.coord.depth as i32;

        if depth_diff >= 0 {
            // coord is at or deeper than region - use contains()
            self.contains(coord)
        } else {
            // coord is shallower than region
            // Scale region bounds up to coord's depth level
            let scale = 1 << (-depth_diff);

            // Region bounds at coord's depth level
            let region_min = self.coord.pos / scale;
            let region_max = (self.coord.pos + self.size * 2 - IVec3::ONE) / scale;

            // Check if coord's cell overlaps with scaled region
            // A cell at depth d has children spanning [pos*2-1, pos*2+1] at depth d+1
            // At the current depth, the cell occupies positions around coord.pos
            let cell_min = coord.pos - IVec3::ONE;
            let cell_max = coord.pos + IVec3::ONE;

            // Overlap test
            cell_max.x >= region_min.x
                && cell_min.x <= region_max.x
                && cell_max.y >= region_min.y
                && cell_min.y <= region_max.y
                && cell_max.z >= region_min.z
                && cell_min.z <= region_max.z
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
    traverse_octree(
        &grid,
        &mut |view, coord, _subleaf| {
            // Only process SOLID voxels (inverted logic from old implementation)
            let center_id = (**view.center()).id();
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
        },
    );
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

    traverse_octree(
        &grid,
        &mut |view, coord, _subleaf| {
            // Early termination: skip branches that cannot contain the region
            if !bounds.might_contain_descendants(&coord) {
                return false;
            }

            // Only process SOLID voxels
            let center_id = (**view.center()).id();
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

                    visitor(&FaceInfo {
                        face,
                        position: base_pos,
                        size: voxel_size,
                        material_id: center_id,
                        viewer_coord: coord,
                    });
                } else {
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
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_bounds_from_local_aabb() {
        // Corner region at depth 1
        let bounds =
            RegionBounds::from_local_aabb(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.4, 0.4, 0.4), 1)
                .unwrap();

        assert_eq!(bounds.octant_count(), 1);
        assert_eq!(bounds.coord.depth, 1);
    }

    #[test]
    fn test_region_bounds_spanning() {
        // Region spanning all octants
        let bounds = RegionBounds::from_local_aabb(Vec3::splat(0.25), Vec3::splat(0.75), 1).unwrap();

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
}
