use crate::octree::{Cube, IVec3Ext, Octree};
use glam::IVec3;

/// Face direction for cube faces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Face {
    Top,    // +Y
    Bottom, // -Y
    Left,   // -X
    Right,  // +X
    Front,  // +Z
    Back,   // -Z
}

impl Face {
    /// Get the normal vector for this face
    fn normal(self) -> [f32; 3] {
        match self {
            Face::Top => [0.0, 1.0, 0.0],
            Face::Bottom => [0.0, -1.0, 0.0],
            Face::Left => [-1.0, 0.0, 0.0],
            Face::Right => [1.0, 0.0, 0.0],
            Face::Front => [0.0, 0.0, 1.0],
            Face::Back => [0.0, 0.0, -1.0],
        }
    }

    /// Get the four vertices for this face in counter-clockwise order when viewed from outside
    fn vertices(self, x: f32, y: f32, z: f32, size: f32) -> [[f32; 3]; 4] {
        match self {
            Face::Top => [
                [x, y + size, z],
                [x + size, y + size, z],
                [x + size, y + size, z + size],
                [x, y + size, z + size],
            ],
            Face::Bottom => [
                [x, y, z + size],
                [x + size, y, z + size],
                [x + size, y, z],
                [x, y, z],
            ],
            Face::Left => [
                [x, y, z],
                [x, y, z + size],
                [x, y + size, z + size],
                [x, y + size, z],
            ],
            Face::Right => [
                [x + size, y, z + size],
                [x + size, y, z],
                [x + size, y + size, z],
                [x + size, y + size, z + size],
            ],
            Face::Front => [
                [x, y, z + size],
                [x + size, y, z + size],
                [x + size, y + size, z + size],
                [x, y + size, z + size],
            ],
            Face::Back => [
                [x + size, y, z],
                [x, y, z],
                [x, y + size, z],
                [x + size, y + size, z],
            ],
        }
    }
}

/// Builder interface for constructing meshes
pub trait MeshBuilder {
    /// Add a single face to the mesh
    ///
    /// # Arguments
    /// * `vertices` - Four vertices forming a quad (counter-clockwise)
    /// * `normal` - Normal vector for the face
    /// * `color` - RGB color for the face
    fn add_face(&mut self, vertices: [[f32; 3]; 4], normal: [f32; 3], color: [f32; 3]);
}

/// Default mesh builder that accumulates data into vectors
pub struct DefaultMeshBuilder {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
    pub normals: Vec<f32>,
    pub colors: Vec<f32>,
    vertex_count: u32,
}

impl DefaultMeshBuilder {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            normals: Vec::new(),
            colors: Vec::new(),
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

/// Generate mesh from octree using hierarchical recursive traversal
///
/// # Arguments
/// * `octree` - The octree to generate mesh from
/// * `builder` - MeshBuilder to receive the faces
/// * `color_fn` - Function to map voxel values to colors
/// * `max_depth` - Maximum depth of the octree
pub fn generate_mesh_hierarchical<B, F>(
    octree: &Octree,
    builder: &mut B,
    color_fn: F,
    max_depth: u32,
) where
    B: MeshBuilder,
    F: Fn(i32) -> [f32; 3],
{
    // Start recursive traversal from root
    traverse_cube(
        &octree.root,
        builder,
        &color_fn,
        max_depth,
        0,
        IVec3::ZERO,
        max_depth,
    );
}

/// Recursively traverse the cube hierarchy and generate faces
///
/// # Arguments
/// * `cube` - Current cube node
/// * `builder` - MeshBuilder to receive the faces
/// * `color_fn` - Function to map voxel values to colors
/// * `max_depth` - Maximum depth of the octree
/// * `current_depth` - Current depth in the tree (0 = root)
/// * `pos` - Position in grid coordinates
/// * `remaining_depth` - Remaining depth to traverse
fn traverse_cube<B, F>(
    cube: &Cube<i32>,
    builder: &mut B,
    color_fn: &F,
    max_depth: u32,
    current_depth: u32,
    pos: IVec3,
    remaining_depth: u32,
) where
    B: MeshBuilder,
    F: Fn(i32) -> [f32; 3],
{
    match cube {
        Cube::Solid(value) => {
            // Skip empty voxels
            if *value == 0 {
                return;
            }

            // Calculate size and position in normalized [0,1] space
            let grid_size = 1 << max_depth; // 2^max_depth
            let voxel_size = 1.0 / grid_size as f32;

            // Scale factor based on remaining depth
            let scale_factor = 1 << remaining_depth;

            // Calculate world position
            let x = (pos.x * scale_factor) as f32 * voxel_size;
            let y = (pos.y * scale_factor) as f32 * voxel_size;
            let z = (pos.z * scale_factor) as f32 * voxel_size;
            let size = voxel_size * scale_factor as f32;

            // Get color for this voxel
            let color = color_fn(*value);

            // Add all six faces
            // TODO: Implement face culling to skip internal faces
            add_cube_faces(builder, x, y, z, size, color);
        }
        Cube::Cubes(children) => {
            // Recurse into children if there's remaining depth
            if remaining_depth > 0 {
                let next_depth = remaining_depth - 1;
                for i in 0..8 {
                    let child_pos = (pos << 1) + IVec3::from_octant_index(i);
                    traverse_cube(
                        &children[i],
                        builder,
                        color_fn,
                        max_depth,
                        current_depth + 1,
                        child_pos,
                        next_depth,
                    );
                }
            } else {
                // At maximum depth, render as a single cube
                let grid_size = 1 << max_depth;
                let voxel_size = 1.0 / grid_size as f32;

                let x = pos.x as f32 * voxel_size;
                let y = pos.y as f32 * voxel_size;
                let z = pos.z as f32 * voxel_size;

                // Use the cube's ID as the value
                let value = cube.id();
                if value != 0 {
                    let color = color_fn(value);
                    add_cube_faces(builder, x, y, z, voxel_size, color);
                }
            }
        }
        Cube::Planes { axis: _, quad: _ } | Cube::Slices { axis: _, layers: _ } => {
            // For Planes and Slices, treat as solid with the ID value
            let value = cube.id();
            if value == 0 {
                return;
            }

            let grid_size = 1 << max_depth;
            let voxel_size = 1.0 / grid_size as f32;
            let scale_factor = 1 << remaining_depth;

            let x = (pos.x * scale_factor) as f32 * voxel_size;
            let y = (pos.y * scale_factor) as f32 * voxel_size;
            let z = (pos.z * scale_factor) as f32 * voxel_size;
            let size = voxel_size * scale_factor as f32;

            let color = color_fn(value);
            add_cube_faces(builder, x, y, z, size, color);
        }
    }
}

/// Add all six faces of a cube to the mesh builder
fn add_cube_faces<B: MeshBuilder>(
    builder: &mut B,
    x: f32,
    y: f32,
    z: f32,
    size: f32,
    color: [f32; 3],
) {
    let faces = [
        Face::Top,
        Face::Bottom,
        Face::Left,
        Face::Right,
        Face::Front,
        Face::Back,
    ];

    for face in &faces {
        let vertices = face.vertices(x, y, z, size);
        let normal = face.normal();
        builder.add_face(vertices, normal, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::octree::{Cube, Octree};
    use std::rc::Rc;

    fn simple_color_mapper(value: i32) -> [f32; 3] {
        if value <= 0 {
            [0.0, 0.0, 0.0]
        } else {
            let hue = (value % 360) as f32;
            // Simple hue to RGB conversion
            let c = 1.0;
            let x = 1.0 - ((hue / 60.0) % 2.0 - 1.0).abs();
            if hue < 60.0 {
                [c, x, 0.0]
            } else if hue < 120.0 {
                [x, c, 0.0]
            } else if hue < 180.0 {
                [0.0, c, x]
            } else if hue < 240.0 {
                [0.0, x, c]
            } else if hue < 300.0 {
                [x, 0.0, c]
            } else {
                [c, 0.0, x]
            }
        }
    }

    #[test]
    fn test_face_normals() {
        assert_eq!(Face::Top.normal(), [0.0, 1.0, 0.0]);
        assert_eq!(Face::Bottom.normal(), [0.0, -1.0, 0.0]);
        assert_eq!(Face::Left.normal(), [-1.0, 0.0, 0.0]);
        assert_eq!(Face::Right.normal(), [1.0, 0.0, 0.0]);
        assert_eq!(Face::Front.normal(), [0.0, 0.0, 1.0]);
        assert_eq!(Face::Back.normal(), [0.0, 0.0, -1.0]);
    }

    #[test]
    fn test_generate_mesh_simple() {
        let tree = Octree::new(Cube::Solid(42));
        let mut builder = DefaultMeshBuilder::new();

        generate_mesh_hierarchical(&tree, &mut builder, simple_color_mapper, 0);

        // Should have vertices for a single cube (6 faces * 4 vertices = 24)
        assert_eq!(builder.vertices.len(), 24 * 3);
        assert_eq!(builder.normals.len(), 24 * 3);
        assert_eq!(builder.colors.len(), 24 * 3);
        // 6 faces * 2 triangles * 3 indices = 36
        assert_eq!(builder.indices.len(), 36);
    }

    #[test]
    fn test_generate_mesh_subdivided() {
        let cube = Cube::cubes([
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(2)),
            Rc::new(Cube::Solid(3)),
            Rc::new(Cube::Solid(4)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(6)),
            Rc::new(Cube::Solid(7)),
            Rc::new(Cube::Solid(8)),
        ]);

        let tree = Octree::new(cube);
        let mut builder = DefaultMeshBuilder::new();

        generate_mesh_hierarchical(&tree, &mut builder, simple_color_mapper, 1);

        // Should have 8 cubes, each with 6 faces * 4 vertices = 192 vertices
        assert_eq!(builder.vertices.len(), 8 * 24 * 3);
    }

    #[test]
    fn test_skip_empty_voxels() {
        let cube = Cube::cubes([
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(2)),
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(3)),
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(4)),
        ]);

        let tree = Octree::new(cube);
        let mut builder = DefaultMeshBuilder::new();

        generate_mesh_hierarchical(&tree, &mut builder, simple_color_mapper, 1);

        // Should have only 4 cubes (non-zero values)
        assert_eq!(builder.vertices.len(), 4 * 24 * 3);
    }

    #[test]
    fn test_all_normals_unit_length() {
        let tree = Octree::new(Cube::Solid(1));
        let mut builder = DefaultMeshBuilder::new();

        generate_mesh_hierarchical(&tree, &mut builder, simple_color_mapper, 0);

        // Check that all normals are unit length
        for i in 0..builder.normals.len() / 3 {
            let nx = builder.normals[i * 3];
            let ny = builder.normals[i * 3 + 1];
            let nz = builder.normals[i * 3 + 2];
            let length = (nx * nx + ny * ny + nz * nz).sqrt();
            assert!((length - 1.0).abs() < 0.001, "Normal not unit length");
        }
    }

    #[test]
    fn test_normals_consistent_per_face() {
        let tree = Octree::new(Cube::Solid(1));
        let mut builder = DefaultMeshBuilder::new();

        generate_mesh_hierarchical(&tree, &mut builder, simple_color_mapper, 0);

        // Each face should have 4 vertices with identical normals
        for face_idx in 0..6 {
            let base = face_idx * 4;
            let n0 = [
                builder.normals[base * 3],
                builder.normals[base * 3 + 1],
                builder.normals[base * 3 + 2],
            ];

            for i in 1..4 {
                let ni = [
                    builder.normals[(base + i) * 3],
                    builder.normals[(base + i) * 3 + 1],
                    builder.normals[(base + i) * 3 + 2],
                ];
                assert_eq!(n0, ni, "Normals not consistent within face");
            }
        }
    }
}
