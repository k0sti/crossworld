use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

use crate::{
    generate_mesh_hierarchical, parse_csm, serialize_csm, ColorMapper, Cube, DefaultMeshBuilder,
    HsvColorMapper, Octree, PaletteColorMapper,
};

#[derive(Serialize, Deserialize)]
pub struct MeshResult {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
    pub normals: Vec<f32>,
    pub colors: Vec<f32>,
}

#[derive(Serialize, Deserialize)]
pub struct ParseError {
    pub error: String,
}

#[derive(Clone)]
struct ModelData {
    cube: Cube<i32>,
    max_depth: usize,
    palette: Option<Vec<[f32; 3]>>,
}

// Thread-local model storage (WASM is single-threaded)
thread_local! {
    static MODEL_STORAGE: RefCell<HashMap<String, ModelData>> = RefCell::new(HashMap::new());
}

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    // Set panic hook for better error messages
    console_error_panic_hook::set_once();

    // Initialize tracing
    tracing_wasm::set_as_global_default();
}

/// Parse CSM code and generate mesh data
#[wasm_bindgen]
pub fn parse_csm_to_mesh(csm_code: &str) -> JsValue {
    match parse_csm(csm_code) {
        Ok(octree) => {
            let mut builder = DefaultMeshBuilder::new();
            let mapper = HsvColorMapper::new();
            generate_mesh_hierarchical(&octree, &mut builder, |v| mapper.map(v), 16);

            let result = MeshResult {
                vertices: builder.vertices,
                indices: builder.indices,
                normals: builder.normals,
                colors: builder.colors,
            };
            serde_wasm_bindgen::to_value(&result).unwrap_or_else(|e| {
                let error = ParseError {
                    error: format!("Serialization error: {}", e),
                };
                serde_wasm_bindgen::to_value(&error).unwrap()
            })
        }
        Err(e) => {
            let error = ParseError {
                error: format!("Parse error: {}", e),
            };
            serde_wasm_bindgen::to_value(&error).unwrap()
        }
    }
}

/// Validate CSM code without generating mesh
#[wasm_bindgen]
pub fn validate_csm(csm_code: &str) -> JsValue {
    match parse_csm(csm_code) {
        Ok(_) => JsValue::NULL,
        Err(e) => {
            let error = ParseError {
                error: format!("{}", e),
            };
            serde_wasm_bindgen::to_value(&error).unwrap()
        }
    }
}

/// Create new model with given ID and max depth (size = 2^depth)
#[wasm_bindgen]
pub fn create_model(model_id: &str, max_depth: usize) -> JsValue {
    MODEL_STORAGE.with(|storage| {
        let mut models = storage.borrow_mut();
        // Create empty cube (all voxels are -1 = empty)
        let cube = Cube::Solid(-1);
        models.insert(
            model_id.to_string(),
            ModelData {
                cube,
                max_depth,
                palette: None,
            },
        );
        JsValue::NULL
    })
}

/// Draw voxel at position with color index and depth level
/// depth: subdivision depth (2, 3, or 4)
/// xyz: coordinates [0, 2^max_depth) at max depth level
/// color_index: color palette index (0-255)
#[wasm_bindgen]
pub fn draw(
    model_id: &str,
    color_index: i32,
    x: usize,
    y: usize,
    z: usize,
    depth: usize,
) -> JsValue {
    MODEL_STORAGE.with(|storage| {
        let mut models = storage.borrow_mut();
        if let Some(model_data) = models.get_mut(model_id) {
            let max_depth = model_data.max_depth;

            // Validate depth
            if depth > max_depth || depth < 1 {
                let error = ParseError {
                    error: format!("Invalid depth: {} (must be 1-{})", depth, max_depth),
                };
                return serde_wasm_bindgen::to_value(&error).unwrap();
            }

            // Validate coordinates
            let max_coord = 1 << max_depth; // 2^max_depth
            if x >= max_coord || y >= max_coord || z >= max_coord {
                let error = ParseError {
                    error: format!(
                        "Coordinates out of bounds: ({},{},{}), max: {}",
                        x, y, z, max_coord
                    ),
                };
                return serde_wasm_bindgen::to_value(&error).unwrap();
            }

            // Calculate voxel size at this depth
            let voxel_size = 1 << (max_depth - depth); // 2^(max_depth - depth)

            // Align coordinates to voxel grid at this depth
            let aligned_x = (x / voxel_size) * voxel_size;
            let aligned_y = (y / voxel_size) * voxel_size;
            let aligned_z = (z / voxel_size) * voxel_size;

            // Build path to voxel
            let path = build_octree_path(aligned_x, aligned_y, aligned_z, depth, max_depth);

            // Set voxel at path
            let new_cube = set_voxel_at_path(&model_data.cube, &path, color_index);
            model_data.cube = new_cube;

            JsValue::NULL
        } else {
            let error = ParseError {
                error: format!("Model not found: {}", model_id),
            };
            serde_wasm_bindgen::to_value(&error).unwrap()
        }
    })
}

/// Set color palette for a model (array of hex color strings like ["#ff0000", "#00ff00"])
#[wasm_bindgen]
pub fn set_model_palette(model_id: &str, palette_hex: Vec<String>) -> JsValue {
    MODEL_STORAGE.with(|storage| {
        let mut models = storage.borrow_mut();
        if let Some(model_data) = models.get_mut(model_id) {
            // Convert hex colors to RGB f32 arrays
            let mut colors = Vec::new();
            for hex in palette_hex {
                let hex = hex.trim_start_matches('#');
                if hex.len() == 6 {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&hex[0..2], 16),
                        u8::from_str_radix(&hex[2..4], 16),
                        u8::from_str_radix(&hex[4..6], 16),
                    ) {
                        colors.push([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]);
                    }
                }
            }
            model_data.palette = Some(colors);
            JsValue::NULL
        } else {
            let error = ParseError {
                error: format!("Model not found: {}", model_id),
            };
            serde_wasm_bindgen::to_value(&error).unwrap()
        }
    })
}

/// Get mesh data for a model
#[wasm_bindgen]
pub fn get_model_mesh(model_id: &str) -> JsValue {
    MODEL_STORAGE.with(|storage| {
        let models = storage.borrow();
        if let Some(model_data) = models.get(model_id) {
            let octree = Octree::new(model_data.cube.clone());

            // Use palette if available, otherwise HSV
            let mut builder = DefaultMeshBuilder::new();
            if let Some(ref palette) = model_data.palette {
                let mapper = PaletteColorMapper::new(palette.clone());
                generate_mesh_hierarchical(
                    &octree,
                    &mut builder,
                    |v| mapper.map(v),
                    model_data.max_depth as u32,
                );
            } else {
                let mapper = HsvColorMapper::new();
                generate_mesh_hierarchical(
                    &octree,
                    &mut builder,
                    |v| mapper.map(v),
                    model_data.max_depth as u32,
                );
            };

            // Scale mesh from [0,1] space to [0, 2^max_depth] world space
            let scale = (1 << model_data.max_depth) as f32;
            for i in 0..builder.vertices.len() {
                builder.vertices[i] *= scale;
            }

            let result = MeshResult {
                vertices: builder.vertices,
                indices: builder.indices,
                normals: builder.normals,
                colors: builder.colors,
            };
            serde_wasm_bindgen::to_value(&result).unwrap_or_else(|e| {
                let error = ParseError {
                    error: format!("Serialization error: {}", e),
                };
                serde_wasm_bindgen::to_value(&error).unwrap()
            })
        } else {
            let error = ParseError {
                error: format!("Model not found: {}", model_id),
            };
            serde_wasm_bindgen::to_value(&error).unwrap()
        }
    })
}

// Helper: Build octree path from coordinates
fn build_octree_path(x: usize, y: usize, z: usize, depth: usize, max_depth: usize) -> Vec<usize> {
    let mut path = Vec::with_capacity(depth);
    let mut curr_x = x;
    let mut curr_y = y;
    let mut curr_z = z;

    for level in (max_depth - depth..max_depth).rev() {
        let half = 1 << level; // 2^level
        let octant = ((curr_x >= half) as usize) * 4
            + ((curr_y >= half) as usize) * 2
            + ((curr_z >= half) as usize);

        path.push(octant);

        if curr_x >= half {
            curr_x -= half;
        }
        if curr_y >= half {
            curr_y -= half;
        }
        if curr_z >= half {
            curr_z -= half;
        }
    }

    path
}

/// Serialize a model to CSM format
#[wasm_bindgen]
pub fn serialize_model_to_csm(model_id: &str) -> JsValue {
    MODEL_STORAGE.with(|storage| {
        let models = storage.borrow();
        if let Some(model_data) = models.get(model_id) {
            let octree = Octree::new(model_data.cube.clone());
            let csm_text = serialize_csm(&octree);
            JsValue::from_str(&csm_text)
        } else {
            let error = ParseError {
                error: format!("Model not found: {}", model_id),
            };
            serde_wasm_bindgen::to_value(&error).unwrap()
        }
    })
}

/// Load a model from CSM text
#[wasm_bindgen]
pub fn load_model_from_csm(model_id: &str, csm_text: &str, max_depth: usize) -> JsValue {
    match parse_csm(csm_text) {
        Ok(octree) => {
            MODEL_STORAGE.with(|storage| {
                let mut models = storage.borrow_mut();
                models.insert(
                    model_id.to_string(),
                    ModelData {
                        cube: octree.root,
                        max_depth,
                        palette: None,
                    },
                );
                JsValue::NULL
            })
        }
        Err(e) => {
            let error = ParseError {
                error: format!("Parse error: {}", e),
            };
            serde_wasm_bindgen::to_value(&error).unwrap()
        }
    }
}

// Helper: Set voxel value at octree path
fn set_voxel_at_path(cube: &Cube<i32>, path: &[usize], color_index: i32) -> Cube<i32> {
    if path.is_empty() {
        return Cube::Solid(color_index);
    }

    match cube {
        Cube::Solid(_) => {
            // Need to subdivide
            let mut children: Vec<Rc<Cube<i32>>> =
                (0..8).map(|_| Rc::new(Cube::Solid(-1))).collect();

            let next_cube = set_voxel_at_path(&Cube::Solid(-1), &path[1..], color_index);
            children[path[0]] = Rc::new(next_cube);

            Cube::cubes(children.try_into().unwrap())
        }
        Cube::Cubes(children) => {
            let mut new_children: Vec<Rc<Cube<i32>>> = children.to_vec();
            let next_cube = set_voxel_at_path(&children[path[0]], &path[1..], color_index);
            new_children[path[0]] = Rc::new(next_cube);
            Cube::cubes(new_children.try_into().unwrap())
        }
        Cube::Planes { .. } | Cube::Slices { .. } => {
            // For simplicity, convert to regular octree
            Cube::Solid(color_index)
        }
    }
}
