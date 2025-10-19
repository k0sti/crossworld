mod avatar;
mod emoji_hash;
mod geometry;

use avatar::AvatarManager;
use emoji_hash::pubkey_to_emoji_hash;
use geometry::GeometryEngine as GeometryEngineInternal;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
}

#[wasm_bindgen]
pub struct GeometryEngine {
    engine: GeometryEngineInternal,
}

#[wasm_bindgen]
impl GeometryEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        web_sys::console::log_1(&"GeometryEngine initialized".into());
        Self {
            engine: GeometryEngineInternal::new(),
        }
    }

    #[wasm_bindgen]
    pub fn generate_frame(&self) -> GeometryData {
        self.engine.generate_frame()
    }
}

impl Default for GeometryEngine {
    fn default() -> Self {
        Self::new()
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
}

/// Convert a hex pubkey to a 5-emoji hash for display
#[wasm_bindgen]
pub fn pubkey_to_emoji(pubkey_hex: String) -> String {
    pubkey_to_emoji_hash(&pubkey_hex)
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

// Helper function to convert body type string to enum
fn parse_body_type(body_type: &str) -> avatar::BodyType {
    match body_type {
        "slim" => avatar::BodyType::Slim,
        "bulky" => avatar::BodyType::Bulky,
        _ => avatar::BodyType::Normal,
    }
}

// Helper function to generate geometry from VoxelModel
fn voxel_model_to_geometry(model: avatar::VoxelModel, user_npub: Option<String>) -> GeometryData {
    let customized_palette = if let Some(npub) = user_npub {
        model.palette.customize_for_user(&npub)
    } else {
        model.palette.clone()
    };

    let mesher = avatar::VoxelMesher::new(&model);
    let (vertices, indices, normals, colors) = mesher.generate_mesh(&customized_palette);
    GeometryData::new(vertices, indices, normals, colors)
}

/// Generate a geometric sphere avatar
#[wasm_bindgen]
pub fn generate_sphere(size: u8, seed: String, user_npub: Option<String>) -> GeometryData {
    let model = avatar::generate_sphere(size, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a geometric cube avatar
#[wasm_bindgen]
pub fn generate_cube(size: u8, seed: String, user_npub: Option<String>) -> GeometryData {
    let model = avatar::generate_cube(size, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a geometric pyramid avatar
#[wasm_bindgen]
pub fn generate_pyramid(size: u8, seed: String, user_npub: Option<String>) -> GeometryData {
    let model = avatar::generate_pyramid(size, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a geometric torus avatar
#[wasm_bindgen]
pub fn generate_torus(size: u8, seed: String, user_npub: Option<String>) -> GeometryData {
    let model = avatar::generate_torus(size, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a geometric cylinder avatar
#[wasm_bindgen]
pub fn generate_cylinder(size: u8, seed: String, user_npub: Option<String>) -> GeometryData {
    let model = avatar::generate_cylinder(size, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a geometric diamond avatar
#[wasm_bindgen]
pub fn generate_diamond(size: u8, seed: String, user_npub: Option<String>) -> GeometryData {
    let model = avatar::generate_diamond(size, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a noise-based avatar
#[wasm_bindgen]
pub fn generate_noise(
    size: u8,
    seed: String,
    complexity: f32,
    user_npub: Option<String>,
) -> GeometryData {
    let model = avatar::generate_noise(size, &seed, complexity);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a humanoid warrior avatar
#[wasm_bindgen]
pub fn generate_warrior(
    size: u8,
    body_type: String,
    seed: String,
    user_npub: Option<String>,
) -> GeometryData {
    let body = parse_body_type(&body_type);
    let model = avatar::generate_warrior(size, body, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a humanoid peasant avatar
#[wasm_bindgen]
pub fn generate_peasant(
    size: u8,
    body_type: String,
    seed: String,
    user_npub: Option<String>,
) -> GeometryData {
    let body = parse_body_type(&body_type);
    let model = avatar::generate_peasant(size, body, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a humanoid mage avatar
#[wasm_bindgen]
pub fn generate_mage(
    size: u8,
    body_type: String,
    seed: String,
    user_npub: Option<String>,
) -> GeometryData {
    let body = parse_body_type(&body_type);
    let model = avatar::generate_mage(size, body, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a humanoid knight avatar
#[wasm_bindgen]
pub fn generate_knight(
    size: u8,
    body_type: String,
    seed: String,
    user_npub: Option<String>,
) -> GeometryData {
    let body = parse_body_type(&body_type);
    let model = avatar::generate_knight(size, body, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a humanoid archer avatar
#[wasm_bindgen]
pub fn generate_archer(
    size: u8,
    body_type: String,
    seed: String,
    user_npub: Option<String>,
) -> GeometryData {
    let body = parse_body_type(&body_type);
    let model = avatar::generate_archer(size, body, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a robot avatar
#[wasm_bindgen]
pub fn generate_robot(
    size: u8,
    body_type: String,
    seed: String,
    user_npub: Option<String>,
) -> GeometryData {
    let body = parse_body_type(&body_type);
    let model = avatar::generate_robot(size, body, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a cat avatar
#[wasm_bindgen]
pub fn generate_cat(
    size: u8,
    body_type: String,
    seed: String,
    user_npub: Option<String>,
) -> GeometryData {
    let body = parse_body_type(&body_type);
    let model = avatar::generate_cat(size, body, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a dog avatar
#[wasm_bindgen]
pub fn generate_dog(
    size: u8,
    body_type: String,
    seed: String,
    user_npub: Option<String>,
) -> GeometryData {
    let body = parse_body_type(&body_type);
    let model = avatar::generate_dog(size, body, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a bird avatar
#[wasm_bindgen]
pub fn generate_bird(
    size: u8,
    body_type: String,
    seed: String,
    user_npub: Option<String>,
) -> GeometryData {
    let body = parse_body_type(&body_type);
    let model = avatar::generate_bird(size, body, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a fish avatar
#[wasm_bindgen]
pub fn generate_fish(
    size: u8,
    body_type: String,
    seed: String,
    user_npub: Option<String>,
) -> GeometryData {
    let body = parse_body_type(&body_type);
    let model = avatar::generate_fish(size, body, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a dragon avatar
#[wasm_bindgen]
pub fn generate_dragon(
    size: u8,
    body_type: String,
    seed: String,
    user_npub: Option<String>,
) -> GeometryData {
    let body = parse_body_type(&body_type);
    let model = avatar::generate_dragon(size, body, &seed);
    voxel_model_to_geometry(model, user_npub)
}

/// Generate a bear avatar
#[wasm_bindgen]
pub fn generate_bear(
    size: u8,
    body_type: String,
    seed: String,
    user_npub: Option<String>,
) -> GeometryData {
    let body = parse_body_type(&body_type);
    let model = avatar::generate_bear(size, body, &seed);
    voxel_model_to_geometry(model, user_npub)
}
