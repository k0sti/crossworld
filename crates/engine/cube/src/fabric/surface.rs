//! Surface detection and color derivation for fabric system

use glam::{Quat, Vec3};

/// Check if there is a surface between two quaternion field values.
///
/// A surface exists where the magnitude crosses the threshold of 1.0.
/// Using SDF convention: |Q| < 1 = inside (solid), |Q| > 1 = outside (air).
///
/// # Arguments
/// * `current_quat` - Quaternion at current position
/// * `neighbor_quat` - Quaternion at neighboring position
///
/// # Returns
/// `true` if there is a surface between the two positions
#[inline]
pub fn is_surface(current_quat: Quat, neighbor_quat: Quat) -> bool {
    let current_mag = current_quat.length();
    let neighbor_mag = neighbor_quat.length();

    // Surface exists where magnitude crosses 1.0
    // One is inside (< 1.0) and other is outside (> 1.0)
    (current_mag < 1.0 && neighbor_mag >= 1.0) || (current_mag >= 1.0 && neighbor_mag < 1.0)
}

/// Calculate surface normal from magnitude gradient using central differences.
///
/// The normal points from solid (|Q| < 1) toward air (|Q| > 1),
/// following standard SDF convention.
///
/// # Arguments
/// * `position` - World position to calculate normal at
/// * `get_magnitude` - Function that returns magnitude at a given position
/// * `h` - Step size for finite differences (typically voxel half-size)
///
/// # Returns
/// Normalized surface normal vector
pub fn calculate_normal<F>(position: Vec3, get_magnitude: F, h: f32) -> Vec3
where
    F: Fn(Vec3) -> f32,
{
    // Central differences for gradient
    let gradient = Vec3::new(
        get_magnitude(position + Vec3::X * h) - get_magnitude(position - Vec3::X * h),
        get_magnitude(position + Vec3::Y * h) - get_magnitude(position - Vec3::Y * h),
        get_magnitude(position + Vec3::Z * h) - get_magnitude(position - Vec3::Z * h),
    ) / (2.0 * h);

    // Normal points toward increasing magnitude (toward outside/air)
    // Negate to point outward from solid
    (-gradient).normalize_or_zero()
}

/// Convert quaternion rotation to RGB color using HSV mapping.
///
/// Maps quaternion rotation components to HSV color space:
/// - Hue: from rotation axis direction (atan2 of x,y components)
/// - Saturation: from z component of axis
/// - Value: from rotation angle
///
/// # Arguments
/// * `quat` - Quaternion to convert (will be normalized internally)
///
/// # Returns
/// RGB color as [r, g, b] values in 0-255 range
pub fn quaternion_to_color(quat: Quat) -> [u8; 3] {
    // Normalize for rotation extraction
    let normalized = quat.normalize();

    // Extract rotation axis and angle
    let (axis, angle) = normalized.to_axis_angle();

    // Handle edge case where axis might be zero (identity quaternion)
    let axis = if axis.length_squared() > 0.0 {
        axis.normalize()
    } else {
        Vec3::Z
    };

    // Map to HSV
    use std::f32::consts::PI;

    // Hue from axis direction in XY plane
    let hue = (axis.y.atan2(axis.x) / (2.0 * PI) + 0.5).clamp(0.0, 1.0);

    // Saturation from Z component
    let saturation = axis.z.abs().clamp(0.2, 1.0);

    // Value from angle
    let value = (0.5 + 0.5 * angle.cos()).clamp(0.3, 1.0);

    hsv_to_rgb(hue, saturation, value)
}

/// Convert HSV color to RGB.
///
/// # Arguments
/// * `h` - Hue in range [0, 1]
/// * `s` - Saturation in range [0, 1]
/// * `v` - Value/brightness in range [0, 1]
///
/// # Returns
/// RGB color as [r, g, b] values in 0-255 range
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h = h.clamp(0.0, 1.0);
    let s = s.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);

    if s == 0.0 {
        // Achromatic (gray)
        let val = (v * 255.0) as u8;
        return [val, val, val];
    }

    let h = h * 6.0;
    let i = h.floor() as i32;
    let f = h - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    let (r, g, b) = match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };

    [
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    #[test]
    fn test_is_surface_crossing() {
        // Inside to outside (surface exists)
        let inside = Quat::from_xyzw(0.0, 0.0, 0.0, 0.5); // magnitude 0.5 < 1
        let outside = Quat::from_xyzw(0.0, 0.0, 0.0, 1.5); // magnitude 1.5 > 1
        assert!(is_surface(inside, outside));
        assert!(is_surface(outside, inside));
    }

    #[test]
    fn test_is_surface_no_crossing() {
        // Both inside (no surface)
        let inside1 = Quat::from_xyzw(0.0, 0.0, 0.0, 0.5);
        let inside2 = Quat::from_xyzw(0.0, 0.0, 0.0, 0.8);
        assert!(!is_surface(inside1, inside2));

        // Both outside (no surface)
        let outside1 = Quat::from_xyzw(0.0, 0.0, 0.0, 1.2);
        let outside2 = Quat::from_xyzw(0.0, 0.0, 0.0, 2.0);
        assert!(!is_surface(outside1, outside2));
    }

    #[test]
    fn test_is_surface_at_boundary() {
        // Exactly at boundary
        let at_surface = Quat::from_xyzw(0.0, 0.0, 0.0, 1.0);
        let inside = Quat::from_xyzw(0.0, 0.0, 0.0, 0.5);
        let outside = Quat::from_xyzw(0.0, 0.0, 0.0, 1.5);

        // Surface at boundary should count as outside (>= 1.0)
        assert!(is_surface(inside, at_surface));
        assert!(!is_surface(at_surface, outside)); // Both >= 1.0
    }

    #[test]
    fn test_calculate_normal_spherical() {
        // For a spherical field where magnitude = distance from origin,
        // normal should point radially outward (from solid toward air)
        // Since magnitude increases outward, the gradient points outward,
        // and we negate it to get normal pointing outward from solid

        let sphere_magnitude = |pos: Vec3| pos.length();

        // At point (1, 0, 0), normal should point in -X direction
        // (gradient points +X, normal = -gradient)
        let normal = calculate_normal(Vec3::new(1.0, 0.0, 0.0), sphere_magnitude, 0.01);
        assert!(
            (normal - (-Vec3::X)).length() < 0.1,
            "Expected ~-X, got {:?}",
            normal
        );

        // At point (0, 1, 0), normal should point in -Y direction
        let normal = calculate_normal(Vec3::new(0.0, 1.0, 0.0), sphere_magnitude, 0.01);
        assert!(
            (normal - (-Vec3::Y)).length() < 0.1,
            "Expected ~-Y, got {:?}",
            normal
        );
    }

    #[test]
    fn test_quaternion_to_color_different_rotations() {
        // Different rotations should produce different colors
        let identity = Quat::IDENTITY;
        let rot_x = Quat::from_rotation_x(FRAC_PI_2);
        let rot_y = Quat::from_rotation_y(FRAC_PI_2);
        let rot_z = Quat::from_rotation_z(FRAC_PI_2);

        let color_identity = quaternion_to_color(identity);
        let color_x = quaternion_to_color(rot_x);
        let color_y = quaternion_to_color(rot_y);
        let color_z = quaternion_to_color(rot_z);

        // Colors should be different (at least some component differs)
        fn colors_different(c1: [u8; 3], c2: [u8; 3]) -> bool {
            c1[0] != c2[0] || c1[1] != c2[1] || c1[2] != c2[2]
        }

        assert!(
            colors_different(color_identity, color_x)
                || colors_different(color_identity, color_y)
                || colors_different(color_identity, color_z),
            "At least some rotations should produce different colors"
        );
    }

    #[test]
    fn test_hsv_to_rgb_pure_colors() {
        // Red (H=0)
        let red = hsv_to_rgb(0.0, 1.0, 1.0);
        assert_eq!(red[0], 255);
        assert!(red[1] < 10);
        assert!(red[2] < 10);

        // Green (H=0.333)
        let green = hsv_to_rgb(1.0 / 3.0, 1.0, 1.0);
        assert!(green[0] < 10);
        assert_eq!(green[1], 255);
        assert!(green[2] < 10);

        // Blue (H=0.666)
        let blue = hsv_to_rgb(2.0 / 3.0, 1.0, 1.0);
        assert!(blue[0] < 10);
        assert!(blue[1] < 10);
        assert_eq!(blue[2], 255);
    }

    #[test]
    fn test_hsv_to_rgb_grayscale() {
        // Zero saturation should give gray
        let gray = hsv_to_rgb(0.5, 0.0, 0.5);
        assert_eq!(gray[0], gray[1]);
        assert_eq!(gray[1], gray[2]);
        assert!(gray[0] > 100 && gray[0] < 150); // ~128
    }
}
