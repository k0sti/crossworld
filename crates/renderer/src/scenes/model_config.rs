use cube::Cube;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::sync::OnceLock;

static MODELS_CONFIG: OnceLock<ModelsConfig> = OnceLock::new();

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

impl ModelEntry {
    /// Create a cube from this model entry
    pub fn create_cube(&self) -> Result<Rc<Cube<u8>>, String> {
        if let Some(csm) = &self.csm {
            // Parse CSM string
            cube::parse_csm(csm)
                .map(Rc::new)
                .map_err(|e| format!("Failed to parse CSM: {}", e))
        } else if let Some(vox_path) = &self.vox_path {
            // Load VOX file
            let bytes = fs::read(vox_path)
                .map_err(|e| format!("Failed to read VOX file '{}': {}", vox_path, e))?;
            cube::load_vox_to_cube(&bytes, cube::glam::Vec3::ZERO)
                .map(Rc::new)
                .map_err(|e| format!("Failed to load VOX: {}", e))
        } else {
            Err("Model has neither csm nor vox_path".to_string())
        }
    }
}

/// Root configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsConfig {
    pub models: Vec<ModelEntry>,
}

impl ModelsConfig {
    /// Load models configuration from a RON file
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

/// Get the global models configuration (loads on first access)
pub fn get_models_config() -> &'static ModelsConfig {
    MODELS_CONFIG.get_or_init(|| {
        let config_path = concat!(env!("CARGO_MANIFEST_DIR"), "/models.ron");
        ModelsConfig::load_from_file(config_path)
            .expect("Failed to load models.ron")
    })
}

/// Create a cube from a model ID
pub fn create_cube_from_id(id: &str) -> Result<Rc<Cube<u8>>, String> {
    let config = get_models_config();
    let model = config.get_model(id)
        .ok_or_else(|| format!("Model '{}' not found", id))?;
    model.create_cube()
}

/// Get all available model IDs
pub fn get_model_ids() -> Vec<&'static str> {
    get_models_config().model_ids()
}

/// Get a model entry by ID
pub fn get_model(id: &str) -> Option<&'static ModelEntry> {
    get_models_config().get_model(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_models_config() {
        let config_path = concat!(env!("CARGO_MANIFEST_DIR"), "/models.ron");
        let config = ModelsConfig::load_from_file(config_path);
        assert!(config.is_ok(), "Failed to load models.ron: {:?}", config.err());

        let config = config.unwrap();
        assert!(!config.models.is_empty(), "No models loaded");

        // Check that required models exist
        assert!(config.get_model("octa").is_some(), "Missing 'octa' model");
        assert!(config.get_model("single").is_some(), "Missing 'single' model");
    }
}
