use super::voxel_model::VoxelModel;

#[derive(Clone, Copy, Debug)]
enum Face {
    Top,
    Bottom,
    Left,
    Right,
    Front,
    Back,
}

pub struct VoxelMesher<'a> {
    model: &'a VoxelModel,
}

impl<'a> VoxelMesher<'a> {
    pub fn new(model: &'a VoxelModel) -> Self {
        Self { model }
    }

    /// Generate mesh from voxel model using greedy meshing
    pub fn generate_mesh(
        &self,
        palette: &VoxelPalette,
    ) -> (Vec<f32>, Vec<u32>, Vec<f32>, Vec<f32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut normals = Vec::new();
        let mut colors = Vec::new();
        let mut vertex_count = 0u32;

        let voxel_size = 0.1; // Scale voxels to reasonable avatar size

        // For each voxel, check all 6 faces for visibility
        for voxel in &self.model.voxels {
            let x = voxel.x as f32 * voxel_size;
            let y = voxel.y as f32 * voxel_size;
            let z = voxel.z as f32 * voxel_size;

            let color = palette.get_color(voxel.color_index);

            // Check each face
            let faces = [
                (0, 1, 0, Face::Top),
                (0, -1, 0, Face::Bottom),
                (-1, 0, 0, Face::Left),
                (1, 0, 0, Face::Right),
                (0, 0, 1, Face::Front),
                (0, 0, -1, Face::Back),
            ];

            for (dx, dy, dz, face) in faces {
                let nx = voxel.x as i32 + dx;
                let ny = voxel.y as i32 + dy;
                let nz = voxel.z as i32 + dz;

                // Check if neighbor position is out of bounds or empty
                let is_visible = nx < 0
                    || ny < 0
                    || nz < 0
                    || nx >= self.model.size_x as i32
                    || ny >= self.model.size_y as i32
                    || nz >= self.model.size_z as i32
                    || !self
                        .model
                        .has_voxel_at(nx as u8, ny as u8, nz as u8);

                if is_visible {
                    let (face_verts, face_norms) = get_face_geometry(face, x, y, z, voxel_size);

                    // Add vertices and normals
                    vertices.extend_from_slice(&face_verts);
                    normals.extend_from_slice(&face_norms);

                    // Add color for each vertex (4 per face)
                    for _ in 0..4 {
                        colors.extend_from_slice(&color);
                    }

                    // Add indices for two triangles
                    indices.extend_from_slice(&[
                        vertex_count,
                        vertex_count + 1,
                        vertex_count + 2,
                        vertex_count,
                        vertex_count + 2,
                        vertex_count + 3,
                    ]);

                    vertex_count += 4;
                }
            }
        }

        (vertices, indices, normals, colors)
    }
}

/// Get vertex positions and normals for a face
fn get_face_geometry(face: Face, x: f32, y: f32, z: f32, size: f32) -> (Vec<f32>, Vec<f32>) {
    let vertices = match face {
        Face::Top => vec![
            x,
            y + size,
            z,
            x + size,
            y + size,
            z,
            x + size,
            y + size,
            z + size,
            x,
            y + size,
            z + size,
        ],
        Face::Bottom => vec![
            x,
            y,
            z + size,
            x + size,
            y,
            z + size,
            x + size,
            y,
            z,
            x,
            y,
            z,
        ],
        Face::Left => vec![
            x, y, z, x, y, z + size, x, y + size, z + size, x, y + size, z,
        ],
        Face::Right => vec![
            x + size,
            y,
            z + size,
            x + size,
            y,
            z,
            x + size,
            y + size,
            z,
            x + size,
            y + size,
            z + size,
        ],
        Face::Front => vec![
            x,
            y,
            z + size,
            x + size,
            y,
            z + size,
            x + size,
            y + size,
            z + size,
            x,
            y + size,
            z + size,
        ],
        Face::Back => vec![
            x + size,
            y,
            z,
            x,
            y,
            z,
            x,
            y + size,
            z,
            x + size,
            y + size,
            z,
        ],
    };

    let normal = match face {
        Face::Top => [0.0, 1.0, 0.0],
        Face::Bottom => [0.0, -1.0, 0.0],
        Face::Left => [-1.0, 0.0, 0.0],
        Face::Right => [1.0, 0.0, 0.0],
        Face::Front => [0.0, 0.0, 1.0],
        Face::Back => [0.0, 0.0, -1.0],
    };

    let mut normals = Vec::new();
    for _ in 0..4 {
        normals.extend_from_slice(&normal);
    }

    (vertices, normals)
}
