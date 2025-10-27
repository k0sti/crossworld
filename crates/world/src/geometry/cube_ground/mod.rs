mod builder;

use crate::GeometryData;
use crossworld_cube::{ColorMapper, Cube, DefaultMeshBuilder, Octree, glam::IVec3};
use noise::{Fbm, Perlin};

pub struct CubeGround {
    octree: Octree,
    macro_depth: u32,  // World size = 2^macro_depth, terrain generation depth
    face_mesh_mode: bool,
}

impl CubeGround {
    /// Create new CubeGround with specified macro depth
    ///
    /// # Arguments
    /// * `macro_depth` - World size depth (e.g., 3 = 8×8×8 world units)
    ///
    /// Architecture (macro_depth=3):
    /// - World size: 8×8×8 units (2^3)
    /// - Terrain generated at macro depth
    /// - Octree dynamically subdivides for finer edits
    pub fn new(macro_depth: u32, _micro_depth: u32) -> Self {
        let noise = Perlin::new(12345);
        let fbm = Fbm::new(12345);

        // Build octree with terrain at macro depth
        // micro_depth is only used by TypeScript for coordinate scaling
        // The octree automatically subdivides when voxels are placed at deeper levels
        let root = builder::build_ground_octree(&noise, &fbm, macro_depth);

        Self {
            octree: Octree::new(root),
            macro_depth,
            face_mesh_mode: true,
        }
    }

    /// Set a voxel at octree coordinates
    /// Coordinates and depth are in octree space (TypeScript handles scaling)
    /// The octree will automatically subdivide to support the requested depth
    pub fn set_voxel_at_depth(&mut self, x: i32, y: i32, z: i32, depth: u32, color_index: i32) {
        tracing::info!(
            "[Rust set_voxel_at_depth] x={}, y={}, z={}, depth={}, color={}, macro_depth={}",
            x,
            y,
            z,
            depth,
            color_index,
            self.macro_depth
        );

        // Calculate position at the target depth
        let pos = IVec3::new(x, y, z);

        tracing::info!(
            "[Rust set_voxel_at_depth] setting at position: {:?} at depth={}",
            pos,
            depth
        );

        // Update octree node at the target depth
        // The octree will dynamically subdivide as needed
        self.octree.root = self
            .octree
            .root
            .updated(Cube::Solid(color_index), depth, pos)
            .simplified();
    }

    /// Set a single voxel at world coordinates (convenience method)
    pub fn set_voxel(&mut self, x: i32, y: i32, z: i32, color_index: i32) {
        self.set_voxel_at_depth(x, y, z, self.macro_depth, color_index);
    }

    /// Remove a voxel at world coordinates at specified depth
    pub fn remove_voxel_at_depth(&mut self, x: i32, y: i32, z: i32, depth: u32) {
        self.set_voxel_at_depth(x, y, z, depth, 0);
    }

    /// Remove a voxel at world coordinates (convenience method)
    pub fn remove_voxel(&mut self, x: i32, y: i32, z: i32) {
        self.set_voxel(x, y, z, 0);
    }

    /// Set face mesh mode (neighbor-aware culling)
    pub fn set_face_mesh_mode(&mut self, enabled: bool) {
        tracing::info!("[CubeGround] Setting face mesh mode: {}", enabled);
        self.face_mesh_mode = enabled;
    }

    /// Set ground render mode (currently unused, placeholder for future)
    pub fn set_ground_render_mode(&mut self, _use_cube: bool) {
        tracing::info!("[CubeGround] Ground render mode not yet implemented");
    }

    pub fn generate_mesh(&self) -> GeometryData {
        // Generate mesh from octree using appropriate mesh builder
        let color_mapper = DawnbringerColorMapper::new();
        let mut builder = DefaultMeshBuilder::new();

        // ALWAYS use macro_depth for mesh generation
        // This ensures the world scale stays constant
        // Subdivided voxels will be rendered as smaller faces automatically
        // by the octree structure itself
        if self.face_mesh_mode {
            // Use face-based mesh generation with neighbor culling
            crossworld_cube::generate_face_mesh(
                &self.octree.root,
                &mut builder,
                |index| color_mapper.map(index),
                self.macro_depth,
            );
        } else {
            // Use hierarchical mesh generation (all faces)
            crossworld_cube::generate_mesh_hierarchical(
                &self.octree,
                &mut builder,
                |index| color_mapper.map(index),
                self.macro_depth,
            );
        }

        // Scale and offset vertices to match world coordinates
        // The mesh generator outputs vertices in [0,1] normalized to macro_depth
        // This keeps world scale constant regardless of micro_depth
        let world_size = (1 << self.macro_depth) as f32;
        let half_world = world_size / 2.0;

        let scaled_vertices: Vec<f32> = builder
            .vertices
            .chunks(3)
            .flat_map(|chunk| {
                let x = chunk[0] * world_size - half_world;
                let y = chunk[1] * world_size - half_world;
                let z = chunk[2] * world_size - half_world;
                vec![x, y, z]
            })
            .collect();

        GeometryData::new(
            scaled_vertices,
            builder.indices,
            builder.normals,
            builder.colors,
        )
    }
}

impl Default for CubeGround {
    fn default() -> Self {
        Self::new(3, 0) // Default: macro depth 3, micro depth 0
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
