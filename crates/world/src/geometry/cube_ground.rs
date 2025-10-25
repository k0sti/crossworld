use crate::GeometryData;
use crossworld_cube::{ColorMapper, Cube, Octree};
use noise::{Fbm, NoiseFn, Perlin};
use std::collections::HashMap;
use std::rc::Rc;

pub struct CubeGround {
    octree: Octree,
    // Track voxel modifications (position -> value, 0 = empty)
    modifications: HashMap<(i32, i32, i32), i32>,
}

impl CubeGround {
    pub fn new() -> Self {
        // Build a 16x16x16 cube with 4 levels deep
        // This covers y=-8 to y=8 (after scaling)
        // y >= 0: surface checkerboard pattern
        // y < 0: underground terrain generated with noise/waves

        let noise = Perlin::new(12345);
        let fbm = Fbm::new(12345);

        let root = Self::build_ground_octree(&noise, &fbm);

        Self {
            octree: Octree::new(root),
            modifications: HashMap::new(),
        }
    }

    /// Set a voxel at world coordinates (x, y, z)
    /// World coords: x=0-7, y=-8-7, z=0-7
    /// color_index: 0 = empty, 1+ = colored voxel
    pub fn set_voxel(&mut self, x: i32, y: i32, z: i32, color_index: i32) {
        // Clamp to valid world range
        if !(0..8).contains(&x) || !(-8..8).contains(&y) || !(0..8).contains(&z) {
            return;
        }

        self.modifications.insert((x, y, z), color_index);
        self.rebuild_octree();
    }

    /// Remove a voxel at world coordinates
    pub fn remove_voxel(&mut self, x: i32, y: i32, z: i32) {
        if !(0..8).contains(&x) || !(-8..8).contains(&y) || !(0..8).contains(&z) {
            return;
        }

        self.modifications.insert((x, y, z), 0);
        self.rebuild_octree();
    }

    /// Rebuild octree with modifications applied
    fn rebuild_octree(&mut self) {
        let noise = Perlin::new(12345);
        let fbm = Fbm::new(12345);
        let root = Self::build_ground_octree_with_mods(&noise, &fbm, &self.modifications);
        self.octree = Octree::new(root);
    }

    fn build_ground_octree(noise: &Perlin, fbm: &Fbm<Perlin>) -> Cube<i32> {
        let mods = HashMap::new();
        Self::build_ground_octree_with_mods(noise, fbm, &mods)
    }

    fn build_ground_octree_with_mods(
        noise: &Perlin,
        fbm: &Fbm<Perlin>,
        modifications: &HashMap<(i32, i32, i32), i32>,
    ) -> Cube<i32> {
        // Build 8 children at level 1 (each represents 8x8x8 space)
        let level1_children: [Rc<Cube<i32>>; 8] =
            std::array::from_fn(|i| Rc::new(Self::build_level2(i, noise, fbm, modifications)));

        Cube::cubes(level1_children)
    }

    fn build_level2(
        parent_idx: usize,
        noise: &Perlin,
        fbm: &Fbm<Perlin>,
        modifications: &HashMap<(i32, i32, i32), i32>,
    ) -> Cube<i32> {
        // Build 8 children at level 2 (each represents 4x4x4 space)
        let level2_children: [Rc<Cube<i32>>; 8] = std::array::from_fn(|i| {
            Rc::new(Self::build_level3(parent_idx, i, noise, fbm, modifications))
        });

        Cube::cubes(level2_children)
    }

    fn build_level3(
        parent1_idx: usize,
        parent2_idx: usize,
        noise: &Perlin,
        fbm: &Fbm<Perlin>,
        modifications: &HashMap<(i32, i32, i32), i32>,
    ) -> Cube<i32> {
        // Build 8 children at level 3 (each represents 2x2x2 space)
        let level3_children: [Rc<Cube<i32>>; 8] = std::array::from_fn(|i| {
            Rc::new(Self::build_level4(
                parent1_idx,
                parent2_idx,
                i,
                noise,
                fbm,
                modifications,
            ))
        });

        Cube::cubes(level3_children)
    }

    fn build_level4(
        parent1_idx: usize,
        parent2_idx: usize,
        parent3_idx: usize,
        noise: &Perlin,
        fbm: &Fbm<Perlin>,
        modifications: &HashMap<(i32, i32, i32), i32>,
    ) -> Cube<i32> {
        // Build 8 children at level 4 (each represents 1x1x1 voxel)
        // Calculate position in 16x16x16 grid (0-15 range)
        let (p1x, p1y, p1z) = octant_offset(parent1_idx);
        let (p2x, p2y, p2z) = octant_offset(parent2_idx);
        let (p3x, p3y, p3z) = octant_offset(parent3_idx);

        // Position within 16x16x16 grid
        let base_x = ((p1x * 16.0) + (p2x * 8.0) + (p3x * 4.0)) as i32;
        let base_y = ((p1y * 16.0) + (p2y * 8.0) + (p3y * 4.0)) as i32;
        let base_z = ((p1z * 16.0) + (p2z * 8.0) + (p3z * 4.0)) as i32;

        let level4_children: [Rc<Cube<i32>>; 8] = std::array::from_fn(|octant_idx| {
            let (ox, oy, oz) = octant_offset(octant_idx);
            let voxel_x = base_x + (ox * 2.0) as i32;
            let voxel_y = base_y + (oy * 2.0) as i32;
            let voxel_z = base_z + (oz * 2.0) as i32;

            // Convert to world coordinates (offset y so 0-15 becomes -8 to 7)
            let world_y = voxel_y - 8;

            // Convert voxel grid coords to world coords for lookup
            let world_x = voxel_x / 2;
            let world_z = voxel_z / 2;

            // Check if there's a modification for this voxel
            let value =
                if let Some(&modified_value) = modifications.get(&(world_x, world_y, world_z)) {
                    modified_value
                } else {
                    Self::get_voxel_value(voxel_x, world_y, voxel_z, noise, fbm)
                };

            Rc::new(Cube::Solid(value))
        });

        Cube::cubes(level4_children)
    }

    fn get_voxel_value(x: i32, y: i32, z: i32, noise: &Perlin, fbm: &Fbm<Perlin>) -> i32 {
        // y >= 0: surface checkerboard pattern (only at y=0)
        if y == 0 {
            let is_light = (x + z) % 2 == 0;
            return if is_light { 1 } else { 2 };
        }

        if y > 0 {
            // Above ground: empty
            return 0;
        }

        // y < 0: underground terrain with noise and waves
        let scale = 0.1;
        let wx = x as f64 * scale;
        let wy = y as f64 * scale;
        let wz = z as f64 * scale;

        // Combine multiple noise functions
        let base_noise = noise.get([wx, wy, wz]);
        let fbm_noise = fbm.get([wx * 0.5, wy * 0.5, wz * 0.5]);

        // Wave function for variation
        let wave = ((wx * 2.0).sin() + (wz * 2.0).cos()) * 0.2;

        // Density increases with depth
        let depth_factor = (-y as f64) * 0.1;

        // Combine all factors
        let density = base_noise + fbm_noise * 0.5 + wave + depth_factor;

        // Threshold to determine if voxel is solid
        // Higher density = more likely to be solid
        if density > 0.3 {
            // Vary color based on depth and noise
            let color_value = ((density * 10.0) as i32 % 8) + 3;
            color_value.clamp(3, 10)
        } else {
            0 // Empty/air
        }
    }

    pub fn generate_mesh(&self) -> GeometryData {
        // Generate mesh from octree using the cube mesher with custom color mapper
        let color_mapper = DawnbringerColorMapper::new();
        let mesh_data = crossworld_cube::generate_mesh_with_mapper(&self.octree, &color_mapper);

        // Scale and offset vertices to match world coordinates
        // The octree generates a 16x16x16 voxel grid within a 1.0 unit cube
        // We need:
        // - x: 0-8 world units (scale by 0.5)
        // - y: -8 to 8 world units (scale by 1.0, offset by -8)
        // - z: 0-8 world units (scale by 0.5)
        let scaled_vertices: Vec<f32> = mesh_data
            .vertices
            .chunks(3)
            .flat_map(|chunk| {
                let x = chunk[0] * 0.5 * 16.0; // 0.5 * 16 = 8 units
                let y = chunk[1] * 1.0 * 16.0 - 8.0; // 16 units, offset by -8
                let z = chunk[2] * 0.5 * 16.0; // 0.5 * 16 = 8 units
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
        Self::new()
    }
}

/// Get octant offset for given index (0-7)
/// Same logic as the removed Octant enum
fn octant_offset(index: usize) -> (f32, f32, f32) {
    let x = if index & 0b100 != 0 { 0.5 } else { 0.0 };
    let y = if index & 0b010 != 0 { 0.5 } else { 0.0 };
    let z = if index & 0b001 != 0 { 0.5 } else { 0.0 };
    (x, y, z)
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
