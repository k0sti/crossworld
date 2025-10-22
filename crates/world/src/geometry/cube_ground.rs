use crate::GeometryData;
use crossworld_cube::{Cube, Octree};
use noise::{Fbm, NoiseFn, Perlin};
use std::rc::Rc;

pub struct CubeGround {
    octree: Octree,
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
        }
    }

    fn build_ground_octree(noise: &Perlin, fbm: &Fbm<Perlin>) -> Cube<i32> {
        // Build 8 children at level 1 (each represents 8x8x8 space)
        let level1_children: [Rc<Cube<i32>>; 8] = std::array::from_fn(|i| {
            Rc::new(Self::build_level2(i, noise, fbm))
        });

        Cube::cubes(level1_children)
    }

    fn build_level2(parent_idx: usize, noise: &Perlin, fbm: &Fbm<Perlin>) -> Cube<i32> {
        // Build 8 children at level 2 (each represents 4x4x4 space)
        let level2_children: [Rc<Cube<i32>>; 8] = std::array::from_fn(|i| {
            Rc::new(Self::build_level3(parent_idx, i, noise, fbm))
        });

        Cube::cubes(level2_children)
    }

    fn build_level3(
        parent1_idx: usize,
        parent2_idx: usize,
        noise: &Perlin,
        fbm: &Fbm<Perlin>,
    ) -> Cube<i32> {
        // Build 8 children at level 3 (each represents 2x2x2 space)
        let level3_children: [Rc<Cube<i32>>; 8] = std::array::from_fn(|i| {
            Rc::new(Self::build_level4(parent1_idx, parent2_idx, i, noise, fbm))
        });

        Cube::cubes(level3_children)
    }

    fn build_level4(
        parent1_idx: usize,
        parent2_idx: usize,
        parent3_idx: usize,
        noise: &Perlin,
        fbm: &Fbm<Perlin>,
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

            let value = Self::get_voxel_value(voxel_x, world_y, voxel_z, noise, fbm);
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
        // Generate mesh from octree using the cube mesher
        let mesh_data = crossworld_cube::generate_mesh(&self.octree);

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
