mod avatar;
mod entity;
mod identity;
mod world_cube;

// Re-export entity system types
pub use avatar::Avatar;
pub use entity::{Entity, Logic};
pub use identity::Identity;

// Re-export Object trait from physics for convenience
pub use crossworld_physics::Object;

#[cfg(test)]
mod tests;

// Re-export for native use (non-WASM)
#[cfg(not(target_arch = "wasm32"))]
pub use world_cube::{World, WorldCube as NativeWorldCube};

// GeometryData is used by both WASM and native, but defined differently
// For native, just re-export the WASM version's fields
#[cfg(not(target_arch = "wasm32"))]
pub use GeometryData as NativeGeometryData;

use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use world_cube::WorldCube as WorldCubeInternal;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
}

/// WorldCube - The main world terrain cube
///
/// This replaces the old GeometryEngine with a simpler, direct interface.
#[wasm_bindgen]
pub struct WorldCube {
    inner: RefCell<WorldCubeInternal>,
}

#[wasm_bindgen]
impl WorldCube {
    #[wasm_bindgen(constructor)]
    pub fn new(macro_depth: u32, micro_depth: u32, border_depth: u32, seed: u32) -> Self {
        Self {
            inner: RefCell::new(WorldCubeInternal::new(
                macro_depth,
                micro_depth,
                border_depth,
                seed,
            )),
        }
    }

    #[wasm_bindgen(js_name = generateFrame)]
    pub fn generate_frame(&self) -> GeometryData {
        self.inner.borrow().generate_mesh()
    }

    /// Set voxel in world cube at specified depth
    /// depth: octree depth (7=finest detail, 4=coarse, etc.)
    #[wasm_bindgen(js_name = setVoxelAtDepth)]
    pub fn set_voxel_at_depth(&self, x: i32, y: i32, z: i32, depth: u32, color_index: u8) {
        // Validate coordinates before borrowing to avoid poisoning RefCell on error
        let max_coord = (1 << depth) - 1;
        if x < 0 || x > max_coord || y < 0 || y > max_coord || z < 0 || z > max_coord {
            tracing::error!(
                "Invalid coordinates ({}, {}, {}) for depth {}. Max coord: {}",
                x,
                y,
                z,
                depth,
                max_coord
            );
            return;
        }

        self.inner
            .borrow_mut()
            .set_voxel_at_depth(x, y, z, depth, color_index.clamp(0, 255));
    }

    /// Remove voxel from world cube at specified depth
    #[wasm_bindgen(js_name = removeVoxelAtDepth)]
    pub fn remove_voxel_at_depth(&self, x: i32, y: i32, z: i32, depth: u32) {
        self.inner
            .borrow_mut()
            .remove_voxel_at_depth(x, y, z, depth);
    }

    /// Export the current world state to CSM format
    #[wasm_bindgen(js_name = exportToCSM)]
    pub fn export_to_csm(&self) -> String {
        self.inner.borrow().export_to_csm()
    }

    /// Get reference to the root cube (NEW unified interface method)
    ///
    /// This enables direct manipulation using the unified Cube interface.
    /// Returns a serialized cube that can be deserialized on the JS side.
    pub fn root(&self) -> String {
        // For now, just export as CSM
        // TODO: Return actual WasmCube reference when cross-crate WASM types are sorted out
        self.export_to_csm()
    }

    /// Set a new root cube (NEW unified interface method)
    ///
    /// Load a cube from CSM format and replace the entire world.
    ///
    /// # Arguments
    /// * `csm_code` - Cubescript format text
    #[wasm_bindgen(js_name = setRoot)]
    pub fn set_root(&self, csm_code: &str) -> Result<(), JsValue> {
        match cube::parse_csm(csm_code) {
            Ok(cube) => {
                self.inner.borrow_mut().set_root(cube);
                Ok(())
            }
            Err(e) => Err(JsValue::from_str(&format!("Parse error: {}", e))),
        }
    }
}

impl Default for WorldCube {
    fn default() -> Self {
        Self::new(3, 0, 0, 0) // Default: macro depth 3, micro depth 0, no borders, seed 0
    }
}

#[wasm_bindgen]
pub struct GeometryData {
    vertices: Vec<f32>,
    indices: Vec<u32>,
    normals: Vec<f32>,
    colors: Vec<f32>,
    uvs: Vec<f32>,
    material_ids: Vec<u8>,
}

#[wasm_bindgen]
impl GeometryData {
    pub fn new(vertices: Vec<f32>, indices: Vec<u32>, normals: Vec<f32>, colors: Vec<f32>) -> Self {
        let vertex_count = vertices.len() / 3;
        Self {
            vertices,
            indices,
            normals,
            colors,
            uvs: vec![0.0; vertex_count * 2],    // Default UVs
            material_ids: vec![0; vertex_count], // Default to material 0
        }
    }

    pub fn new_with_uvs(
        vertices: Vec<f32>,
        indices: Vec<u32>,
        normals: Vec<f32>,
        colors: Vec<f32>,
        uvs: Vec<f32>,
        material_ids: Vec<u8>,
    ) -> Self {
        Self {
            vertices,
            indices,
            normals,
            colors,
            uvs,
            material_ids,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn vertices(&self) -> Vec<f32> {
        self.vertices.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn indices(&self) -> Vec<u32> {
        self.indices.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn normals(&self) -> Vec<f32> {
        self.normals.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn colors(&self) -> Vec<f32> {
        self.colors.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn uvs(&self) -> Vec<f32> {
        self.uvs.clone()
    }

    #[wasm_bindgen(getter, js_name = materialIds)]
    pub fn material_ids(&self) -> Vec<u8> {
        self.material_ids.clone()
    }
}

#[wasm_bindgen]
pub struct NetworkClient {
    // TODO: Implement network client state
}

#[wasm_bindgen]
impl NetworkClient {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn connect(
        &mut self,
        _server_url: String,
        _npub: String,
        _display_name: String,
        _avatar_url: Option<String>,
        _initial_x: f32,
        _initial_y: f32,
        _initial_z: f32,
    ) -> Result<(), JsValue> {
        tracing::info!("NetworkClient::connect called");
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_position(&self, _x: f32, _y: f32, _z: f32, _rx: f32, _ry: f32, _rz: f32, _rw: f32) {
        tracing::debug!("NetworkClient::send_position called");
    }

    pub async fn send_chat(&self, _message: String) -> Result<(), JsValue> {
        tracing::debug!("NetworkClient::send_chat called");
        Ok(())
    }
}

impl Default for NetworkClient {
    fn default() -> Self {
        Self::new()
    }
}
