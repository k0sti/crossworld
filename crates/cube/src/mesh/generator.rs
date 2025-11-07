use crate::mesh::face::Face;
use crate::traversal::{
    traverse_octree, traverse_with_neighbors, CubeCoord, NeighborGrid, OFFSET_BACK, OFFSET_DOWN,
    OFFSET_FRONT, OFFSET_LEFT, OFFSET_RIGHT, OFFSET_UP,
};
use crate::core::Cube;
use glam::Vec3;

/// Builder interface for constructing meshes
pub trait MeshBuilder {
    /// Add a single face to the mesh
    ///
    /// # Arguments
    /// * `vertices` - Four vertices forming a quad (counter-clockwise)
    /// * `normal` - Normal vector for the face
    /// * `color` - RGB color for the face
    fn add_face(&mut self, vertices: [[f32; 3]; 4], normal: [f32; 3], color: [f32; 3]);

    /// Add a single textured face to the mesh
    ///
    /// # Arguments
    /// * `vertices` - Four vertices forming a quad (counter-clockwise)
    /// * `normal` - Normal vector for the face
    /// * `color` - RGB color for the face
    /// * `uvs` - UV coordinates for each vertex
    /// * `material_id` - Material index for texture lookup
    fn add_textured_face(
        &mut self,
        vertices: [[f32; 3]; 4],
        normal: [f32; 3],
        color: [f32; 3],
        uvs: [[f32; 2]; 4],
        material_id: u8,
    );
}

/// Default mesh builder that accumulates data into vectors
pub struct DefaultMeshBuilder {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
    pub normals: Vec<f32>,
    pub colors: Vec<f32>,
    pub uvs: Vec<f32>,
    pub material_ids: Vec<u8>,
    vertex_count: u32,
}

impl DefaultMeshBuilder {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            normals: Vec::new(),
            colors: Vec::new(),
            uvs: Vec::new(),
            material_ids: Vec::new(),
            vertex_count: 0,
        }
    }
}

impl Default for DefaultMeshBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshBuilder for DefaultMeshBuilder {
    fn add_face(&mut self, vertices: [[f32; 3]; 4], normal: [f32; 3], color: [f32; 3]) {
        let base_index = self.vertex_count;

        // Add vertices
        for vertex in &vertices {
            self.vertices.extend_from_slice(vertex);
            self.normals.extend_from_slice(&normal);
            self.colors.extend_from_slice(&color);
            // Default UVs (not used for solid color materials)
            self.uvs.extend_from_slice(&[0.0, 0.0]);
            self.material_ids.push(0); // 0 = no texture
        }

        // Add indices for two triangles (0,1,2) and (0,2,3)
        self.indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);

        self.vertex_count += 4;
    }

    fn add_textured_face(
        &mut self,
        vertices: [[f32; 3]; 4],
        normal: [f32; 3],
        color: [f32; 3],
        uvs: [[f32; 2]; 4],
        material_id: u8,
    ) {
        let base_index = self.vertex_count;

        // Add vertices with UVs and material ID
        for (i, vertex) in vertices.iter().enumerate() {
            self.vertices.extend_from_slice(vertex);
            self.normals.extend_from_slice(&normal);
            self.colors.extend_from_slice(&color);
            self.uvs.extend_from_slice(&uvs[i]);
            self.material_ids.push(material_id);
        }

        // Add indices for two triangles (0,1,2) and (0,2,3)
        self.indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);

        self.vertex_count += 4;
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
/// use crossworld_cube::{Cube, visit_faces};
///
/// let root = Cube::Solid(1);
/// visit_faces(&root, |face_info| {
///     println!("Face {:?} at {:?} with material {}",
///         face_info.face, face_info.position, face_info.material_id);
/// }, 3, [0, 0, 0, 0]);
/// ```
pub fn visit_faces<F>(
    root: &Cube<i32>,
    mut visitor: F,
    max_depth: u32,
    border_materials: [i32; 4],
) where
    F: FnMut(&FaceInfo),
{
    let grid = NeighborGrid::new(root, border_materials);

    traverse_octree(&grid, &mut |view, coord, _subleaf| {
        // Only process empty voxels
        if view.center().id() != 0 {
            return false;
        }

        let voxel_size = 1.0 / (1 << (max_depth - coord.depth + 1)) as f32;
        let base_pos = coord.pos.as_vec3() * voxel_size;

        // Check all 6 directions
        const DIRECTIONS: [(Face, i32, Vec3); 6] = [
            (Face::Right, OFFSET_LEFT, Vec3::new(-1.0, 0.0, 0.0)),
            (Face::Left, OFFSET_RIGHT, Vec3::new(1.0, 0.0, 0.0)),
            (Face::Top, OFFSET_DOWN, Vec3::new(0.0, -1.0, 0.0)),
            (Face::Bottom, OFFSET_UP, Vec3::new(0.0, 1.0, 0.0)),
            (Face::Front, OFFSET_BACK, Vec3::new(0.0, 0.0, -1.0)),
            (Face::Back, OFFSET_FRONT, Vec3::new(0.0, 0.0, 1.0)),
        ];

        let mut should_subdivide = false;

        for (face, dir_offset, offset_vec) in DIRECTIONS {
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
                    viewer_coord: coord.clone(),
                });
            }
        }

        should_subdivide
    }, max_depth);
}

/// Generate mesh from octree using neighbor-aware face culling
///
/// This function only generates faces where empty voxels meet solid voxels.
/// Each face is rendered from the empty space looking into the solid voxel.
///
/// # Arguments
/// * `root` - The root cube of the octree
/// * `builder` - MeshBuilder to receive the faces
/// * `color_fn` - Function to map voxel values to colors
/// * `max_depth` - Maximum depth of the octree
/// * `border_materials` - Array of 4 material IDs for border voxels at each Y layer [y0, y1, y2, y3]
/// * `base_depth` - The depth at which voxels are 1 unit in size (for UV scaling)
pub fn generate_face_mesh<B, F>(
    root: &Cube<i32>,
    builder: &mut B,
    color_fn: F,
    max_depth: u32,
    border_materials: [i32; 4],
    base_depth: u32,
) where
    B: MeshBuilder,
    F: Fn(i32) -> [f32; 3] + Copy,
{
    // Create initial neighbor grid and traverse
    let grid = NeighborGrid::new(root, border_materials);
    let mut face_count = 0;
    traverse_with_neighbors(
        &grid,
        &mut |view, coord, _subleaf| {
            // Calculate voxel size based on actual depth of this voxel
            // coord.depth counts down from max_depth as we traverse
            // Coordinate space: positions are in [0, 2^(max_depth - coord.depth + 1))
            // because octants start in [0, 2) space and double each level down
            let voxel_size = 1.0 / (1 << (max_depth - coord.depth + 1)) as f32;

            // Calculate position in normalized [0,1] space
            let x = coord.pos.x as f32 * voxel_size;
            let y = coord.pos.y as f32 * voxel_size;
            let z = coord.pos.z as f32 * voxel_size;

            // Check all 6 neighbors for subdivision requirements and face generation
            // For each direction, we render the opposite face of the solid neighbor
            // with normal pointing from solid into empty space
            const DIRECTIONS: [(Face, i32, f32, f32, f32); 6] = [
                (Face::Right, OFFSET_LEFT, -1.0, 0.0, 0.0), // Left neighbor: render RIGHT face
                (Face::Left, OFFSET_RIGHT, 1.0, 0.0, 0.0),  // Right neighbor: render LEFT face
                (Face::Top, OFFSET_DOWN, 0.0, -1.0, 0.0),   // Down neighbor: render TOP face
                (Face::Bottom, OFFSET_UP, 0.0, 1.0, 0.0),   // Up neighbor: render BOTTOM face
                (Face::Front, OFFSET_BACK, 0.0, 0.0, -1.0), // Back neighbor: render FRONT face
                (Face::Back, OFFSET_FRONT, 0.0, 0.0, 1.0),  // Front neighbor: render BACK face
            ];

            let mut should_subdivide = false;

            for (face, dir_offset, dx, dy, dz) in DIRECTIONS {
                if let Some(neighbor_cube) = view.get(dir_offset) {
                    // Check if neighbor is subdivided (branch node)
                    // If so, we need to subdivide this voxel to match the detail level
                    if !neighbor_cube.is_leaf() {
                        should_subdivide = true;
                        continue; // Don't generate face, need to subdivide first
                    }

                    let center_id = view.center().id();
                    // Only process empty voxels
                    if center_id != 0 {
                        continue;
                    }
                    let neighbor_id = neighbor_cube.id();
                    // Skip empty neighbors
                    if neighbor_id == 0 {
                        continue;
                    }

                    // Neighbor is a leaf at the same depth - generate face
                    let nx = x + dx * voxel_size;
                    let ny = y + dy * voxel_size;
                    let nz = z + dz * voxel_size;
                    let vertices = face.vertices(nx, ny, nz, voxel_size);
                    let normal = face.normal();
                    let color = color_fn(neighbor_id);

                    // Check if material needs texture (2-127 are textured materials)
                    if (2..=127).contains(&neighbor_id) {
                        // Use a constant UV scale across all depths
                        // Scale by 2^base_depth for proper tiling
                        let uv_scale = (1 << base_depth) as f32;

                        // Generate UVs with world position for seamless tiling
                        // Pass the actual voxel_size separately for UV bounds calculation
                        let uvs = face.uvs(nx, ny, nz, voxel_size, uv_scale);
                        builder.add_textured_face(vertices, normal, color, uvs, neighbor_id as u8);
                    } else {
                        // Solid color material (0-1, 128-255)
                        builder.add_face(vertices, normal, color);
                    }
                    face_count += 1;
                }
            }

            // Subdivide if any neighbor is octa
            should_subdivide
        },
        max_depth,
    );

    // tracing::debug!(
    //     "[generate_face_mesh] Generated {} faces at max_depth={}",
    //     face_count,
    //     max_depth
    // );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Cube;
    use std::rc::Rc;

    fn simple_color_mapper(value: i32) -> [f32; 3] {
        if value <= 0 {
            [0.0, 0.0, 0.0]
        } else {
            [1.0, 0.0, 0.0] // Simple red color for testing
        }
    }

    #[test]
    fn test_single_solid_cube() {
        // A single solid cube should have all 6 faces rendered from surrounding empty space
        let root = Cube::Solid(1);
        let mut builder = DefaultMeshBuilder::new();
        let border_materials = [0, 0, 0, 0]; // All empty borders

        generate_face_mesh(
            &root,
            &mut builder,
            simple_color_mapper,
            0,
            border_materials,
            0, // base_depth
        );

        // Should have no faces because there are no empty voxels in a solid cube
        // The traversal only processes the octants, and a solid cube has no subdivision
        assert_eq!(builder.indices.len(), 0);
    }

    #[test]
    fn test_checkerboard_pattern() {
        // Create octree with checkerboard solid/empty pattern
        let root = Cube::cubes([
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(1)), // Solid
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(1)), // Solid
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(1)), // Solid
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(1)), // Solid
        ]);

        let mut builder = DefaultMeshBuilder::new();
        let border_materials = [0, 0, 0, 0]; // All empty borders
        generate_face_mesh(
            &root,
            &mut builder,
            simple_color_mapper,
            1,
            border_materials,
            1, // base_depth
        );

        // Empty voxels should have faces where they touch solid voxels
        // 4 empty voxels, each can have multiple solid neighbors
        assert!(builder.indices.len() > 0, "Should generate some faces");
    }

    #[test]
    fn test_all_empty_with_borders() {
        // All empty octree will generate faces where empty voxels meet border ground
        let root = Cube::cubes([
            Rc::new(Cube::Solid(0)), // All empty
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
        ]);

        let mut builder = DefaultMeshBuilder::new();
        let border_materials = [33, 33, 0, 0]; // Ground at bottom, air at top
        generate_face_mesh(
            &root,
            &mut builder,
            simple_color_mapper,
            1,
            border_materials,
            1, // base_depth
        );

        // Empty voxels at the border will see ground (33) below them and generate upward faces
        // This is expected behavior for terrain rendering
        assert!(
            builder.indices.len() > 0,
            "Empty voxels touching ground borders should generate faces"
        );
    }

    #[test]
    fn test_single_solid_in_empty() {
        // One solid voxel surrounded by empty
        let root = Cube::cubes([
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(1)), // Solid - octant 3 (0,1,1)
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(0)), // Empty
        ]);

        let mut builder = DefaultMeshBuilder::new();
        let border_materials = [0, 0, 0, 0]; // All empty borders
        generate_face_mesh(
            &root,
            &mut builder,
            simple_color_mapper,
            1,
            border_materials,
            1, // base_depth
        );

        // Empty neighbors of the solid voxel should render faces towards it
        // Expect at least some faces
        assert!(
            builder.indices.len() > 0,
            "Should generate faces for visible solid"
        );
    }

    #[test]
    fn test_all_solid() {
        // All solid octree should generate no faces (no empty voxels to render from)
        let root = Cube::cubes([
            Rc::new(Cube::Solid(1)), // All solid
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(1)),
        ]);

        let mut builder = DefaultMeshBuilder::new();
        let border_materials = [0, 0, 0, 0]; // All empty borders
        generate_face_mesh(
            &root,
            &mut builder,
            simple_color_mapper,
            1,
            border_materials,
            1, // base_depth
        );

        // No empty voxels means no faces are rendered (we only render from empty voxels)
        assert_eq!(
            builder.indices.len(),
            0,
            "All solid octree should have no faces"
        );
    }
}
