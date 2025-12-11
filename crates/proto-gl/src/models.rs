use std::rc::Rc;
use cube::{Cube, load_vox_to_cube};
use crossworld_physics::rapier3d::prelude::{RigidBodyHandle, ColliderHandle};
use glam::Vec3;

/// A voxel model loaded from a .vox file
pub struct VoxModel {
    pub cube: Rc<Cube<u8>>,
    pub name: String,
    pub depth: u32,
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
}

/// Load .vox models from a directory
pub fn load_vox_models(models_path: &str) -> Vec<VoxModel> {
    use std::fs;
    use std::path::Path;

    let mut models = Vec::new();

    // Check if directory exists
    let path = Path::new(models_path);
    if !path.exists() || !path.is_dir() {
        eprintln!("Warning: Models directory not found: {}", models_path);
        eprintln!("Creating fallback simple cube models");

        // Create a few simple cube models as fallback
        models.push(VoxModel {
            cube: Rc::new(Cube::solid(5)), // Grass
            name: "simple_cube_grass".to_string(),
            depth: 0,
        });
        models.push(VoxModel {
            cube: Rc::new(Cube::solid(4)), // Stone
            name: "simple_cube_stone".to_string(),
            depth: 0,
        });
        models.push(VoxModel {
            cube: Rc::new(Cube::solid(9)), // Wood
            name: "simple_cube_wood".to_string(),
            depth: 0,
        });

        return models;
    }

    // Load .vox files from directory
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let file_path = entry.path();
            if file_path.extension().map_or(false, |ext| ext == "vox") {
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
                        // Calculate depth from cube size
                        let depth = calculate_cube_depth(&cube);
                        let name = file_path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        models.push(VoxModel {
                            cube: Rc::new(cube),
                            name,
                            depth,
                        });
                        println!("Loaded model: {} (depth {})", file_path.display(), depth);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to load {}: {}", file_path.display(), e);
                    }
                }
            }
        }
    }

    // If no models loaded, use fallback
    if models.is_empty() {
        eprintln!("Warning: No .vox models found in {}", models_path);
        eprintln!("Using fallback simple cube models");
        models.push(VoxModel {
            cube: Rc::new(Cube::solid(5)),
            name: "fallback_cube".to_string(),
            depth: 0,
        });
    }

    println!("Loaded {} model(s)", models.len());
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
