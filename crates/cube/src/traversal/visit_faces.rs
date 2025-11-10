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
            // Only process empty voxels
            if view.center().id() != 0 {
                return false;
            }

            let voxel_size = 1.0 / (1 << (max_depth - coord.depth + 1)) as f32;
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
