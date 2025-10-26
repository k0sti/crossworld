mod avatar;
mod geometry;

use avatar::AvatarManager;
use geometry::GeometryEngine as GeometryEngineInternal;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
}

#[wasm_bindgen]
pub struct GeometryEngine {
    engine: RefCell<GeometryEngineInternal>,
}

#[wasm_bindgen]
impl GeometryEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(world_depth: u32, scale_depth: u32) -> Self {
        web_sys::console::log_1(&format!("GeometryEngine initialized with world_depth={}, scale_depth={}", world_depth, scale_depth).into());
        Self {
            engine: RefCell::new(GeometryEngineInternal::new(world_depth, scale_depth)),
        }
    }

    #[wasm_bindgen]
    pub fn generate_frame(&self) -> GeometryData {
        self.engine.borrow().generate_frame()
    }

    /// Set voxel in cube ground at specified depth
    /// depth: octree depth (7=finest detail, 4=coarse, etc.)
    #[wasm_bindgen(js_name = setVoxelAtDepth)]
    pub fn set_voxel_at_depth(&self, x: i32, y: i32, z: i32, depth: u32, color_index: i32) {
        self.engine.borrow_mut().set_voxel_at_depth(x, y, z, depth, color_index);
    }

    /// Set single voxel in cube ground
    #[wasm_bindgen(js_name = setVoxel)]
    pub fn set_voxel(&self, x: i32, y: i32, z: i32, color_index: i32) {
        self.engine.borrow_mut().set_voxel(x, y, z, color_index);
    }

    /// Remove voxel from cube ground at specified depth
    #[wasm_bindgen(js_name = removeVoxelAtDepth)]
    pub fn remove_voxel_at_depth(&self, x: i32, y: i32, z: i32, depth: u32) {
        self.engine.borrow_mut().remove_voxel_at_depth(x, y, z, depth);
    }

    /// Remove voxel from cube ground
    #[wasm_bindgen(js_name = removeVoxel)]
    pub fn remove_voxel(&self, x: i32, y: i32, z: i32) {
        self.engine.borrow_mut().remove_voxel(x, y, z);
    }
}

impl Default for GeometryEngine {
    fn default() -> Self {
        Self::new(4, 1) // Default: depth 4 (macro=3, micro=1), scale 1
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
        web_sys::console::log_1(&"AvatarEngine initialized".into());
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
