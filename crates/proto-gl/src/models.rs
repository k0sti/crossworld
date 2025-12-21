use std::rc::Rc;
use std::path::Path;
use cube::{Cube, load_vox_to_cube};
use crossworld_physics::rapier3d::prelude::{RigidBodyHandle, ColliderHandle};
use glam::Vec3;
use serde::Deserialize;

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

/// A voxel model loaded from a .vox file
pub struct VoxModel {
    pub cube: Rc<Cube<u8>>,
    pub name: String,
    pub depth: u32,
    /// Scale exponent from CSV (actual_scale = 2^scale_exp)
    pub scale_exp: i32,
}

/// A dynamic cube object in the physics simulation
#[allow(dead_code)]
pub struct CubeObject {
    /// Voxel data
    pub cube: Rc<Cube<u8>>,
    /// Rapier rigid body handle
    pub body_handle: RigidBodyHandle,
    /// Rapier collider handle
    pub collider_handle: ColliderHandle,
    /// Model source (for identification)
    pub model_name: String,
    /// Octree depth for rendering
    pub depth: u32,
    /// Scale exponent from CSV (actual_scale = 2^scale_exp)
    pub scale_exp: i32,
}

/// Load object models from CSV (models with model_type="object")
pub fn load_vox_models(csv_path: &str, models_path: &str) -> Vec<VoxModel> {
    use std::fs;

    let mut models = Vec::new();
    let csv_path_obj = Path::new(csv_path);

    if !csv_path_obj.exists() {
        eprintln!("Warning: Models CSV not found: {}", csv_path);
        eprintln!("Using fallback simple cube models");
        return create_fallback_models();
    }

    // Load CSV and filter for object models
    let mut reader = match csv::Reader::from_path(csv_path_obj) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Warning: Failed to read CSV {}: {}", csv_path, e);
            return create_fallback_models();
        }
    };

    let object_entries: Vec<CsvModelEntry> = reader
        .deserialize()
        .filter_map(|r: Result<CsvModelEntry, _>| r.ok())
        .filter(|e| e.model_type == "object" && e.file_type == "vox")
        .collect();

    if object_entries.is_empty() {
        println!("No object models found in CSV (model_type='object')");
        return create_fallback_models();
    }

    println!("Found {} object entries in CSV", object_entries.len());

    for entry in object_entries {
        // Build full path: models_path + entry.path
        let file_path = Path::new(models_path).join(&entry.path);

        if !file_path.exists() {
            eprintln!("Warning: Model file not found: {}", file_path.display());
            continue;
        }

        // Parse scale exponent
        let scale_exp: i32 = entry.scale.trim().parse().unwrap_or(0);

        // Read file bytes
        let bytes = match fs::read(&file_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Warning: Failed to read {}: {}", file_path.display(), e);
                continue;
            }
        };

        // Load with center alignment
        match load_vox_to_cube(&bytes, Vec3::splat(0.5)) {
            Ok(cube) => {
                let depth = calculate_cube_depth(&cube);
                println!(
                    "Loaded object model: {} (depth {}, scale_exp {})",
                    entry.name, depth, scale_exp
                );
                models.push(VoxModel {
                    cube: Rc::new(cube),
                    name: entry.name.clone(),
                    depth,
                    scale_exp,
                });
            }
            Err(e) => {
                eprintln!("Warning: Failed to load {}: {}", file_path.display(), e);
            }
        }
    }

    // If no models loaded, use fallback
    if models.is_empty() {
        eprintln!("Warning: No object models loaded from CSV");
        return create_fallback_models();
    }

    println!("Loaded {} object model(s)", models.len());
    models
}

/// Create fallback simple cube models
fn create_fallback_models() -> Vec<VoxModel> {
    vec![
        VoxModel {
            cube: Rc::new(Cube::solid(5)), // Grass
            name: "simple_cube_grass".to_string(),
            depth: 0,
            scale_exp: 0,
        },
        VoxModel {
            cube: Rc::new(Cube::solid(4)), // Stone
            name: "simple_cube_stone".to_string(),
            depth: 0,
            scale_exp: 0,
        },
        VoxModel {
            cube: Rc::new(Cube::solid(9)), // Wood
            name: "simple_cube_wood".to_string(),
            depth: 0,
            scale_exp: 0,
        },
    ]
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
