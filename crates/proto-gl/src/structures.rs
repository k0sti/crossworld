//! Structure placement module for randomly placing vox models into the world cube.
//!
//! This module loads vox models marked as "structure" in models.csv and places them
//! randomly within the world cube, aligned to the ground (y < 0, at -half_world) and
//! rotated in 90° steps. The world uses origin-centered coordinates.

use crate::config::StructuresConfig;
use cube::{load_vox_to_cube, Axis, Cube};
use glam::{IVec3, Vec3};
use rand::prelude::*;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::rc::Rc;

/// CSV record for models.csv
#[derive(Debug, Clone, Deserialize)]
struct CsvModelEntry {
    name: String,
    path: String,
    file_type: String,
    #[allow(dead_code)]
    size: u64,
    model_type: String,
    scale: String,
    #[allow(dead_code)]
    notes: String,
}

/// A structure model loaded from a .vox file, ready for placement
pub struct StructureModel {
    /// The voxel cube data (y-aligned to bottom)
    pub cube: Cube<u8>,
    /// Model name (filename without extension)
    pub name: String,
    /// Octree depth of this model
    pub depth: u32,
    /// Scale exponent from CSV (actual_scale = 2^scale_exp)
    pub scale_exp: i32,
}

/// Load structure models from CSV (models with model_type="structure")
pub fn load_structure_models(config: &StructuresConfig) -> Vec<StructureModel> {
    let mut models = Vec::new();
    let csv_path = Path::new(&config.models_csv);

    if !csv_path.exists() {
        eprintln!(
            "Warning: Models CSV not found: {}",
            config.models_csv
        );
        return models;
    }

    // Load CSV and filter for structure models
    let mut reader = match csv::Reader::from_path(csv_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Warning: Failed to read CSV {}: {}", config.models_csv, e);
            return models;
        }
    };

    let structure_entries: Vec<CsvModelEntry> = reader
        .deserialize()
        .filter_map(|r: Result<CsvModelEntry, _>| r.ok())
        .filter(|e| e.model_type == "structure" && e.file_type == "vox")
        .collect();

    if structure_entries.is_empty() {
        println!("No structure models found in CSV (model_type='structure')");
        return models;
    }

    println!("Found {} structure entries in CSV", structure_entries.len());

    for entry in structure_entries {
        // Build full path: models_path + entry.path
        let file_path = Path::new(&config.models_path).join(&entry.path);

        if !file_path.exists() {
            eprintln!("Warning: Model file not found: {}", file_path.display());
            continue;
        }

        // Parse scale exponent
        let scale_exp: i32 = entry.scale.trim().parse().unwrap_or(0);

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
                        name: entry.name.clone(),
                        depth,
                        scale_exp,
                    });
                } else {
                    println!(
                        "Skipping {} (depth {} > max_depth {})",
                        entry.name, depth, config.max_depth
                    );
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to load {}: {}", file_path.display(), e);
            }
        }
    }

    // Sort by depth and show stats
    models.sort_by_key(|m| m.depth);
    let depths: Vec<(String, u32, i32)> = models
        .iter()
        .map(|m| (m.name.clone(), m.depth, m.scale_exp))
        .collect();
    println!(
        "Loaded {} structure models: {:?}",
        models.len(),
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
    let half_world = world_size as f32 / 2.0;

    // Calculate the scale factor to match model voxels with terrain voxels
    // The terrain content is at content_depth resolution, then expanded with borders
    // To make model voxels the same size as terrain voxels, we need to scale up
    // by the difference between world_depth and content_depth
    let depth_offset = world_depth - content_depth;

    // Spawn radius is specified in world units (centered at origin)
    let radius = config.spawn_radius;

    for _ in 0..config.count {
        // Select random model
        let model = &models[rng.gen_range(0..models.len())];

        // Calculate position in world coordinates (origin at center)
        // Random X, Z within spawn radius, Y at ground level
        let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
        let dist: f32 = rng.gen_range(0.0..radius);
        // Convert from origin-centered coords to octree coords (0 to world_size)
        let x = (half_world + angle.cos() * dist) as i32;
        let z = (half_world + angle.sin() * dist) as i32;

        // Find Y position: place on ground surface
        // Ground is at y = -half_world in world coords, which is y = 0 in octree coords
        // Place structures at ground surface (just above ground level)
        let y = 64; // Approximate ground surface in octree coords

        // Random rotation (0, 90, 180, or 270 degrees)
        let rotation = rng.gen_range(0..4);
        let rotated_cube = rotate_y_90(&model.cube, rotation);

        // Calculate model size at its native resolution
        let model_size = 1u32 << model.depth;

        // Base scale = model.depth + depth_offset means model voxels match terrain voxel size
        // A depth-2 model (4x4x4) with depth_offset=6 gets scale=8
        // This means the 4x4x4 model becomes 4*2^6 = 256 world voxels per axis
        //
        // Additional scaling from CSV: actual_scale = 2^scale_exp
        // Positive scale_exp = larger, Negative scale_exp = smaller
        // Final scale = base_scale + scale_exp
        let base_scale = (model.depth + depth_offset) as i32;
        let final_scale = (base_scale + model.scale_exp).max(0) as u32;

        // Calculate scaled model size in world coordinates
        // scale_exp affects the model size: if positive, model is larger
        let effective_depth_offset = (depth_offset as i32 + model.scale_exp).max(0) as u32;
        let scaled_model_size = model_size << effective_depth_offset;

        // Ensure position is within bounds
        let x = x.clamp(0, (world_size - scaled_model_size) as i32);
        let z = z.clamp(0, (world_size - scaled_model_size) as i32);

        // Place the structure using update_depth_tree
        let offset = IVec3::new(x, y, z);

        println!(
            "  Placing {} at ({}, {}, {}) rotation={}° native_size={} scaled_size={} scale={} (base={} + csv_scale={})...",
            model.name,
            x,
            y,
            z,
            rotation * 90,
            model_size,
            scaled_model_size,
            final_scale,
            base_scale,
            model.scale_exp
        );
        result = result.update_depth_tree(world_depth, offset, final_scale, &rotated_cube);
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
