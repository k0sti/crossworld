//! Structure placement module for randomly placing vox models into the world cube.
//!
//! This module loads vox models with a specific prefix (e.g., "obj_") and places them
//! randomly within the world cube, aligned to the ground (y=0) and rotated in 90° steps.

use crate::config::StructuresConfig;
use cube::{load_vox_to_cube, Axis, Cube};
use glam::{IVec3, Vec3};
use rand::prelude::*;
use std::fs;
use std::path::Path;
use std::rc::Rc;

/// A structure model loaded from a .vox file, ready for placement
pub struct StructureModel {
    /// The voxel cube data (y-aligned to bottom)
    pub cube: Cube<u8>,
    /// Model name (filename without extension)
    pub name: String,
    /// Octree depth of this model
    pub depth: u32,
}

/// Load structure models from directory matching the prefix filter
pub fn load_structure_models(config: &StructuresConfig) -> Vec<StructureModel> {
    let mut models = Vec::new();
    let path = Path::new(&config.models_path);

    if !path.exists() || !path.is_dir() {
        eprintln!(
            "Warning: Structures directory not found: {}",
            config.models_path
        );
        return models;
    }

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let file_path = entry.path();

            // Check if it's a .vox file
            if file_path.extension().is_none_or(|ext| ext != "vox") {
                continue;
            }

            // Check prefix filter
            let file_name = file_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            if !file_name.starts_with(&config.prefix) {
                continue;
            }

            // Read and load the vox file
            let bytes = match fs::read(&file_path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("Warning: Failed to read {}: {}", file_path.display(), e);
                    continue;
                }
            };

            // Load with bottom alignment (align.y = 0.0 means model starts at y=0)
            // Center on X and Z axes (0.5), align bottom on Y (0.0)
            match load_vox_to_cube(&bytes, Vec3::new(0.5, 0.0, 0.5)) {
                Ok(cube) => {
                    let depth = calculate_cube_depth(&cube);
                    // Filter by max_depth for performance
                    if depth <= config.max_depth {
                        models.push(StructureModel {
                            cube,
                            name: file_name,
                            depth,
                        });
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to load {}: {}", file_path.display(), e);
                }
            }
        }
    }

    // Sort by depth and show stats
    models.sort_by_key(|m| m.depth);
    let depths: Vec<u32> = models.iter().map(|m| m.depth).collect();
    println!(
        "Loaded {} structure models with prefix '{}', depths: {:?}",
        models.len(),
        config.prefix,
        depths
    );
    models
}

/// Calculate the depth of a cube (how many levels of octree)
fn calculate_cube_depth(cube: &Cube<u8>) -> u32 {
    fn depth_recursive(cube: &Cube<u8>) -> u32 {
        match cube {
            Cube::Solid(_) => 0,
            Cube::Cubes(children) => {
                1 + children
                    .iter()
                    .map(|c| depth_recursive(c))
                    .max()
                    .unwrap_or(0)
            }
            Cube::Quad { quads, .. } => {
                1 + quads
                    .iter()
                    .map(|c| depth_recursive(c))
                    .max()
                    .unwrap_or(0)
            }
            Cube::Layers { layers, .. } => {
                1 + layers
                    .iter()
                    .map(|c| depth_recursive(c))
                    .max()
                    .unwrap_or(0)
            }
        }
    }
    depth_recursive(cube)
}

/// Rotate a cube by 90° around Y axis (0, 1, 2, or 3 times for 0°, 90°, 180°, 270°)
fn rotate_y_90(cube: &Cube<u8>, rotations: u32) -> Cube<u8> {
    let mut result = cube.clone();
    for _ in 0..(rotations % 4) {
        // Rotate 90° around Y: swap X and Z axes, then mirror X
        result = result.apply_mirror(&[Axis::PosX]);
        result = swap_xz(&result);
    }
    result
}

/// Swap X and Z axes in the octree
fn swap_xz(cube: &Cube<u8>) -> Cube<u8> {
    match cube {
        Cube::Solid(v) => Cube::Solid(*v),
        Cube::Cubes(children) => {
            // Octant indexing: x + y*2 + z*4
            // Swap X and Z: new_index = z + y*2 + x*4
            let new_children: [Rc<Cube<u8>>; 8] = std::array::from_fn(|i| {
                let x = i & 1;
                let y = (i >> 1) & 1;
                let z = (i >> 2) & 1;
                let old_index = z | (y << 1) | (x << 2);
                Rc::new(swap_xz(&children[old_index]))
            });
            Cube::Cubes(Box::new(new_children))
        }
        Cube::Quad { axis, quads } => Cube::Quad {
            axis: *axis,
            quads: quads.clone(),
        },
        Cube::Layers { axis, layers } => Cube::Layers {
            axis: *axis,
            layers: layers.clone(),
        },
    }
}

/// Place structures randomly into the world cube
///
/// # Arguments
/// * `world_cube` - The world cube to modify
/// * `world_depth` - Total depth of the world cube
/// * `content_depth` - Depth of the original content (before border expansion)
/// * `config` - Structure placement configuration
/// * `models` - List of loaded structure models
///
/// # Returns
/// A new world cube with structures placed
pub fn place_structures(
    world_cube: &Cube<u8>,
    world_depth: u32,
    content_depth: u32,
    config: &StructuresConfig,
    models: &[StructureModel],
) -> Cube<u8> {
    if models.is_empty() || config.count == 0 {
        return world_cube.clone();
    }

    // Initialize RNG with seed (use current time if seed is 0)
    let seed = if config.seed == 0 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    } else {
        config.seed
    };
    let mut rng = StdRng::seed_from_u64(seed);
    println!("Placing {} structures with seed {}", config.count, seed);

    let mut result = world_cube.clone();
    let world_size = 1u32 << world_depth; // 2^world_depth

    // Calculate the scale factor to match model voxels with terrain voxels
    // The terrain content is at content_depth resolution, then expanded with borders
    // To make model voxels the same size as terrain voxels, we need to scale up
    // by the difference between world_depth and content_depth
    let depth_offset = world_depth - content_depth;

    // Calculate spawn area (centered around 0.5, 0.5 in XZ)
    let center = world_size as f32 / 2.0;
    let radius = config.spawn_radius * world_size as f32;

    for _ in 0..config.count {
        // Select random model
        let model = &models[rng.gen_range(0..models.len())];

        // Calculate position in world coordinates
        // Random X, Z within spawn radius, Y starting above 0
        let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
        let dist: f32 = rng.gen_range(0.0..radius);
        let x = (center + angle.cos() * dist) as i32;
        let z = (center + angle.sin() * dist) as i32;

        // Find Y position: place on ground surface
        // The world uses border expansion. With 6 border expansions:
        // - Ground extends from y=0 to approximately y=world_size/2
        // - Air is above that
        // The exact ground level is at (world_size/2) + (world_size/128) for typical configs
        // Place structures at the ground surface (just above ground level)
        let y = (world_size / 2) as i32 + 64; // Approximate ground surface

        // Random rotation (0, 90, 180, or 270 degrees)
        let rotation = rng.gen_range(0..4);
        let rotated_cube = rotate_y_90(&model.cube, rotation);

        // Calculate model size at its native resolution
        let model_size = 1u32 << model.depth;

        // Scale = model.depth + depth_offset means model voxels match terrain voxel size
        // A depth-2 model (4x4x4) with depth_offset=6 gets scale=8
        // This means the 4x4x4 model becomes 4*2^6 = 256 world voxels per axis
        let scale = model.depth + depth_offset;

        // Calculate scaled model size in world coordinates
        let scaled_model_size = model_size << depth_offset; // model_size * 2^depth_offset

        // Ensure position is within bounds
        let x = x.clamp(0, (world_size - scaled_model_size) as i32);
        let z = z.clamp(0, (world_size - scaled_model_size) as i32);

        // Place the structure using update_depth_tree
        let offset = IVec3::new(x, y, z);

        println!(
            "  Placing {} at ({}, {}, {}) rotation={}° native_size={} scaled_size={} scale={}...",
            model.name,
            x,
            y,
            z,
            rotation * 90,
            model_size,
            scaled_model_size,
            scale
        );
        result = result.update_depth_tree(world_depth, offset, scale, &rotated_cube);
        println!("    Done.");
    }

    // Simplify to collapse any uniform regions
    result.simplified()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_cube_depth() {
        // Solid cube has depth 0
        let solid = Cube::Solid(5u8);
        assert_eq!(calculate_cube_depth(&solid), 0);

        // Single level of children has depth 1
        let one_level = Cube::cubes(std::array::from_fn(|_| Rc::new(Cube::Solid(5u8))));
        assert_eq!(calculate_cube_depth(&one_level), 1);
    }

    #[test]
    fn test_swap_xz() {
        // Create a cube with different values in different octants
        let cube = Cube::cubes([
            Rc::new(Cube::Solid(0)), // x=0, y=0, z=0
            Rc::new(Cube::Solid(1)), // x=1, y=0, z=0
            Rc::new(Cube::Solid(2)), // x=0, y=1, z=0
            Rc::new(Cube::Solid(3)), // x=1, y=1, z=0
            Rc::new(Cube::Solid(4)), // x=0, y=0, z=1
            Rc::new(Cube::Solid(5)), // x=1, y=0, z=1
            Rc::new(Cube::Solid(6)), // x=0, y=1, z=1
            Rc::new(Cube::Solid(7)), // x=1, y=1, z=1
        ]);

        let swapped = swap_xz(&cube);

        // After swapping X and Z:
        // Old (x=1, y=0, z=0) -> New (x=0, y=0, z=1), value 1 moves to index 4
        // Old (x=0, y=0, z=1) -> New (x=1, y=0, z=0), value 4 moves to index 1
        if let Cube::Cubes(children) = swapped {
            assert_eq!(children[0].id(), 0); // (0,0,0) stays
            assert_eq!(children[1].id(), 4); // was (0,0,1) now (1,0,0)
            assert_eq!(children[4].id(), 1); // was (1,0,0) now (0,0,1)
            assert_eq!(children[7].id(), 7); // (1,1,1) stays
        } else {
            panic!("Expected Cubes variant");
        }
    }
}
