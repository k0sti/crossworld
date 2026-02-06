//! Platform detection and abstraction
//!
//! This module provides platform-specific functionality and detection for
//! cross-platform support (Native, Web/WASM).

use serde::{Deserialize, Serialize};

/// Target platform for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    /// Native desktop platform (Windows, macOS, Linux)
    Native,
    /// Web browser via WebAssembly
    Web,
}

impl Default for Platform {
    fn default() -> Self {
        Self::current()
    }
}

impl Platform {
    /// Detect the current platform at compile time
    #[inline]
    pub const fn current() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            Self::Web
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::Native
        }
    }

    /// Check if running on native platform
    #[inline]
    pub const fn is_native(&self) -> bool {
        matches!(self, Self::Native)
    }

    /// Check if running on web/WASM platform
    #[inline]
    pub const fn is_web(&self) -> bool {
        matches!(self, Self::Web)
    }

    /// Get the platform name as a string
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Native => "Native",
            Self::Web => "Web",
        }
    }

    /// Get detailed OS information (native only)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn os_name() -> &'static str {
        #[cfg(target_os = "windows")]
        {
            "Windows"
        }
        #[cfg(target_os = "macos")]
        {
            "macOS"
        }
        #[cfg(target_os = "linux")]
        {
            "Linux"
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            "Unknown"
        }
    }

    /// Get detailed OS information (web - returns browser info placeholder)
    #[cfg(target_arch = "wasm32")]
    pub fn os_name() -> &'static str {
        "Browser"
    }

    /// Get the target architecture
    pub const fn arch() -> &'static str {
        #[cfg(target_arch = "x86_64")]
        {
            "x86_64"
        }
        #[cfg(target_arch = "x86")]
        {
            "x86"
        }
        #[cfg(target_arch = "aarch64")]
        {
            "aarch64"
        }
        #[cfg(target_arch = "arm")]
        {
            "arm"
        }
        #[cfg(target_arch = "wasm32")]
        {
            "wasm32"
        }
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "x86",
            target_arch = "aarch64",
            target_arch = "arm",
            target_arch = "wasm32"
        )))]
        {
            "unknown"
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Feature detection for platform capabilities
#[derive(Debug, Clone, Copy, Default)]
pub struct PlatformCapabilities {
    /// Whether hardware-accelerated rendering is available
    pub hardware_rendering: bool,
    /// Whether multi-threading is supported
    pub multi_threading: bool,
    /// Whether filesystem access is available
    pub filesystem_access: bool,
    /// Whether network access is available
    pub network_access: bool,
    /// Whether gamepad/controller input is supported
    pub gamepad_support: bool,
    /// Whether audio output is supported
    pub audio_support: bool,
}

impl PlatformCapabilities {
    /// Detect capabilities for the current platform
    pub fn detect() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            Self {
                hardware_rendering: true, // WebGL
                multi_threading: false,   // Web Workers have limitations
                filesystem_access: false, // No direct filesystem access
                network_access: true,     // Fetch API
                gamepad_support: true,    // Gamepad API
                audio_support: true,      // Web Audio API
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {
                hardware_rendering: true,
                multi_threading: true,
                filesystem_access: true,
                network_access: true,
                gamepad_support: cfg!(feature = "gilrs"),
                audio_support: true,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = Platform::current();
        #[cfg(target_arch = "wasm32")]
        assert_eq!(platform, Platform::Web);
        #[cfg(not(target_arch = "wasm32"))]
        assert_eq!(platform, Platform::Native);
    }

    #[test]
    fn test_platform_is_methods() {
        let native = Platform::Native;
        let web = Platform::Web;

        assert!(native.is_native());
        assert!(!native.is_web());
        assert!(!web.is_native());
        assert!(web.is_web());
    }

    #[test]
    fn test_platform_display() {
        assert_eq!(Platform::Native.to_string(), "Native");
        assert_eq!(Platform::Web.to_string(), "Web");
    }

    #[test]
    fn test_capabilities() {
        let caps = PlatformCapabilities::detect();
        // Basic sanity checks
        assert!(caps.hardware_rendering);
    }
}
