//! Material system for renderer
//!
//! This module provides a minimal 6-color palette for testing purposes.
//! The full 128-material system is available in the `world` crate, but the
//! renderer uses this subset for standalone tests.
//!
//! ## Material Indices
//!
//! - 0: Empty (transparent, not rendered)
//! - 1: Red
//! - 2: Green
//! - 3: Blue
//! - 4: Yellow
//! - 5: White
//! - 6: Black
//!
//! Colors are returned as normalized RGB values in range [0.0, 1.0].

use glam::Vec3;

/// Material palette for testing (7 materials including empty)
///
/// Index 0 is reserved for empty/transparent voxels.
/// Indices 1-6 are primary test colors.
pub const MATERIAL_PALETTE: [Vec3; 7] = [
    Vec3::new(0.0, 0.0, 0.0), // 0: Empty (not used in rendering)
    Vec3::new(1.0, 0.0, 0.0), // 1: Red
    Vec3::new(0.0, 1.0, 0.0), // 2: Green
    Vec3::new(0.0, 0.0, 1.0), // 3: Blue
    Vec3::new(1.0, 1.0, 0.0), // 4: Yellow
    Vec3::new(1.0, 1.0, 1.0), // 5: White
    Vec3::new(0.0, 0.0, 0.0), // 6: Black
];

/// Get material color for a voxel value
///
/// # Arguments
///
/// * `value` - Voxel material index (0-6)
///
/// # Returns
///
/// Normalized RGB color as Vec3 in range [0.0, 1.0].
/// Returns black for invalid indices.
///
/// # Examples
///
/// ```
/// use renderer::materials::get_material_color;
/// use glam::Vec3;
///
/// let red = get_material_color(1);
/// assert_eq!(red, Vec3::new(1.0, 0.0, 0.0));
///
/// let green = get_material_color(2);
/// assert_eq!(green, Vec3::new(0.0, 1.0, 0.0));
/// ```
pub fn get_material_color(value: i32) -> Vec3 {
    if value < 0 || value >= MATERIAL_PALETTE.len() as i32 {
        // Invalid index, return black
        return Vec3::ZERO;
    }

    MATERIAL_PALETTE[value as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_palette_colors() {
        assert_eq!(get_material_color(0), Vec3::new(0.0, 0.0, 0.0)); // Empty
        assert_eq!(get_material_color(1), Vec3::new(1.0, 0.0, 0.0)); // Red
        assert_eq!(get_material_color(2), Vec3::new(0.0, 1.0, 0.0)); // Green
        assert_eq!(get_material_color(3), Vec3::new(0.0, 0.0, 1.0)); // Blue
        assert_eq!(get_material_color(4), Vec3::new(1.0, 1.0, 0.0)); // Yellow
        assert_eq!(get_material_color(5), Vec3::new(1.0, 1.0, 1.0)); // White
        assert_eq!(get_material_color(6), Vec3::new(0.0, 0.0, 0.0)); // Black
    }

    #[test]
    fn test_invalid_indices() {
        assert_eq!(get_material_color(-1), Vec3::ZERO);
        assert_eq!(get_material_color(7), Vec3::ZERO);
        assert_eq!(get_material_color(100), Vec3::ZERO);
    }

    #[test]
    fn test_palette_size() {
        assert_eq!(MATERIAL_PALETTE.len(), 7);
    }
}
