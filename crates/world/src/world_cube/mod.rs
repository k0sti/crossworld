mod builder;

use crate::GeometryData;
use crossworld_cube::{
    ColorMapper, Cube, CubeCoord, DefaultMeshBuilder, Octree, glam::IVec3, serialize_csm,
};
use noise::{Fbm, Perlin};

pub struct WorldCube {
    octree: Octree,
    macro_depth: u32,   // World size = 2^macro_depth, terrain generation depth
    render_depth: u32,  // Maximum traversal depth for mesh generation
    _border_depth: u32, // Number of border cube layers (not yet implemented)
}

impl WorldCube {
    /// Create new WorldCube with specified macro depth
    ///
    /// # Arguments
    /// * `macro_depth` - World size depth (e.g., 3 = 8×8×8 world units)
    /// * `micro_depth` - Additional subdivision levels for user edits (0-3)
    /// * `_border_depth` - Number of border cube layers (not yet implemented)
    ///
    /// Architecture (macro_depth=3, micro_depth=3):
    /// - World size: 8×8×8 units (2^3)
    /// - Terrain generated at macro depth
    /// - User voxels can be placed up to macro+micro depth (depth 6)
    /// - Octree dynamically subdivides for finer edits
    pub fn new(macro_depth: u32, micro_depth: u32, _border_depth: u32) -> Self {
        // Generate a random seed for unique world generation each time
        // Use JavaScript's Math.random() for WASM compatibility
        let random_value = js_sys::Math::random();
        let seed = (random_value * (u32::MAX as f64)) as u32;

        let noise = Perlin::new(seed);
        let fbm = Fbm::new(seed);

        // Build octree with terrain at macro depth
        // The octree automatically subdivides when voxels are placed at deeper levels
        let root = builder::build_ground_octree(&noise, &fbm, macro_depth);

        // Render depth must be deep enough to capture all possible voxel placements
        // macro_depth for terrain + micro_depth for user edits
        let render_depth = macro_depth + micro_depth;

        Self {
            octree: Octree::new(root),
            macro_depth,
            render_depth,
            _border_depth,
        }
    }

    /// Set a voxel at octree coordinates
    /// Coordinates and depth are in octree space (TypeScript handles scaling)
    /// The octree will automatically subdivide to support the requested depth
    pub fn set_voxel_at_depth(&mut self, x: i32, y: i32, z: i32, depth: u32, color_index: i32) {
        let pos = IVec3::new(x, y, z);
        self.octree.root = self
            .octree
            .root
            .update(CubeCoord::new(pos, depth), Cube::Solid(color_index))
            .simplified();
    }

    /// Remove a voxel at world coordinates at specified depth
    pub fn remove_voxel_at_depth(&mut self, x: i32, y: i32, z: i32, depth: u32) {
        self.set_voxel_at_depth(x, y, z, depth, 0);
    }

    /// Export the octree to CSM format
    pub fn export_to_csm(&self) -> String {
        serialize_csm(&self.octree)
    }

    pub fn generate_mesh(&self) -> GeometryData {
        // Generate mesh from octree using appropriate mesh builder
        let color_mapper = DawnbringerColorMapper::new();
        let mut builder = DefaultMeshBuilder::new();

        // Use render_depth for traversal to find all voxels (terrain + subdivisions)
        // The mesh generator will automatically calculate correct voxel sizes
        // for each depth level, ensuring subdivided voxels render at correct positions
        // Use face-based mesh generation with neighbor culling
        crossworld_cube::generate_face_mesh(
            &self.octree.root,
            &mut builder,
            |index| color_mapper.map(index),
            self.render_depth,
        );

        // Scale and offset vertices to match world coordinates
        // The mesh generator outputs vertices in [0,1] space where:
        // - Terrain voxels (at macro_depth) are correctly normalized
        // - Subdivided voxels (at macro+micro_depth) are correctly normalized
        // We scale by world_size (2^macro_depth) to convert to world units
        let world_size = (1 << self.macro_depth) as f32;
        let half_size = world_size / 2.0;

        let scaled_vertices: Vec<f32> = builder
            .vertices
            .chunks(3)
            .flat_map(|chunk| {
                let x = chunk[0] * world_size - half_size;
                let y = chunk[1] * world_size - half_size;
                let z = chunk[2] * world_size - half_size;
                vec![x, y, z]
            })
            .collect();

        GeometryData::new_with_uvs(
            scaled_vertices,
            builder.indices,
            builder.normals,
            builder.colors,
            builder.uvs,
            builder.material_ids,
        )
    }

    /// Get reference to the root cube
    #[allow(dead_code)]
    pub fn root(&self) -> &Cube<i32> {
        &self.octree.root
    }

    /// Set a new root cube
    ///
    /// Replaces the entire octree root. The cube will be simplified automatically.
    pub fn set_root(&mut self, cube: Cube<i32>) {
        self.octree.root = cube.simplified();
    }
}

impl Default for WorldCube {
    fn default() -> Self {
        Self::new(3, 0, 0) // Default: macro depth 3, micro depth 0, no borders
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

        if (2..=127).contains(&index) {
            // Materials 2-127: textured materials from materials.json
            // These get textures applied in rendering, but we provide fallback colors
            // Use HSV-based placeholder colors for when textures aren't loaded
            let hue = ((index * 23) % 360) as f32;
            return hsv_to_rgb(hue, 0.7, 0.8);
        }

        if (128..=255).contains(&index) {
            // Materials 128-255: solid colors (vox models, color palette)
            // Generate RGB from 7-bit encoding: r:2, g:3, b:2
            let bits = (index - 128) as u8;
            let r_bits = (bits >> 5) & 0b11;
            let g_bits = (bits >> 2) & 0b111;
            let b_bits = bits & 0b11;

            // Convert to RGB values
            let r = match r_bits {
                0 => 0.0,
                1 => 0.286,  // 0x49/255
                2 => 0.573,  // 0x92/255
                3 => 0.859,  // 0xDB/255
                _ => 0.0,
            };
            let g = match g_bits {
                0 => 0.0,
                1 => 0.141,  // 0x24/255
                2 => 0.286,  // 0x49/255
                3 => 0.427,  // 0x6D/255
                4 => 0.573,  // 0x92/255
                5 => 0.714,  // 0xB6/255
                6 => 0.859,  // 0xDB/255
                7 => 1.0,    // 0xFF/255
                _ => 0.0,
            };
            let b = match b_bits {
                0 => 0.0,
                1 => 0.286,
                2 => 0.573,
                3 => 0.859,
                _ => 0.0,
            };
            return [r, g, b];
        }

        // Values 1, 32-63: Legacy support for Dawnbringer palette
        if (32..=63).contains(&index) {
            let palette_idx = (index - 32) as usize;
            return self.palette[palette_idx];
        }

        // Value 1 or any other: terrain colors
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
