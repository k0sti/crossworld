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
        let half_grid = (1 << max_depth); // 2^max_depth / 2
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
    // y > 0: Above ground - no voxels
    // y = 0: Surface level with 50% coverage
    // y < 0: Underground with increasing coverage
    if y >= 0 {
        return 0;
    }
    

    // y < 0: Underground terrain with noise
    // Use larger scale for more variation
    let scale = 0.1; // Increased from 0.1 for higher frequency noise
    let wx = x as f64 * scale;
    let wy = y as f64 * scale;
    let wz = z as f64 * scale;

    // Combine multiple noise functions for natural terrain
    let base_noise = noise.get([wx, wy, wz]);
    let fbm_noise = fbm.get([wx * 0.5, wy * 0.5, wz * 0.5]);

    // Additional detail layer
    let detail_noise = noise.get([wx * 2.0, wy * 2.0, wz * 2.0]) * 0.15;

    // Wave function for variation
    let wave = ((wx * 1.5).sin() + (wz * 1.5).cos()) * 0.00001;

    // Calculate base density
    let base_density = base_noise + fbm_noise * 0.3 + wave + detail_noise;

    // Depth-based threshold adjustment
    // At y=0: threshold = 0.0 (50% coverage, noise is centered around 0)
    // At y=-16: threshold = -1.0 (100% coverage, almost everything is solid)
    // Linear interpolation between y=0 and y=-16
    let depth = -y as f64;
    let threshold = if depth <= 16.0 {
        -depth / 16.0 // Interpolate from 0 to -1
    } else {
        -1.0 // Beyond -16, keep at 100% coverage
    };

    // Check if voxel should be solid
    if base_density > threshold {
        // Use smooth color gradient based on position and noise
        // Combine horizontal position with noise for continuous color variation
        let color_noise = noise.get([wx * 0.3, wy * 0.3, wz * 0.3]);

        // Create color value that varies smoothly (3-10 range, 8 colors)
        // Mix position-based and noise-based components
        let position_component = ((x as f64 * 0.2 + z as f64 * 0.2).sin() * 0.5 + 0.5) * 3.0;
        let noise_component = (color_noise * 0.5 + 0.5) * 4.0;
        let depth_component = (depth / 8.0).min(1.0); // Subtle depth variation

        let color_value = (position_component + noise_component + depth_component) as i32 + 3;
        color_value.clamp(3, 10)
    } else {
        0 // Empty/air
    }
}

