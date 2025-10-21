use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{parse_csm, generate_mesh};

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
            let mesh = generate_mesh(&octree);
            let result = MeshResult {
                vertices: mesh.vertices,
                indices: mesh.indices,
                normals: mesh.normals,
                colors: mesh.colors,
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
