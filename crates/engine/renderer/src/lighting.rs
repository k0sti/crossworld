//! Lighting constants for standardized rendering
//!
//! These constants define a simple directional lighting model with ambient
//! and diffuse components. All tracers (CPU, GL, GPU) use these same values
//! to ensure consistent visual output.

use glam::Vec3;

/// Directional light direction (normalized)
///
/// Light comes from upper-right-front direction.
/// Pre-normalized: normalize(0.5, 1.0, 0.3) = (0.431934, 0.863868, 0.259161)
pub const LIGHT_DIR: Vec3 = Vec3::new(0.431934, 0.863868, 0.259161);

/// Ambient lighting term (0.0-1.0)
///
/// 30% ambient illumination ensures all surfaces are visible even when
/// facing away from the light.
pub const AMBIENT: f32 = 0.3;

/// Diffuse lighting strength multiplier
///
/// Applied to the diffuse term before adding to ambient.
pub const DIFFUSE_STRENGTH: f32 = 0.7;

/// Background color for empty space
///
/// Bluish-gray color rendered when rays miss all voxels.
pub const BACKGROUND_COLOR: Vec3 = Vec3::new(0.4, 0.5, 0.6);
