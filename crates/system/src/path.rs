//! Cross-platform path resolution
//!
//! This module provides utilities for resolving paths across different
//! platforms (Native, Web/WASM) with support for common directories like
//! assets, configuration, and user data.

use std::path::{Path, PathBuf};

use crate::Platform;

/// Path resolver for cross-platform file access
///
/// Resolves paths based on the current platform and provides
/// access to standard directories (assets, config, user data, etc.)
#[derive(Debug, Clone)]
pub struct PathResolver {
    /// Base path for the application (typically the executable's directory)
    base_path: PathBuf,
    /// Platform-specific configuration
    platform: Platform,
}

impl Default for PathResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl PathResolver {
    /// Create a new path resolver with automatic base path detection
    pub fn new() -> Self {
        Self {
            base_path: Self::detect_base_path(),
            platform: Platform::current(),
        }
    }

    /// Create a path resolver with a custom base path
    pub fn with_base_path<P: Into<PathBuf>>(base_path: P) -> Self {
        Self {
            base_path: base_path.into(),
            platform: Platform::current(),
        }
    }

    /// Detect the base path for the application
    #[cfg(not(target_arch = "wasm32"))]
    fn detect_base_path() -> PathBuf {
        // Try to get the executable's directory first
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                return parent.to_path_buf();
            }
        }
        // Fall back to current working directory
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    /// Detect the base path for web platforms (virtual path)
    #[cfg(target_arch = "wasm32")]
    fn detect_base_path() -> PathBuf {
        // On web, we use a virtual root path
        PathBuf::from("/")
    }

    /// Get the base path
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Get the current platform
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// Resolve a relative path against the base path
    pub fn resolve<P: AsRef<Path>>(&self, relative: P) -> PathBuf {
        self.base_path.join(relative)
    }

    /// Get the assets directory path
    ///
    /// On native: `{base_path}/assets`
    /// On web: `/assets`
    pub fn assets_dir(&self) -> PathBuf {
        self.resolve("assets")
    }

    /// Resolve a path within the assets directory
    pub fn asset<P: AsRef<Path>>(&self, relative: P) -> PathBuf {
        self.assets_dir().join(relative)
    }

    /// Get the configuration directory path
    ///
    /// On native: Uses platform-specific config directory (XDG, AppData, etc.)
    /// On web: `/config` (virtual)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn config_dir(&self) -> PathBuf {
        dirs::config_dir()
            .map(|p| p.join("crossworld"))
            .unwrap_or_else(|| self.resolve("config"))
    }

    #[cfg(target_arch = "wasm32")]
    pub fn config_dir(&self) -> PathBuf {
        self.resolve("config")
    }

    /// Resolve a path within the config directory
    pub fn config<P: AsRef<Path>>(&self, relative: P) -> PathBuf {
        self.config_dir().join(relative)
    }

    /// Get the user data directory path
    ///
    /// On native: Uses platform-specific data directory
    /// On web: `/data` (virtual, likely stored in IndexedDB)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn data_dir(&self) -> PathBuf {
        dirs::data_dir()
            .map(|p| p.join("crossworld"))
            .unwrap_or_else(|| self.resolve("data"))
    }

    #[cfg(target_arch = "wasm32")]
    pub fn data_dir(&self) -> PathBuf {
        self.resolve("data")
    }

    /// Resolve a path within the data directory
    pub fn data<P: AsRef<Path>>(&self, relative: P) -> PathBuf {
        self.data_dir().join(relative)
    }

    /// Get the cache directory path
    ///
    /// On native: Uses platform-specific cache directory
    /// On web: `/cache` (virtual)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn cache_dir(&self) -> PathBuf {
        dirs::cache_dir()
            .map(|p| p.join("crossworld"))
            .unwrap_or_else(|| self.resolve("cache"))
    }

    #[cfg(target_arch = "wasm32")]
    pub fn cache_dir(&self) -> PathBuf {
        self.resolve("cache")
    }

    /// Resolve a path within the cache directory
    pub fn cache<P: AsRef<Path>>(&self, relative: P) -> PathBuf {
        self.cache_dir().join(relative)
    }

    /// Get the models directory path
    pub fn models_dir(&self) -> PathBuf {
        self.asset("models")
    }

    /// Get the VOX models directory path
    pub fn vox_models_dir(&self) -> PathBuf {
        self.asset("models/vox")
    }

    /// Get the textures directory path
    pub fn textures_dir(&self) -> PathBuf {
        self.asset("textures")
    }

    /// Get the shaders directory path
    pub fn shaders_dir(&self) -> PathBuf {
        self.asset("shaders")
    }

    /// Get the scripts directory path (for Lua/KDL scripts)
    pub fn scripts_dir(&self) -> PathBuf {
        self.resolve("scripts")
    }

    /// Check if a path exists (native only)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn exists<P: AsRef<Path>>(&self, path: P) -> bool {
        path.as_ref().exists()
    }

    /// Check if a path exists (web - always returns false, use async fetch)
    #[cfg(target_arch = "wasm32")]
    pub fn exists<P: AsRef<Path>>(&self, _path: P) -> bool {
        // On web, we can't synchronously check if a file exists
        // Use async fetch operations instead
        false
    }

    /// Ensure a directory exists (native only)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn ensure_dir<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    /// Ensure a directory exists (web - no-op)
    #[cfg(target_arch = "wasm32")]
    pub fn ensure_dir<P: AsRef<Path>>(&self, _path: P) -> std::io::Result<()> {
        // On web, directories are virtual
        Ok(())
    }
}

/// Convert a path to a URL for web asset loading
///
/// On native, returns the file:// URL
/// On web, returns the relative URL path
pub fn path_to_url<P: AsRef<Path>>(path: P) -> String {
    let path = path.as_ref();

    #[cfg(not(target_arch = "wasm32"))]
    {
        format!("file://{}", path.display())
    }

    #[cfg(target_arch = "wasm32")]
    {
        // Convert to web-friendly path (forward slashes, URL encoding)
        path.to_string_lossy()
            .replace('\\', "/")
            .trim_start_matches('/')
            .to_string()
    }
}

/// Normalize a path for the current platform
pub fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();

    #[cfg(not(target_arch = "wasm32"))]
    {
        // On native, canonicalize if possible
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    #[cfg(target_arch = "wasm32")]
    {
        // On web, just normalize separators
        PathBuf::from(path.to_string_lossy().replace('\\', "/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_resolver_creation() {
        let resolver = PathResolver::new();
        // Should not be empty
        assert!(!resolver.base_path().as_os_str().is_empty());
    }

    #[test]
    fn test_path_resolver_with_base() {
        let resolver = PathResolver::with_base_path("/custom/base");
        assert_eq!(resolver.base_path(), Path::new("/custom/base"));
    }

    #[test]
    fn test_resolve() {
        let resolver = PathResolver::with_base_path("/app");
        let resolved = resolver.resolve("assets/test.txt");
        assert_eq!(resolved, PathBuf::from("/app/assets/test.txt"));
    }

    #[test]
    fn test_asset_path() {
        let resolver = PathResolver::with_base_path("/app");
        let asset = resolver.asset("models/player.vox");
        assert_eq!(asset, PathBuf::from("/app/assets/models/player.vox"));
    }

    #[test]
    fn test_standard_directories() {
        let resolver = PathResolver::with_base_path("/app");
        assert!(resolver.assets_dir().ends_with("assets"));
        assert!(resolver.models_dir().ends_with("models"));
        assert!(resolver.vox_models_dir().ends_with("vox"));
    }

    #[test]
    fn test_path_to_url() {
        let url = path_to_url("/assets/model.vox");
        #[cfg(not(target_arch = "wasm32"))]
        assert!(url.starts_with("file://"));
        #[cfg(target_arch = "wasm32")]
        assert!(!url.contains("file://"));
    }
}
