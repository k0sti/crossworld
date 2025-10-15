use crate::GeometryData;

pub struct Ground {
    width: usize,
    height: usize,
}

impl Ground {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    pub fn generate_mesh(&self) -> GeometryData {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut normals = Vec::new();
        let mut colors = Vec::new();

        // Generate a simple flat ground plane
        // Each grid cell is 1x1 unit
        for z in 0..self.height {
            for x in 0..self.width {
                let x_pos = x as f32;
                let z_pos = z as f32;
                let y_pos = 0.0; // Flat ground at y=0

                // Vertex indices for this quad (not used in flat grid generation)

                // Create quad vertices (4 corners)
                // Bottom-left
                vertices.push(x_pos);
                vertices.push(y_pos);
                vertices.push(z_pos);

                // Bottom-right
                vertices.push(x_pos + 1.0);
                vertices.push(y_pos);
                vertices.push(z_pos);

                // Top-left
                vertices.push(x_pos);
                vertices.push(y_pos);
                vertices.push(z_pos + 1.0);

                // Top-right
                vertices.push(x_pos + 1.0);
                vertices.push(y_pos);
                vertices.push(z_pos + 1.0);

                // Add normals (pointing up)
                for _ in 0..4 {
                    normals.push(0.0);
                    normals.push(1.0);
                    normals.push(0.0);
                }

                // Add colors (checkerboard pattern)
                let is_light = (x + z) % 2 == 0;
                let color = if is_light {
                    [0.8, 0.8, 0.8]
                } else {
                    [0.6, 0.6, 0.6]
                };

                for _ in 0..4 {
                    colors.push(color[0]);
                    colors.push(color[1]);
                    colors.push(color[2]);
                }

                // Create two triangles for this quad
                let vertex_base = ((z * self.width + x) * 4) as u32;

                // First triangle (bottom-left, bottom-right, top-left)
                indices.push(vertex_base);
                indices.push(vertex_base + 1);
                indices.push(vertex_base + 2);

                // Second triangle (bottom-right, top-right, top-left)
                indices.push(vertex_base + 1);
                indices.push(vertex_base + 3);
                indices.push(vertex_base + 2);
            }
        }

        GeometryData::new(vertices, indices, normals, colors)
    }
}
