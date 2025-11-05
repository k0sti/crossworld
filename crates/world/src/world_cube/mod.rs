mod builder;

use crate::GeometryData;
use crossworld_cube::{
    ColorMapper, Cube, CubeCoord, DefaultMeshBuilder, Octree, glam::IVec3, serialize_csm,
};
use noise::{Fbm, Perlin};

pub struct WorldCube {
    octree: Octree,
    macro_depth: u32,  // World size = 2^macro_depth, terrain generation depth
    render_depth: u32, // Maximum traversal depth for mesh generation
    border_depth: u32, // Number of border cube layers
    material_colors: Option<Vec<[f32; 3]>>, // Material colors loaded from materials.json
}

impl WorldCube {
    /// Create new WorldCube with specified macro depth
    ///
    /// # Arguments
    /// * `macro_depth` - World size depth (e.g., 3 = 8×8×8 world units)
    /// * `micro_depth` - Additional subdivision levels for user edits (0-3)
    /// * `border_depth` - Number of border cube layers (0 = no border, 1+ = wrap in border octants)
    ///
    /// Architecture (macro_depth=3, micro_depth=3):
    /// - World size: 8×8×8 units (2^3)
    /// - Terrain generated at macro depth
    /// - User voxels can be placed up to macro+micro depth (depth 6)
    /// - Octree dynamically subdivides for finer edits
    ///
    /// Border layers:
    /// - Each border layer wraps the world in an octa (8 cubes)
    /// - 4 bottom cubes + 4 top cubes surround the world
    /// - Original world placed at octant 0 (bottom-front-left)
    pub fn new(macro_depth: u32, micro_depth: u32, border_depth: u32) -> Self {
        // Generate a random seed for unique world generation each time
        #[cfg(not(test))]
        let seed = {
            // Use JavaScript's Math.random() for WASM compatibility
            let random_value = js_sys::Math::random();
            (random_value * (u32::MAX as f64)) as u32
        };

        // Use fixed seed for tests for reproducibility
        #[cfg(test)]
        let seed = 12345u32;

        let noise = Perlin::new(seed);
        let fbm = Fbm::new(seed);

        // Build octree with terrain at macro depth
        // The octree automatically subdivides when voxels are placed at deeper levels
        let root = builder::build_ground_octree(&noise, &fbm, macro_depth);

        // Apply border layers if requested
        // Each border layer doubles the world size, so we need to account for this
        let root_with_borders = if border_depth > 0 {
            Self::add_border_layers(root, border_depth)
        } else {
            root
        };

        // Render depth must be deep enough to capture all possible voxel placements
        // macro_depth for terrain + micro_depth for user edits + border_depth for border expansion
        // Each border layer adds 1 to the effective depth (2^1 = doubles the size)
        let render_depth = macro_depth + micro_depth + border_depth;

        Self {
            octree: Octree::new(root_with_borders),
            macro_depth,
            render_depth,
            border_depth,
            material_colors: None,
        }
    }

    /// Wrap a cube with border layers using 4 vertical levels
    ///
    /// Each layer divides depth into 4 levels (y=0,1,2,3):
    /// - Level 0 (bottom): hard_ground (16)
    /// - Level 1: water (17)
    /// - Level 2: air (0)
    /// - Level 3 (top): air (0)
    /// - Original world placed at depth level where it fits centered
    ///
    /// # Arguments
    /// * `world` - The original world cube to wrap
    /// * `layers` - Number of border layers to add (each layer doubles world size)
    ///
    /// # Example
    /// With 1 border layer, the world is subdivided vertically into 4 levels.
    /// At each XZ position, we have 4 vertical slices from bottom to top.
    fn add_border_layers(world: Cube<i32>, layers: u32) -> Cube<i32> {
        const HARD_GROUND: i32 = 16;
        const WATER: i32 = 17;
        const AIR: i32 = 0;

        let mut result = world;

        for _ in 0..layers {
            // Create border structure with 4 vertical divisions (depth 2)
            // First level: create 8 octants
            let level1 = Cube::tabulate_vector(|pos1| {
                // Second level: subdivide each octant
                Cube::tabulate_vector(|pos2| {
                    // Calculate absolute Y position at depth 2 (0-3 range)
                    let y_pos = pos1.y * 2 + pos2.y;

                    // Assign materials based on Y level
                    match y_pos {
                        0 => Cube::Solid(HARD_GROUND), // Bottom: hard ground
                        1 => Cube::Solid(WATER),        // Lower middle: water
                        2 => Cube::Solid(AIR),          // Upper middle: air
                        3 => Cube::Solid(AIR),          // Top: air
                        _ => Cube::Solid(AIR),
                    }
                })
            });

            // Place the world in the center (position 1,1,1 at depth 2)
            // This means the world occupies a 2x2x2 region in the middle of the 4x4x4 grid
            result = level1.update_depth(2, IVec3::new(1, 1, 1), 1, result);
        }

        result
    }

    /// Set a voxel at octree coordinates
    ///
    /// # Coordinate System
    /// - Coordinates are in octree space at the specified depth
    /// - TypeScript handles world→octree conversion via worldToCube()
    /// - At depth d: valid coordinates are [0, 2^d - 1] per axis
    /// - Example with macroDepth=3:
    ///   * depth=0: coords [0,0] (entire world = one voxel)
    ///   * depth=1: coords [0,1] (2×2×2 voxels)
    ///   * depth=2: coords [0,3] (4×4×4 voxels)
    ///   * depth=3: coords [0,7] (8×8×8 voxels, 1 voxel = 1 world unit)
    ///
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

    /// Get a reference to the octree root (for testing)
    #[cfg(test)]
    pub fn get_root(&self) -> &Cube<i32> {
        &self.octree.root
    }

    /// Set material colors from materials.json
    ///
    /// # Arguments
    /// * `colors` - Flat array of RGB colors [r1,g1,b1, r2,g2,b2, ...] for materials 0-127
    pub fn set_material_colors(&mut self, colors: Vec<f32>) {
        if colors.len() < 128 * 3 {
            tracing::warn!("Material colors array too short, expected at least 384 values (128 materials × 3 RGB)");
            return;
        }

        let mut material_colors = Vec::with_capacity(128);
        for i in 0..128 {
            let idx = i * 3;
            material_colors.push([
                colors[idx],
                colors[idx + 1],
                colors[idx + 2],
            ]);
        }
        self.material_colors = Some(material_colors);
    }

    pub fn generate_mesh(&self) -> GeometryData {
        // Generate mesh from octree using appropriate mesh builder
        let color_mapper = MaterialColorMapper::new(self.material_colors.as_ref());
        let mut builder = DefaultMeshBuilder::new();

        // Border materials for world at each Y layer [y0, y1, y2, y3]
        // y=0,1 (bottom): bedrock/stone (32)
        // y=2,3 (top): air (0)
        let border_materials = [32, 32, 0, 0];

        // Use render_depth for traversal to find all voxels (terrain + subdivisions)
        // The mesh generator will automatically calculate correct voxel sizes
        // for each depth level, ensuring subdivided voxels render at correct positions
        // Use face-based mesh generation with neighbor culling
        crossworld_cube::generate_face_mesh(
            &self.octree.root,
            &mut builder,
            |index| color_mapper.map(index),
            self.render_depth,
            border_materials,
        );

        // Scale and offset vertices to match world coordinates
        // The mesh generator outputs vertices in [0,1] space where:
        // - Terrain voxels (at macro_depth) are correctly normalized
        // - Subdivided voxels (at macro+micro_depth) are correctly normalized
        // - Border layers expand the world by 2^border_depth
        // We scale by world_size (2^(macro_depth + border_depth)) to convert to world units
        let world_size = (1 << (self.macro_depth + self.border_depth)) as f32;
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
        Self::new(4, 0, 4) // Default: macro depth 4, micro depth 0, border depth 4
    }
}

/// Color mapper for cube ground that uses proper material colors
struct MaterialColorMapper {
    // Dawnbringer 32 palette RGB values (for indices 32-63)
    palette: [[f32; 3]; 32],
    // Material colors from materials.json (indices 0-127)
    materials: [[f32; 3]; 128],
}

impl MaterialColorMapper {
    fn new(material_colors: Option<&Vec<[f32; 3]>>) -> Self {
        let materials = if let Some(colors) = material_colors {
            // Use colors from materials.json passed from JavaScript
            let mut arr = [[0.0f32; 3]; 128];
            for (i, color) in colors.iter().take(128).enumerate() {
                arr[i] = *color;
            }
            arr
        } else {
            // Fallback to default colors if not set
            let mut materials = [[0.0f32; 3]; 128];

            // Initialize materials array with colors from materials.json
            // These match the exact colors defined in assets/materials.json
            materials[0] = [0.0, 0.0, 0.0];           // 0: empty
            materials[1] = [0.0, 0.0, 0.0];           // 1: set_empty
            materials[2] = [1.0, 1.0, 1.0];           // 2: glass
            materials[3] = [0.816, 1.0, 1.0];         // 3: ice
            materials[4] = [0.0, 0.498, 1.0];         // 4: water_surface
            materials[5] = [0.0, 1.0, 0.0];           // 5: slime
            materials[6] = [1.0, 0.647, 0.0];         // 6: honey
            materials[7] = [1.0, 0.0, 1.0];           // 7: crystal
            materials[8] = [0.0, 1.0, 1.0];           // 8: force_field
            materials[9] = [0.667, 0.0, 1.0];         // 9: portal
            materials[10] = [0.8, 0.8, 0.8];          // 10: mist
            materials[11] = [1.0, 0.0, 0.0];          // 11: stained_glass_red
            materials[12] = [0.0, 1.0, 0.0];          // 12: stained_glass_green
            materials[13] = [0.0, 0.0, 1.0];          // 13: stained_glass_blue
            materials[14] = [1.0, 1.0, 0.0];          // 14: stained_glass_yellow
            materials[15] = [0.502, 0.502, 0.502];    // 15: transparent_15
            materials[16] = [0.4, 0.267, 0.2];        // 16: hard_ground
            materials[17] = [0.0, 0.314, 0.624];      // 17: water
            materials[18] = [0.545, 0.271, 0.075];    // 18: dirt
            materials[19] = [0.227, 0.490, 0.227];    // 19: grass
            materials[20] = [0.502, 0.502, 0.502];    // 20: stone
            materials[21] = [0.431, 0.431, 0.431];    // 21: cobblestone
            materials[22] = [0.929, 0.788, 0.686];    // 22: sand
            materials[23] = [0.788, 0.655, 0.439];    // 23: sandstone
            materials[24] = [0.533, 0.533, 0.533];    // 24: gravel
            materials[25] = [0.627, 0.627, 0.627];    // 25: clay
            materials[26] = [1.0, 1.0, 1.0];          // 26: snow
            materials[27] = [0.690, 0.878, 1.0];      // 27: ice_solid
            materials[28] = [0.102, 0.059, 0.180];    // 28: obsidian
            materials[29] = [0.545, 0.0, 0.0];        // 29: netherrack
            materials[30] = [0.612, 0.365, 0.239];    // 30: granite
            materials[31] = [0.749, 0.749, 0.749];    // 31: diorite
            materials[32] = [0.427, 0.427, 0.427];    // 32: andesite
            materials[33] = [0.910, 0.910, 0.910];    // 33: marble
            materials[34] = [0.855, 0.816, 0.753];    // 34: limestone
            materials[35] = [0.169, 0.169, 0.169];    // 35: basalt
            materials[36] = [0.627, 0.510, 0.427];    // 36: wood_oak
            materials[37] = [0.420, 0.333, 0.208];    // 37: wood_spruce
            materials[38] = [0.843, 0.796, 0.553];    // 38: wood_birch
            materials[39] = [0.545, 0.435, 0.278];    // 39: wood_jungle
            materials[40] = [0.722, 0.408, 0.243];    // 40: wood_acacia
            materials[41] = [0.290, 0.220, 0.161];    // 41: wood_dark_oak
            materials[42] = [0.769, 0.651, 0.447];    // 42: planks_oak
            materials[43] = [0.486, 0.365, 0.243];    // 43: planks_spruce
            materials[44] = [0.890, 0.851, 0.659];    // 44: planks_birch
            materials[45] = [0.176, 0.314, 0.086];    // 45: leaves
            materials[46] = [0.365, 0.561, 0.227];    // 46: leaves_birch
            materials[47] = [0.239, 0.376, 0.188];    // 47: leaves_spruce
            materials[48] = [0.102, 0.102, 0.102];    // 48: coal
            materials[49] = [0.847, 0.847, 0.847];    // 49: iron
            materials[50] = [1.0, 0.843, 0.0];        // 50: gold
            materials[51] = [0.722, 0.451, 0.2];      // 51: copper
            materials[52] = [0.753, 0.753, 0.753];    // 52: silver
            materials[53] = [0.804, 0.498, 0.196];    // 53: bronze
            materials[54] = [0.565, 0.565, 0.627];    // 54: steel
            materials[55] = [0.529, 0.525, 0.506];    // 55: titanium
            materials[56] = [0.545, 0.227, 0.227];    // 56: brick
            materials[57] = [0.620, 0.620, 0.620];    // 57: concrete
            materials
        };

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
            materials,
        }
    }
}

impl ColorMapper for MaterialColorMapper {
    fn map(&self, index: i32) -> [f32; 3] {
        if index <= 0 {
            // 0 or negative: transparent/black
            return [0.0, 0.0, 0.0];
        }

        if (0..128).contains(&index) {
            // Materials 0-127: Use actual colors from materials.json
            return self.materials[index as usize];
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
                1 => 0.286, // 0x49/255
                2 => 0.573, // 0x92/255
                3 => 0.859, // 0xDB/255
                _ => 0.0,
            };
            let g = match g_bits {
                0 => 0.0,
                1 => 0.141, // 0x24/255
                2 => 0.286, // 0x49/255
                3 => 0.427, // 0x6D/255
                4 => 0.573, // 0x92/255
                5 => 0.714, // 0xB6/255
                6 => 0.859, // 0xDB/255
                7 => 1.0,   // 0xFF/255
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
