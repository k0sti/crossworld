mod builder;

use crate::GeometryData;
use crossworld_cube::{glam::IVec3, ColorMapper, Cube, Octree};
use noise::{Fbm, Perlin};

pub struct CubeGround {
    octree: Octree,
    depth: u32,
}

impl CubeGround {
    /// Create new CubeGround with specified depth
    ///
    /// # Arguments
    /// * `depth` - Octree subdivision depth (e.g., 5 = 32^3 voxels)
    ///
    /// Architecture:
    /// - Octree depth: 5 (32^3 voxels in octree space)
    /// - World scale depth: 1 (each octree unit = 2^1 = 2 world units)
    /// - World size: 32 * 2 = 64 units
    /// - At max depth, 1 octree voxel = 2 world units
    pub fn new(depth: u32) -> Self {
        // Build octree with specified depth
        // depth=5 creates 32x32x32 cube (2^5 = 32 octree voxels)
        // World size is 64 units (32 * 2^WORLD_SCALE_DEPTH)
        // y >= 0: surface checkerboard pattern
        // y < 0: underground terrain generated with noise/waves

        let noise = Perlin::new(12345);
        let fbm = Fbm::new(12345);

        let root = builder::build_ground_octree(&noise, &fbm, depth);

        Self {
            octree: Octree::new(root),
            depth,
        }
    }

    /// Set a voxel at octree coordinates at max-depth level
    /// Coordinates: x,y,z all in [0, 2^max_depth) octree space
    /// World space is centered: [-32, 32] in all axes (64 world units for depth 5, scale 1)
    /// depth: octree depth level (5=finest, 4=coarse, etc.)
    /// color_index: 0 = empty, 1+ = colored voxel
    pub fn set_voxel_at_depth(&mut self, x: i32, y: i32, z: i32, depth: u32, color_index: i32) {
        tracing::info!(
            "[Rust set_voxel_at_depth] input: x={}, y={}, z={}, depth={}, color={}, max_depth={}",
            x, y, z, depth, color_index, self.depth
        );

        // Clamp depth to valid range
        let depth = depth.clamp(0, self.depth);

        let grid_size = 1 << self.depth; // 2^self.depth (e.g., 128 for depth 7)

        // Coordinates are already in max-depth octree space (0 to 2^max_depth)
        // Check bounds at max depth level
        if x < 0 || x >= grid_size || y < 0 || y >= grid_size || z < 0 || z >= grid_size {
            tracing::warn!(
                "[Rust set_voxel_at_depth] out of bounds: ({}, {}, {}) not in [0, {})",
                x, y, z, grid_size
            );
            return;
        }

        // Scale from max-depth coordinates to target depth coordinates
        let scale = 1 << (self.depth - depth);  // 2^(depth_max - depth_target)
        let pos_x = x / scale;
        let pos_y = y / scale;
        let pos_z = z / scale;

        let pos = IVec3::new(pos_x, pos_y, pos_z);

        tracing::info!(
            "[Rust set_voxel_at_depth] scaled position: ({}, {}, {}) at depth={}, scale={}",
            pos_x, pos_y, pos_z, depth, scale
        );

        // Update single octree node at the target depth
        self.octree.root = self.octree.root
            .updated(Cube::Solid(color_index), depth, pos)
            .simplified();
    }

    /// Set a single voxel at world coordinates (convenience method)
    pub fn set_voxel(&mut self, x: i32, y: i32, z: i32, color_index: i32) {
        self.set_voxel_at_depth(x, y, z, self.depth, color_index);
    }

    /// Remove a voxel at world coordinates at specified depth
    pub fn remove_voxel_at_depth(&mut self, x: i32, y: i32, z: i32, depth: u32) {
        self.set_voxel_at_depth(x, y, z, depth, 0);
    }

    /// Remove a voxel at world coordinates (convenience method)
    pub fn remove_voxel(&mut self, x: i32, y: i32, z: i32) {
        self.set_voxel(x, y, z, 0);
    }

    pub fn generate_mesh(&self) -> GeometryData {
        // Generate mesh from octree using the cube mesher with custom color mapper
        // Use depth-aware version for efficient visitor-based traversal
        let color_mapper = DawnbringerColorMapper::new();
        let mesh_data = crossworld_cube::generate_mesh_with_mapper_depth(&self.octree, &color_mapper, self.depth);

        // Scale and offset vertices to match world coordinates
        // The octree generates normalized [0,1] coordinates
        // Architecture:
        // - Octree depth: 5 (32^3 voxels in octree space)
        // - World scale depth: 1 (each octree unit = 2^1 = 2 world units)
        // - World size: 32 * 2 = 64 units
        // - Centered coordinate system: [0,1] -> [-32, 32]
        const WORLD_SCALE_DEPTH: u32 = 1;
        let scale = 1 << WORLD_SCALE_DEPTH; // 2
        let octree_size = (1 << self.depth) as f32; // 32 for depth 5
        let world_size = octree_size * scale as f32; // 64 for depth 5, scale 1
        let half_world = world_size / 2.0; // 32

        let scaled_vertices: Vec<f32> = mesh_data
            .vertices
            .chunks(3)
            .flat_map(|chunk| {
                let x = chunk[0] * world_size - half_world; // [0,1] -> [-32, 32]
                let y = chunk[1] * world_size - half_world; // [0,1] -> [-32, 32]
                let z = chunk[2] * world_size - half_world; // [0,1] -> [-32, 32]
                vec![x, y, z]
            })
            .collect();

        GeometryData::new(
            scaled_vertices,
            mesh_data.indices,
            mesh_data.normals,
            mesh_data.colors,
        )
    }
}

impl Default for CubeGround {
    fn default() -> Self {
        Self::new(5) // Default: depth 5 (32^3 voxels, 64×64×64 world with scale=1)
    }
}

/// Color mapper for cube ground that uses Dawnbringer palette for user-placed voxels
struct DawnbringerColorMapper {
    // Dawnbringer 32 palette RGB values
    palette: [[f32; 3]; 32],
}

impl DawnbringerColorMapper {
    fn new() -> Self {
        Self {
            palette: [
                [0.0, 0.0, 0.0],       // 0: #000000
                [0.133, 0.125, 0.204], // 1: #222034
                [0.271, 0.157, 0.235], // 2: #45283c
                [0.4, 0.224, 0.192],   // 3: #663931
                [0.561, 0.337, 0.231], // 4: #8f563b
                [0.875, 0.443, 0.149], // 5: #df7126
                [0.851, 0.627, 0.4],   // 6: #d9a066
                [0.933, 0.765, 0.604], // 7: #eec39a
                [0.984, 0.949, 0.212], // 8: #fbf236
                [0.6, 0.898, 0.314],   // 9: #99e550
                [0.416, 0.745, 0.188], // 10: #6abe30
                [0.216, 0.58, 0.431],  // 11: #37946e
                [0.294, 0.412, 0.184], // 12: #4b692f
                [0.322, 0.294, 0.141], // 13: #524b24
                [0.196, 0.235, 0.224], // 14: #323c39
                [0.247, 0.247, 0.455], // 15: #3f3f74
                [0.188, 0.376, 0.51],  // 16: #306082
                [0.357, 0.431, 0.882], // 17: #5b6ee1
                [0.388, 0.608, 1.0],   // 18: #639bff
                [0.373, 0.804, 0.894], // 19: #5fcde4
                [0.796, 0.859, 0.988], // 20: #cbdbfc
                [1.0, 1.0, 1.0],       // 21: #ffffff
                [0.608, 0.678, 0.718], // 22: #9badb7
                [0.518, 0.494, 0.529], // 23: #847e87
                [0.412, 0.416, 0.416], // 24: #696a6a
                [0.349, 0.337, 0.322], // 25: #595652
                [0.463, 0.259, 0.541], // 26: #76428a
                [0.675, 0.196, 0.196], // 27: #ac3232
                [0.851, 0.341, 0.388], // 28: #d95763
                [0.843, 0.482, 0.729], // 29: #d77bba
                [0.561, 0.596, 0.29],  // 30: #8f974a
                [0.541, 0.435, 0.188], // 31: #8a6f30
            ],
        }
    }
}

impl ColorMapper for DawnbringerColorMapper {
    fn map(&self, index: i32) -> [f32; 3] {
        if index <= 0 {
            // 0 or negative: transparent/black
            return [0.0, 0.0, 0.0];
        }

        if (32..=63).contains(&index) {
            // Values 32-63 map to Dawnbringer palette indices 0-31
            let palette_idx = (index - 32) as usize;
            return self.palette[palette_idx];
        }

        // Values 1-31: terrain colors (checkerboard, underground)
        // Use simple HSV-based colors for terrain
        let hue = ((index * 37) % 360) as f32;
        hsv_to_rgb(hue, 0.6, 0.7)
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
