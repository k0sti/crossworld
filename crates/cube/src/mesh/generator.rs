use crate::core::Cube;
use crate::traversal::visit_faces;

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
    root: &Cube<u8>,
    builder: &mut B,
    color_fn: F,
    border_materials: [u8; 4],
    base_depth: u32,
) where
    B: MeshBuilder,
    F: Fn(u8) -> [f32; 3] + Copy,
{
    // Use visit_faces to iterate through all visible faces
    visit_faces(
        root,
        |face_info| {
            // Extract face position and size
            let position = face_info.position;
            let size = face_info.size;

            // Generate vertices for this face
            let vertices = face_info
                .face
                .vertices(position.x, position.y, position.z, size);
            let normal = face_info.face.normal();
            let color = color_fn(face_info.material_id);

            // Check if material needs texture (2-127 are textured materials)
            if (2..=127).contains(&face_info.material_id) {
                // Use a constant UV scale across all depths
                // Scale by 2^base_depth for proper tiling
                let uv_scale = (1 << base_depth) as f32;

                // Generate UVs with world position for seamless tiling
                let uvs = face_info
                    .face
                    .uvs(position.x, position.y, position.z, size, uv_scale);
                builder.add_textured_face(vertices, normal, color, uvs, face_info.material_id);
            } else {
                // Solid color material (0-1, 128-255)
                builder.add_face(vertices, normal, color);
            }
        },
        border_materials,
    );
}
