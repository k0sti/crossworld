/// Native (non-WASM) utilities for physics integration
///
/// This module provides Bevy-specific physics utilities that are only available
/// in native builds (not compiled to WASM).

#[cfg(feature = "bevy")]
pub use bevy_integration::*;

#[cfg(feature = "bevy")]
mod bevy_integration {
    use super::*;

    // Bevy-specific utilities will be added here as needed
    // For example: Bevy component conversion, resource helpers, etc.
}

/// Re-export Aabb for convenience
pub use rapier3d::parry::bounding_volume::Aabb;
