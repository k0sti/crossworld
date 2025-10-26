use crossworld_cube::{glam::IVec3, Cube, IVec3Ext};
use noise::{Fbm, NoiseFn, Perlin};
use std::rc::Rc;

/// Build octree for ground
pub fn build_ground_octree(noise: &Perlin, fbm: &Fbm<Perlin>, depth: u32) -> Cube<i32> {
    // Start recursive build at (0,0,0) with specified depth
    // e.g. Depth 7 -> 6 -> 5 -> ... -> 1 -> 0 (leaf voxels)
    // Pass depth to know the grid size for centering
    build_octree_recursive(0, 0, 0, depth, depth, noise, fbm)
}

/// Recursively build octree from given position and depth
///
/// - base_x, base_y, base_z: Position in voxel grid coordinates [0, 2^max_depth)
/// - depth: Current depth level (max_depth = root, 0 = leaf voxel)
/// - max_depth: Maximum octree depth (for centering calculations)
fn build_octree_recursive(
    base_x: i32,
    base_y: i32,
    base_z: i32,
    depth: u32,
    max_depth: u32,
    noise: &Perlin,
    fbm: &Fbm<Perlin>,
) -> Cube<i32> {
    if depth == 0 {
        // Base case: create leaf voxel
        let voxel_x = base_x;
        let voxel_y = base_y;
        let voxel_z = base_z;

        // Convert to centered coordinates
        // For depth 7: grid is [0, 128), center is 64, so [-64, 64)
        let half_grid = (1 << max_depth) / 2; // 2^max_depth / 2
        let world_y = voxel_y - half_grid;

        let value = get_voxel_value(voxel_x, world_y, voxel_z, noise, fbm);

        return Cube::Solid(value);
    }

    // Recursive case: create 8 children at next depth level
    // Each level halves the step size: depth 7 -> step 64, depth 6 -> step 32, etc.
    let step = 1 << depth; // 2^depth

    let children: [Rc<Cube<i32>>; 8] = std::array::from_fn(|octant_idx| {
        let offset = IVec3::from_octant_index(octant_idx);
        let child_x = base_x + offset.x * step;
        let child_y = base_y + offset.y * step;
        let child_z = base_z + offset.z * step;

        Rc::new(build_octree_recursive(
            child_x,
            child_y,
            child_z,
            depth - 1,
            max_depth,
            noise,
            fbm,
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

