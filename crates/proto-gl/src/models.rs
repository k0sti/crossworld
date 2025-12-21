use std::path::Path;
use std::rc::Rc;

use cube::{Cube, CubeBox, load_vox_to_cubebox_compact};
use crossworld_physics::rapier3d::prelude::{ColliderHandle, RigidBodyHandle};
use glam::IVec3;
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
    /// The bounded voxel model with preserved dimensions
    pub cubebox: CubeBox<u8>,
    pub name: String,
    /// Scale exponent from CSV (actual_scale = 2^scale_exp)
    pub scale_exp: i32,
}

impl VoxModel {
    /// Get the cube reference for rendering
    pub fn cube(&self) -> &Cube<u8> {
        &self.cubebox.cube
    }

    /// Get a shared reference to the cube
    pub fn cube_rc(&self) -> Rc<Cube<u8>> {
        Rc::new(self.cubebox.cube.clone())
    }

    /// Get the octree depth
    pub fn depth(&self) -> u32 {
        self.cubebox.depth
    }

    /// Get the original model size in voxels
    pub fn size(&self) -> IVec3 {
        self.cubebox.size
    }
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
    /// Original model size in voxels (for accurate bounding box)
    pub model_size: IVec3,
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

        // Load with compact CubeBox (tight bounds around actual voxels)
        match load_vox_to_cubebox_compact(&bytes) {
            Ok(cubebox) => {
                println!(
                    "Loaded object model: {} (size {:?}, depth {}, scale_exp {})",
                    entry.name, cubebox.size, cubebox.depth, scale_exp
                );
                models.push(VoxModel {
                    cubebox,
                    name: entry.name.clone(),
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
            cubebox: CubeBox::new(Cube::solid(5), IVec3::ONE, 0), // Grass
            name: "simple_cube_grass".to_string(),
            scale_exp: 0,
        },
        VoxModel {
            cubebox: CubeBox::new(Cube::solid(4), IVec3::ONE, 0), // Stone
            name: "simple_cube_stone".to_string(),
            scale_exp: 0,
        },
        VoxModel {
            cubebox: CubeBox::new(Cube::solid(9), IVec3::ONE, 0), // Wood
            name: "simple_cube_wood".to_string(),
            scale_exp: 0,
        },
    ]
}

