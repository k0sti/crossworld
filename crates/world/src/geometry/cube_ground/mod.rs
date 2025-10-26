mod builder;

use crate::GeometryData;
use crossworld_cube::{glam::IVec3, ColorMapper, Cube, Octree};
use noise::{Fbm, Perlin};

pub struct CubeGround {
    octree: Octree,
    depth: u32,
}

impl CubeGround {
    pub fn new(depth: u32) -> Self {
        // Build octree with specified depth
        // depth=4 creates 16x16x16 cube (2^4 = 16)
        // This covers y=-8 to y=8 (after scaling)
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

    /// Convert world coordinates to voxel grid coordinates
    /// World coords: x=0-15, y=-8-7, z=0-15
    /// Voxel coords: x=0-15, y=0-15, z=0-15 (for depth=4)
    fn world_to_voxel(&self, x: i32, y: i32, z: i32) -> IVec3 {
        IVec3::new(
            x,          // world 0-15 -> voxel 0-15
            y + 8,      // world -8-7 -> voxel 0-15
            z,          // world 0-15 -> voxel 0-15
        )
    }

    /// Set a voxel at world coordinates (x, y, z)
    /// World coords: x=0-15, y=-8-7, z=0-15
    /// color_index: 0 = empty, 1+ = colored voxel
    pub fn set_voxel(&mut self, x: i32, y: i32, z: i32, color_index: i32) {
        // Clamp to valid world range
        if !(0..16).contains(&x) || !(-8..8).contains(&y) || !(0..16).contains(&z) {
            return;
        }

        let voxel_pos = self.world_to_voxel(x, y, z);

        // Use the new functional update interface - O(log n) instead of O(n)
        self.octree.root = self.octree.root
            .updated(Cube::Solid(color_index), self.depth, voxel_pos)
            .simplified();
    }

    /// Set a cube of voxels at world coordinates
    /// size: number of voxels in each dimension (1, 2, 4, 8, 16)
    /// The cube is placed with (x,y,z) as the corner with minimum coordinates
    pub fn set_voxel_cube(&mut self, x: i32, y: i32, z: i32, size: i32, color_index: i32) {
        // For size=1, just set a single voxel
        if size <= 1 {
            self.set_voxel(x, y, z, color_index);
            return;
        }

        // For larger sizes, fill a cube of voxels
        for dx in 0..size {
            for dy in 0..size {
                for dz in 0..size {
                    let vx = x + dx;
                    let vy = y + dy;
                    let vz = z + dz;

                    // Check bounds
                    if (0..16).contains(&vx) && (-8..8).contains(&vy) && (0..16).contains(&vz) {
                        self.set_voxel(vx, vy, vz, color_index);
                    }
                }
            }
        }
    }

    /// Remove a voxel at world coordinates
    pub fn remove_voxel(&mut self, x: i32, y: i32, z: i32) {
        // Removing is just setting to 0 (empty)
        self.set_voxel(x, y, z, 0);
    }

    /// Remove a cube of voxels at world coordinates
    pub fn remove_voxel_cube(&mut self, x: i32, y: i32, z: i32, size: i32) {
        self.set_voxel_cube(x, y, z, size, 0);
    }

    pub fn generate_mesh(&self) -> GeometryData {
        // Generate mesh from octree using the cube mesher with custom color mapper
        // Use depth-aware version for efficient visitor-based traversal
        let color_mapper = DawnbringerColorMapper::new();
        let mesh_data = crossworld_cube::generate_mesh_with_mapper_depth(&self.octree, &color_mapper, self.depth);

        // Scale and offset vertices to match world coordinates
        // The octree generates a 16x16x16 voxel grid within a 1.0 unit cube
        // We need:
        // - x: 0-16 world units (scale by 1.0)
        // - y: -8 to 8 world units (scale by 1.0, offset by -8)
        // - z: 0-16 world units (scale by 1.0)
        let scaled_vertices: Vec<f32> = mesh_data
            .vertices
            .chunks(3)
            .flat_map(|chunk| {
                let x = chunk[0] * 16.0; // 1.0 * 16 = 16 units
                let y = chunk[1] * 16.0 - 8.0; // 16 units, offset by -8
                let z = chunk[2] * 16.0; // 1.0 * 16 = 16 units
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
        Self::new(4) // Default depth of 4 (16x16x16 grid)
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
