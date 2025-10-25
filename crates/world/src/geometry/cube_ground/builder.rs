use crossworld_cube::Cube;
use noise::{Fbm, NoiseFn, Perlin};
use std::collections::HashMap;
use std::rc::Rc;

/// Build octree for ground without modifications
pub fn build_ground_octree(noise: &Perlin, fbm: &Fbm<Perlin>, depth: u32) -> Cube<i32> {
    let mods = HashMap::new();
    build_ground_octree_with_mods(noise, fbm, &mods, depth)
}

/// Build octree for ground with modifications applied
pub fn build_ground_octree_with_mods(
    noise: &Perlin,
    fbm: &Fbm<Perlin>,
    modifications: &HashMap<(i32, i32, i32), i32>,
    depth: u32,
) -> Cube<i32> {
    // Start recursive build at (0,0,0) with specified depth
    // e.g. Depth 4 -> 3 -> 2 -> 1 -> 0 (leaf voxels)
    build_octree_recursive(0, 0, 0, depth, noise, fbm, modifications)
}

/// Recursively build octree from given position and depth
///
/// - base_x, base_y, base_z: Position in voxel grid coordinates (0-30 range)
/// - depth: Current depth level (4 = root, 0 = leaf voxel)
fn build_octree_recursive(
    base_x: i32,
    base_y: i32,
    base_z: i32,
    depth: u32,
    noise: &Perlin,
    fbm: &Fbm<Perlin>,
    modifications: &HashMap<(i32, i32, i32), i32>,
) -> Cube<i32> {
    if depth == 0 {
        // Base case: create leaf voxel
        let voxel_x = base_x;
        let voxel_y = base_y;
        let voxel_z = base_z;

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
                get_voxel_value(voxel_x, world_y, voxel_z, noise, fbm)
            };

        return Cube::Solid(value);
    }

    // Recursive case: create 8 children at next depth level
    // Each level halves the step size: depth 4 -> step 8, depth 3 -> step 4, etc.
    let step = 1 << depth; // 2^depth

    let children: [Rc<Cube<i32>>; 8] = std::array::from_fn(|octant_idx| {
        let (ox, oy, oz) = octant_offset(octant_idx);
        let child_x = base_x + (ox * step as f32) as i32;
        let child_y = base_y + (oy * step as f32) as i32;
        let child_z = base_z + (oz * step as f32) as i32;

        Rc::new(build_octree_recursive(
            child_x,
            child_y,
            child_z,
            depth - 1,
            noise,
            fbm,
            modifications,
        ))
    });

    Cube::cubes(children)
}

/// Get voxel value at given coordinates
fn get_voxel_value(x: i32, y: i32, z: i32, noise: &Perlin, fbm: &Fbm<Perlin>) -> i32 {
    // y >= 0: surface checkerboard pattern (only at y=0)

    if y >= 0 {
        // Above ground: empty
        return 0;
    }
    // if y == -1 0 {
    //     let is_light = (x + z) % 2 == 0;
    //     return if is_light { 1 } else { 2 };
    // }

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

/// Get octant offset for given index (0-7)
fn octant_offset(index: usize) -> (f32, f32, f32) {
    let x = if index & 0b100 != 0 { 0.5 } else { 0.0 };
    let y = if index & 0b010 != 0 { 0.5 } else { 0.0 };
    let z = if index & 0b001 != 0 { 0.5 } else { 0.0 };
    (x, y, z)
}
