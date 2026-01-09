//! Editor configuration management
//!
//! Handles loading and saving editor configuration including:
//! - Last opened model path
//! - Window state
//! - Editor preferences

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Editor configuration stored in a config file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    /// Path to the last opened model file
    pub last_model_path: Option<PathBuf>,
    /// Maximum editing depth (default: 4 = 16x16x16)
    pub max_depth: u32,
    /// Camera distance on startup
    pub camera_distance: f32,
    /// Grid visibility
    pub show_grid: bool,
    /// Axis helper visibility
    pub show_axes: bool,
    /// Keep model data in memory after loading (uses more RAM)
    #[serde(default = "default_keep_models_in_memory")]
    pub keep_models_in_memory: bool,
}

fn default_keep_models_in_memory() -> bool {
    false
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            last_model_path: None,
            max_depth: 4,
            camera_distance: 10.0,
            show_grid: true,
            show_axes: true,
            keep_models_in_memory: false,
        }
    }
}

impl EditorConfig {
    /// Get the config file path
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("crossworld").join("editor.toml"))
    }

    /// Load config from file, or return default if not found
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };

        if !path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save config to file
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("Could not determine config directory")?;

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let content =
            toml::to_string_pretty(self).map_err(|e| format!("Failed to serialize config: {}", e))?;

        std::fs::write(&path, content).map_err(|e| format!("Failed to write config: {}", e))?;

        Ok(())
    }

    /// Update last model path and save
    pub fn set_last_model_path(&mut self, path: Option<PathBuf>) {
        self.last_model_path = path;
        if let Err(e) = self.save() {
            eprintln!("[Editor] Failed to save config: {}", e);
        }
    }

    /// Get the cube size (2^max_depth)
    pub fn cube_size(&self) -> u32 {
        1 << self.max_depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EditorConfig::default();
        assert!(config.last_model_path.is_none());
        assert_eq!(config.max_depth, 4);
        assert!(config.show_grid);
        assert!(config.show_axes);
    }

    #[test]
    fn test_cube_size() {
        let config = EditorConfig::default();
        assert_eq!(config.cube_size(), 16); // 2^4 = 16
    }
}
