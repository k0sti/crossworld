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
    pub material_id: i32,
    /// Coordinate of the empty voxel from which the face is visible
    pub viewer_coord: CubeCoord,
}

/// Visit all visible faces in the octree
///
/// A face is visible when an empty voxel borders a solid voxel.
/// The callback receives information about each visible face.
///
/// This is the core face detection algorithm, separated from mesh building
/// for reusability. Can be used for mesh generation, surface area calculation,
/// ambient occlusion, light map baking, debug visualization, etc.
///
/// # Arguments
/// * `root` - The root cube of the octree
/// * `visitor` - Callback invoked for each visible face
/// * `max_depth` - Maximum depth to traverse
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
/// }, 3, [0, 0, 0, 0]);
/// ```
pub fn visit_faces<F>(root: &Cube<i32>, mut visitor: F, max_depth: u32, border_materials: [i32; 4])
where
    F: FnMut(&FaceInfo),
{
    let grid = NeighborGrid::new(root, border_materials);

    traverse_octree(
        &grid,
        &mut |view, coord, _subleaf| {
            let center_id = view.center().id();

            let voxel_size = 1.0 / (1 << (max_depth - coord.depth + 1)) as f32;

            // Convert center-based coordinates to world space [0,1]
            // At depth d, positions range from -(2^(max_depth-d+1)-1) to +(2^(max_depth-d+1)-1) in steps of 2
            // Map this range to [0,1] by: (pos + max_pos) * voxel_size / 2
            // where max_pos = 2^(max_depth-d+1) - 1
            let num_voxels = (1 << (max_depth - coord.depth + 1)) as f32;
            let base_pos = (coord.pos.as_vec3() + num_voxels - 1.0) * voxel_size / 2.0;

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

                    // Generate face if there's a transition between solid and empty
                    let is_viewing_from_empty = center_id == 0 && neighbor_id != 0;
                    let is_viewing_from_solid = center_id != 0 && neighbor_id == 0;

                    if !is_viewing_from_empty && !is_viewing_from_solid {
                        continue;
                    }

                    // Face material: use solid voxel's material
                    let material_id = if is_viewing_from_empty { neighbor_id } else { center_id };

                    // Face position adjustment:
                    // - From empty→solid: face is on neighbor's side, offset points toward it
                    // - From solid→empty: face is on our side, need to negate offset
                    let position_offset = if is_viewing_from_solid {
                        -offset_vec // Invert direction when viewing from solid
                    } else {
                        offset_vec
                    };

                    // Found a visible face!
                    let face_position = base_pos + position_offset * voxel_size;
                    visitor(&FaceInfo {
                        face,
                        position: face_position,
                        size: voxel_size,
                        material_id,
                        viewer_coord: coord,
                    });
                }
            }

            should_subdivide
        },
        max_depth,
    );
}
