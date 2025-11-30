use crate::core::Cube;
use crate::mesh::face::Face;
use crate::traversal::{traverse_octree, CubeCoord, NeighborGrid};
use glam::Vec3;

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

/// Visit all visible faces in the octree (OLD IMPLEMENTATION - BROKEN)
///
/// This is the original implementation that looks FROM empty voxels TOWARD solid voxels.
/// This approach fails when the root is solid or mostly solid, because there are no
/// empty voxels to traverse from.
///
/// Kept for reference. Use `visit_faces` instead.
#[deprecated(note = "Use visit_faces instead - this implementation is broken")]
#[allow(dead_code)]
pub fn visit_faces_old<F>(
    root: &Cube<u8>,
    mut visitor: F,
    max_depth: u32,
    border_materials: [u8; 4],
) where
    F: FnMut(&FaceInfo),
{
    let grid = NeighborGrid::new(root, border_materials);

    traverse_octree(
        &grid,
        &mut |view, coord, _subleaf| {
            // Only process empty voxels - THIS IS THE PROBLEM
            if view.center().id() != 0 {
                return false;
            }

            // Clamp depth to max_depth to avoid underflow
            let depth = coord.depth.min(max_depth);
            let voxel_size = 1.0 / (1 << (max_depth - depth + 1)) as f32;
            let base_pos = coord.pos.as_vec3() * voxel_size;

            let mut should_subdivide = false;

            // Check all 6 directions using Face::DIRECTIONS
            for (face, dir_offset, offset_vec) in Face::DIRECTIONS {
                if let Some(neighbor_cube) = view.get(dir_offset) {
                    // Check if neighbor is subdivided
                    if !neighbor_cube.is_leaf() {
                        should_subdivide = true;
                        continue;
                    }

                    let neighbor_id = neighbor_cube.id();
                    if neighbor_id == 0 {
                        continue; // Skip empty neighbors
                    }

                    // Found a visible face!
                    let face_position = base_pos + offset_vec * voxel_size;
                    visitor(&FaceInfo {
                        face,
                        position: face_position,
                        size: voxel_size,
                        material_id: neighbor_id,
                        viewer_coord: coord,
                    });
                }
            }

            should_subdivide
        },
        max_depth,
    );
}

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
            let center_id = view.center().id();
            if center_id == 0 {
                return false; // Skip empty voxels
            }

            // Calculate voxel size from coord.depth
            // voxel_size = 1.0 / 2^(coord.depth + 1)
            let voxel_size = 1.0 / (2 << coord.depth) as f32;

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
                    if !neighbor_cube.is_leaf() {
                        should_subdivide = true;
                        continue;
                    }

                    let neighbor_id = neighbor_cube.id();
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
        0,
    );
}
