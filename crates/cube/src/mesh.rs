use crate::octree::Octree;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshData {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
    pub normals: Vec<f32>,
    pub colors: Vec<f32>,
}

impl MeshData {
    pub fn new() -> Self {
        MeshData {
            vertices: Vec::new(),
            indices: Vec::new(),
            normals: Vec::new(),
            colors: Vec::new(),
        }
    }
}

impl Default for MeshData {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a mesh from an octree
pub fn generate_mesh(octree: &Octree) -> MeshData {
    let mut mesh = MeshData::new();

    // Collect all voxels from the octree
    let voxels = octree.collect_voxels();

    // For each voxel, generate a cube
    for (x, y, z, size, value) in voxels {
        if value == 0 {
            continue; // Skip empty voxels
        }

        // Convert value to color (simple grayscale for now)
        let color = value_to_color(value);

        add_cube(&mut mesh, x, y, z, size, color);
    }

    mesh
}

/// Convert a voxel value to RGB color
fn value_to_color(value: i32) -> [f32; 3] {
    // Simple color mapping - can be made more sophisticated
    if value < 0 {
        [1.0, 0.0, 0.0] // Red for negative values
    } else if value == 0 {
        [0.0, 0.0, 0.0] // Black for zero (shouldn't happen)
    } else {
        // Use value as hue for positive values
        let hue = (value % 360) as f32;
        hsv_to_rgb(hue, 0.8, 0.9)
    }
}

/// Convert HSV to RGB
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h % 360.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    [r + m, g + m, b + m]
}

/// Add a cube to the mesh
fn add_cube(mesh: &mut MeshData, x: f32, y: f32, z: f32, size: f32, color: [f32; 3]) {
    let base_index = (mesh.vertices.len() / 3) as u32;

    // Define 8 vertices of the cube
    let vertices = [
        [x, y, z],                      // 0: left-bottom-back
        [x + size, y, z],               // 1: right-bottom-back
        [x + size, y + size, z],        // 2: right-top-back
        [x, y + size, z],               // 3: left-top-back
        [x, y, z + size],               // 4: left-bottom-front
        [x + size, y, z + size],        // 5: right-bottom-front
        [x + size, y + size, z + size], // 6: right-top-front
        [x, y + size, z + size],        // 7: left-top-front
    ];

    // Add vertices
    for vertex in &vertices {
        mesh.vertices.extend_from_slice(vertex);
        mesh.colors.extend_from_slice(&color);
    }

    // Define faces with their normals
    // Each face is defined by 4 vertices (2 triangles)
    let faces = [
        // Back face (z = 0)
        ([0, 1, 2, 3], [0.0, 0.0, -1.0]),
        // Front face (z = size)
        ([5, 4, 7, 6], [0.0, 0.0, 1.0]),
        // Left face (x = 0)
        ([4, 0, 3, 7], [-1.0, 0.0, 0.0]),
        // Right face (x = size)
        ([1, 5, 6, 2], [1.0, 0.0, 0.0]),
        // Bottom face (y = 0)
        ([4, 5, 1, 0], [0.0, -1.0, 0.0]),
        // Top face (y = size)
        ([3, 2, 6, 7], [0.0, 1.0, 0.0]),
    ];

    for (indices, normal) in &faces {
        // First triangle
        mesh.indices.push(base_index + indices[0]);
        mesh.indices.push(base_index + indices[1]);
        mesh.indices.push(base_index + indices[2]);

        // Second triangle
        mesh.indices.push(base_index + indices[0]);
        mesh.indices.push(base_index + indices[2]);
        mesh.indices.push(base_index + indices[3]);

        // Add normals for all 4 vertices of this face (flipped to point outward)
        let flipped_normal = [-normal[0], -normal[1], -normal[2]];
        for _ in 0..4 {
            mesh.normals.extend_from_slice(&flipped_normal);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::octree::{Cube, Octree};

    #[test]
    fn test_generate_mesh_simple() {
        let tree = Octree::new(Cube::Solid(42));
        let mesh = generate_mesh(&tree);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.vertices.len() / 3, mesh.colors.len() / 3);
    }

    #[test]
    fn test_hsv_to_rgb() {
        let red = hsv_to_rgb(0.0, 1.0, 1.0);
        assert_eq!(red, [1.0, 0.0, 0.0]);

        let green = hsv_to_rgb(120.0, 1.0, 1.0);
        assert!((green[1] - 1.0).abs() < 0.01);
    }
}
