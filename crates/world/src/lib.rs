mod geometry;

use wasm_bindgen::prelude::*;
use geometry::GeometryEngine as GeometryEngineInternal;

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

    pub fn send_position(
        &self,
        _x: f32,
        _y: f32,
        _z: f32,
        _rx: f32,
        _ry: f32,
        _rz: f32,
        _rw: f32,
    ) {
        tracing::debug!("NetworkClient::send_position called");
    }

    pub async fn send_chat(&self, _message: String) -> Result<(), JsValue> {
        tracing::debug!("NetworkClient::send_chat called");
        Ok(())
    }
}
