use cube::{fabric::FabricConfig, Cube};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::sync::OnceLock;

static RENDERER_CONFIG: OnceLock<RendererConfig> = OnceLock::new();

/// Configuration for a single test model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    /// Short identifier for CLI (e.g., "octa")
    pub id: String,
    /// Display name for UI
    pub name: String,
    /// CubeScript Model format string (if CSM-based model)
    #[serde(default)]
    pub csm: Option<String>,
    /// Path to .vox file (if VOX-based model)
    #[serde(default)]
    pub vox_path: Option<String>,
}

/// Base path for VOX model files (relative to workspace root)
const VOX_MODELS_BASE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/models/vox/");

impl ModelEntry {
    /// Create a cube from this model entry
    pub fn create_cube(&self) -> Result<Rc<Cube<u8>>, String> {
        if let Some(csm) = &self.csm {
            // Parse CSM string
            cube::parse_csm(csm)
                .map(Rc::new)
                .map_err(|e| format!("Failed to parse CSM: {}", e))
        } else if let Some(vox_path) = &self.vox_path {
            // Load VOX file from assets/models/vox/ directory
            let full_path = format!("{}{}", VOX_MODELS_BASE_PATH, vox_path);
            let bytes = fs::read(&full_path)
                .map_err(|e| format!("Failed to read VOX file '{}': {}", full_path, e))?;
            // Load VOX file and extract the cube from the CubeBox
            cube::load_vox_to_cubebox(&bytes)
                .map(|cubebox| Rc::new(cubebox.cube))
                .map_err(|e| format!("Failed to load VOX: {}", e))
        } else {
            Err("Model has neither csm nor vox_path".to_string())
        }
    }
}

/// Configuration for the single cube model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleCubeConfig {
    /// Default material ID for single cube
    #[serde(default = "default_material")]
    pub default_material: u8,
}

fn default_material() -> u8 {
    224
}

impl Default for SingleCubeConfig {
    fn default() -> Self {
        Self {
            default_material: default_material(),
        }
    }
}

/// Configuration for rendering parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderingConfig {
    /// Default resolution [width, height]
    #[serde(default = "default_resolution")]
    pub default_resolution: [u32; 2],
}

fn default_resolution() -> [u32; 2] {
    [400, 300]
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            default_resolution: default_resolution(),
        }
    }
}

/// Legacy ModelsConfig for backwards compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsConfig {
    pub models: Vec<ModelEntry>,
}

/// Root configuration structure (unified config)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererConfig {
    /// Model entries
    pub models: Vec<ModelEntry>,

    /// Single cube configuration
    #[serde(default)]
    pub single_cube: SingleCubeConfig,

    /// Fabric generation configuration
    #[serde(default)]
    pub fabric: FabricConfig,

    /// Rendering parameters
    #[serde(default)]
    pub rendering: RenderingConfig,
}

impl RendererConfig {
    /// Load configuration from a RON file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: RendererConfig = ron::from_str(&content)?;
        Ok(config)
    }

    /// Get a model by its ID
    pub fn get_model(&self, id: &str) -> Option<&ModelEntry> {
        self.models.iter().find(|m| m.id == id)
    }

    /// Get all model IDs
    pub fn model_ids(&self) -> Vec<&str> {
        self.models.iter().map(|m| m.id.as_str()).collect()
    }
}

impl ModelsConfig {
    /// Load models configuration from a RON file (legacy support)
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: ModelsConfig = ron::from_str(&content)?;
        Ok(config)
    }

    /// Get a model by its ID
    pub fn get_model(&self, id: &str) -> Option<&ModelEntry> {
        self.models.iter().find(|m| m.id == id)
    }

    /// Get all model IDs
    pub fn model_ids(&self) -> Vec<&str> {
        self.models.iter().map(|m| m.id.as_str()).collect()
    }
}

/// Get the global renderer configuration (loads on first access)
pub fn get_renderer_config() -> &'static RendererConfig {
    RENDERER_CONFIG.get_or_init(|| {
        let config_path = concat!(env!("CARGO_MANIFEST_DIR"), "/config.ron");
        RendererConfig::load_from_file(config_path)
            .expect("Failed to load config.ron")
    })
}

/// Get the global models configuration (loads on first access) - legacy compatibility
pub fn get_models_config() -> &'static RendererConfig {
    get_renderer_config()
}

/// Create a cube from a model ID
pub fn create_cube_from_id(id: &str) -> Result<Rc<Cube<u8>>, String> {
    let config = get_renderer_config();
    let model = config.get_model(id)
        .ok_or_else(|| format!("Model '{}' not found", id))?;
    model.create_cube()
}

/// Get all available model IDs
pub fn get_model_ids() -> Vec<&'static str> {
    get_renderer_config().model_ids()
}

/// Get a model entry by ID
pub fn get_model(id: &str) -> Option<&'static ModelEntry> {
    get_renderer_config().get_model(id)
}

/// Get the fabric configuration
pub fn get_fabric_config() -> &'static FabricConfig {
    &get_renderer_config().fabric
}

/// Get the single cube configuration
pub fn get_single_cube_config() -> &'static SingleCubeConfig {
    &get_renderer_config().single_cube
}

/// Get the rendering configuration
pub fn get_rendering_config() -> &'static RenderingConfig {
    &get_renderer_config().rendering
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_renderer_config() {
        let config_path = concat!(env!("CARGO_MANIFEST_DIR"), "/config.ron");
        let config = RendererConfig::load_from_file(config_path);
        assert!(config.is_ok(), "Failed to load config.ron: {:?}", config.err());

        let config = config.unwrap();
        assert!(!config.models.is_empty(), "No models loaded");

        // Check that required models exist
        assert!(config.get_model("octa").is_some(), "Missing 'octa' model");
        assert!(config.get_model("single").is_some(), "Missing 'single' model");

        // Check fabric config has defaults
        assert!(config.fabric.max_depth > 0, "Fabric max_depth should be positive");

        // Check single_cube config
        assert!(config.single_cube.default_material > 0, "Single cube should have a material");
    }
}
