mod avatar;
mod world_cube;

use avatar::AvatarManager;
use world_cube::WorldCube as WorldCubeInternal;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

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
    pub fn new(macro_depth: u32, micro_depth: u32, _border_depth: u32) -> Self {
        Self {
            inner: RefCell::new(WorldCubeInternal::new(
                macro_depth,
                micro_depth,
                _border_depth,
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
    pub fn set_voxel_at_depth(&self, x: i32, y: i32, z: i32, depth: u32, color_index: i32) {
        self.inner
            .borrow_mut()
            .set_voxel_at_depth(x, y, z, depth, color_index);
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
        match crossworld_cube::parse_csm(csm_code) {
            Ok(octree) => {
                self.inner.borrow_mut().set_root(octree.root);
                Ok(())
            }
            Err(e) => Err(JsValue::from_str(&format!("Parse error: {}", e))),
        }
    }
}

impl Default for WorldCube {
    fn default() -> Self {
        Self::new(3, 0, 0) // Default: macro depth 3, micro depth 0, no borders
    }
}

#[wasm_bindgen]
pub struct GeometryData {
    vertices: Vec<f32>,
    indices: Vec<u32>,
    normals: Vec<f32>,
    colors: Vec<f32>,
}

#[wasm_bindgen]
impl GeometryData {
    pub fn new(vertices: Vec<f32>, indices: Vec<u32>, normals: Vec<f32>, colors: Vec<f32>) -> Self {
        Self {
            vertices,
            indices,
            normals,
            colors,
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

#[wasm_bindgen]
pub struct AvatarEngine {
    manager: AvatarManager,
}

#[wasm_bindgen]
impl AvatarEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            manager: AvatarManager::new(),
        }
    }

    /// Generate avatar geometry for a specific user
    #[wasm_bindgen]
    pub fn generate_avatar(&mut self, user_npub: String) -> GeometryData {
        self.manager.generate_avatar_geometry(&user_npub)
    }

    /// Clear the avatar cache
    #[wasm_bindgen]
    pub fn clear_cache(&mut self) {
        self.manager.clear_cache();
    }
}

impl Default for AvatarEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl AvatarEngine {
    /// Get the number of cached avatars
    #[wasm_bindgen]
    pub fn cache_size(&self) -> usize {
        self.manager.cache_size()
    }

    /// Set voxel in the base avatar model
    #[wasm_bindgen]
    pub fn set_voxel(&mut self, x: u8, y: u8, z: u8, color_index: u8) {
        self.manager.set_voxel(x, y, z, color_index);
    }

    /// Remove voxel from the base avatar model
    #[wasm_bindgen]
    pub fn remove_voxel(&mut self, x: u8, y: u8, z: u8) {
        self.manager.remove_voxel(x, y, z);
    }

    /// Regenerate mesh for a user (after modifications)
    #[wasm_bindgen]
    pub fn regenerate_mesh(&mut self, user_npub: String) -> GeometryData {
        self.generate_avatar(user_npub)
    }
}

/// Load a .vox file from bytes and generate geometry
#[wasm_bindgen]
pub fn load_vox_from_bytes(
    bytes: &[u8],
    user_npub: Option<String>,
) -> Result<GeometryData, JsValue> {
    let voxel_model = avatar::load_vox_from_bytes(bytes)
        .map_err(|e| JsValue::from_str(&format!("Failed to load .vox file: {}", e)))?;

    // Apply user-specific color customization if npub provided
    let customized_palette = if let Some(npub) = user_npub {
        voxel_model.palette.customize_for_user(&npub)
    } else {
        voxel_model.palette.clone()
    };

    // Generate mesh from voxel model
    let mesher = avatar::VoxelMesher::new(&voxel_model);
    let (vertices, indices, normals, colors) = mesher.generate_mesh(&customized_palette);

    Ok(GeometryData::new(vertices, indices, normals, colors))
}
