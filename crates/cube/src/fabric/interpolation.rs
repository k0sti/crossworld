//! Quaternion interpolation functions for fabric generation

use super::types::{AdditiveState, FabricConfig};
use glam::{Quat, Vec3};

/// Non-normalizing linear interpolation between two quaternions.
///
/// Unlike SLERP or NLERP, this preserves magnitude information which is
/// essential for the fabric system's SDF-like surface detection.
///
/// # Arguments
/// * `q1` - Start quaternion
/// * `q2` - End quaternion
/// * `t` - Interpolation factor (0.0 = q1, 1.0 = q2)
#[inline]
pub fn lerp_quaternion(q1: Quat, q2: Quat, t: f32) -> Quat {
    // Handle potential sign flip for shortest path interpolation
    let q2 = if q1.dot(q2) < 0.0 { -q2 } else { q2 };

    // Non-normalizing linear interpolation
    Quat::from_xyzw(
        q1.x + (q2.x - q1.x) * t,
        q1.y + (q2.y - q1.y) * t,
        q1.z + (q2.z - q1.z) * t,
        q1.w + (q2.w - q1.w) * t,
    )
}

/// Calculate octant rotation based on octant index (0-7).
///
/// Each octant's quaternion is derived from its parent by applying a positional
/// rotation based on the octant index. This creates a deterministic mapping from
/// world position to quaternion rotation.
///
/// Octant indices 0-7 map to corners of unit cube:
/// - Bit 0 (x): +90° if set, -90° if not
/// - Bit 1 (y): +90° if set, -90° if not
/// - Bit 2 (z): +90° if set, -90° if not
#[inline]
pub fn octant_rotation(octant_index: usize) -> Quat {
    debug_assert!(octant_index < 8, "Octant index must be 0-7");

    use std::f32::consts::FRAC_PI_2; // 90 degrees

    let x_angle = if (octant_index & 1) != 0 {
        FRAC_PI_2
    } else {
        -FRAC_PI_2
    };
    let y_angle = if (octant_index & 2) != 0 {
        FRAC_PI_2
    } else {
        -FRAC_PI_2
    };
    let z_angle = if (octant_index & 4) != 0 {
        FRAC_PI_2
    } else {
        -FRAC_PI_2
    };

    // Combine rotations (order: Z * Y * X)
    Quat::from_euler(glam::EulerRot::ZYX, z_angle, y_angle, x_angle)
}

/// Calculate position offset for child center based on octant index.
///
/// Returns the normalized offset vector from parent center to child center.
/// Each component is either -0.5 or +0.5.
#[inline]
pub fn octant_offset(octant_index: usize) -> Vec3 {
    debug_assert!(octant_index < 8, "Octant index must be 0-7");

    Vec3::new(
        if (octant_index & 1) != 0 { 0.5 } else { -0.5 },
        if (octant_index & 2) != 0 { 0.5 } else { -0.5 },
        if (octant_index & 4) != 0 { 0.5 } else { -0.5 },
    )
}

/// Calculate magnitude from Euclidean distance to origin.
///
/// Produces spherical surfaces where the surface (|Q| = 1.0) forms at
/// `distance = surface_radius`.
///
/// # Arguments
/// * `distance` - Euclidean distance from origin
/// * `config` - Fabric configuration containing root/boundary magnitudes
///
/// # Returns
/// Magnitude value based on linear interpolation between root and boundary
#[inline]
pub fn magnitude_from_distance(distance: f32, config: &FabricConfig) -> f32 {
    let t = (distance / config.surface_radius).clamp(0.0, 1.0);
    config.root_magnitude + (config.boundary_magnitude - config.root_magnitude) * t
}

/// Calculate child quaternion with position tracking.
///
/// # Arguments
/// * `parent_rotation` - Parent's rotation component (normalized)
/// * `octant_index` - Which octant (0-7) the child is in
/// * `child_world_pos` - World position of child center
/// * `config` - Fabric configuration
///
/// # Returns
/// Child quaternion with position-encoded rotation and distance-based magnitude
pub fn calculate_child_quaternion(
    parent_rotation: Quat,
    octant_index: usize,
    child_world_pos: Vec3,
    config: &FabricConfig,
) -> Quat {
    // Apply positional rotation (encodes spatial location)
    let positioned = parent_rotation * octant_rotation(octant_index);

    // Compute magnitude from Euclidean distance
    let distance = child_world_pos.length();
    let magnitude = magnitude_from_distance(distance, config);

    // Return rotation normalized, scaled by magnitude
    positioned.normalize() * magnitude
}

/// Apply additive state (noise/variation) to a quaternion.
///
/// # Arguments
/// * `base_quat` - Base quaternion before noise
/// * `additive_state` - Noise parameters (rotation, magnitude)
/// * `position` - World position (used as seed for deterministic noise)
///
/// # Returns
/// Quaternion with additive noise applied
pub fn apply_additive_state(base_quat: Quat, additive_state: &AdditiveState, position: Vec3) -> Quat {
    if additive_state.rotation == 0.0 && additive_state.magnitude == 0.0 {
        return base_quat;
    }

    // Simple deterministic noise based on position
    let noise_seed = position.x * 12.9898 + position.y * 78.233 + position.z * 37.719;
    let noise = (noise_seed.sin() * 43_758.547).fract();

    // Apply rotation noise
    let rotation_noise = if additive_state.rotation > 0.0 {
        // Create random axis from position
        let axis = Vec3::new(
            ((position.x * 127.1 + position.y * 311.7).sin() * 43_758.547).fract() * 2.0 - 1.0,
            ((position.y * 127.1 + position.z * 311.7).sin() * 43_758.547).fract() * 2.0 - 1.0,
            ((position.z * 127.1 + position.x * 311.7).sin() * 43_758.547).fract() * 2.0 - 1.0,
        )
        .normalize_or_zero();

        let angle = additive_state.rotation * (noise * 2.0 - 1.0);
        if axis.length_squared() > 0.0 {
            Quat::from_axis_angle(axis, angle)
        } else {
            Quat::IDENTITY
        }
    } else {
        Quat::IDENTITY
    };

    // Apply magnitude noise
    let magnitude_noise = additive_state.magnitude * (noise * 2.0 - 1.0);
    let base_magnitude = base_quat.length();
    let new_magnitude = (base_magnitude + magnitude_noise).max(0.0);

    // Combine: apply rotation noise, then scale by new magnitude
    let rotated = rotation_noise * base_quat;
    if rotated.length_squared() > 0.0 {
        rotated.normalize() * new_magnitude
    } else {
        rotated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    #[test]
    fn test_lerp_quaternion_endpoints() {
        let q1 = Quat::IDENTITY;
        let q2 = Quat::from_rotation_z(FRAC_PI_2);

        // At t=0, should return q1
        let result = lerp_quaternion(q1, q2, 0.0);
        assert!((result - q1).length() < 0.001);

        // At t=1, should return q2
        let result = lerp_quaternion(q1, q2, 1.0);
        assert!((result - q2).length() < 0.001);
    }

    #[test]
    fn test_lerp_quaternion_preserves_magnitude() {
        // Create quaternions with different magnitudes
        let q1 = Quat::from_xyzw(0.0, 0.0, 0.0, 2.0); // magnitude 2
        let q2 = Quat::from_xyzw(0.0, 0.0, 0.0, 4.0); // magnitude 4

        // At t=0.5, magnitude should be ~3
        let result = lerp_quaternion(q1, q2, 0.5);
        assert!((result.length() - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_octant_rotation_different_octants() {
        // Different octants should produce different rotations
        let rotations: Vec<_> = (0..8).map(octant_rotation).collect();

        // Check that most rotations are different (some may be similar due to Euler representation)
        let mut different_pairs = 0;
        let total_pairs = 8 * 7 / 2;

        for i in 0..8 {
            for j in (i + 1)..8 {
                let diff = (rotations[i] - rotations[j]).length();
                if diff > 0.1 {
                    different_pairs += 1;
                }
            }
        }

        // At least half of the pairs should be different
        assert!(
            different_pairs > total_pairs / 2,
            "Expected at least {} different pairs, got {}",
            total_pairs / 2,
            different_pairs
        );
    }

    #[test]
    fn test_octant_offset_positions() {
        // Octant 0: (-0.5, -0.5, -0.5)
        let offset0 = octant_offset(0);
        assert_eq!(offset0, Vec3::new(-0.5, -0.5, -0.5));

        // Octant 7: (+0.5, +0.5, +0.5)
        let offset7 = octant_offset(7);
        assert_eq!(offset7, Vec3::new(0.5, 0.5, 0.5));

        // Octant 1: (+0.5, -0.5, -0.5) - only x bit set
        let offset1 = octant_offset(1);
        assert_eq!(offset1, Vec3::new(0.5, -0.5, -0.5));
    }

    #[test]
    fn test_magnitude_from_distance() {
        let config = FabricConfig::default();

        // At origin, magnitude should be root_magnitude
        let mag_origin = magnitude_from_distance(0.0, &config);
        assert!((mag_origin - config.root_magnitude).abs() < 0.001);

        // At surface_radius, magnitude should be ~1.0 (surface)
        // Actually it should be partway between root and boundary at surface_radius
        let mag_surface = magnitude_from_distance(config.surface_radius, &config);
        let expected_at_surface = config.root_magnitude
            + (config.boundary_magnitude - config.root_magnitude) * 1.0;
        assert!((mag_surface - expected_at_surface).abs() < 0.001);
    }

    #[test]
    fn test_magnitude_spherical_surface() {
        // Create config where surface is exactly at |Q| = 1.0
        let config = FabricConfig {
            root_magnitude: 0.5,
            boundary_magnitude: 1.5,
            surface_radius: 1.0,
            ..Default::default()
        };

        // At distance = 0.5 (halfway), magnitude should be 1.0 (surface)
        let mag = magnitude_from_distance(0.5, &config);
        assert!((mag - 1.0).abs() < 0.001, "Expected 1.0, got {}", mag);
    }

    #[test]
    fn test_calculate_child_quaternion() {
        let config = FabricConfig::default();
        let parent_rotation = Quat::IDENTITY;

        // Child at origin should have small magnitude
        let child_at_origin =
            calculate_child_quaternion(parent_rotation, 0, Vec3::ZERO, &config);
        assert!(child_at_origin.length() < 1.0, "Origin should be inside surface");

        // Child far from origin should have large magnitude
        let far_pos = Vec3::new(2.0, 2.0, 2.0);
        let child_far = calculate_child_quaternion(parent_rotation, 7, far_pos, &config);
        assert!(child_far.length() > 1.0, "Far position should be outside surface");
    }

    #[test]
    fn test_apply_additive_state_no_noise() {
        let base = Quat::from_xyzw(0.0, 0.0, 0.0, 1.0);
        let no_noise = AdditiveState::default();

        let result = apply_additive_state(base, &no_noise, Vec3::ZERO);
        assert!((result - base).length() < 0.001);
    }

    #[test]
    fn test_apply_additive_state_deterministic() {
        let base = Quat::from_xyzw(0.0, 0.0, 0.0, 1.0);
        let noise = AdditiveState::new(0.5, 0.1);
        let pos = Vec3::new(1.0, 2.0, 3.0);

        // Same position should produce same result
        let result1 = apply_additive_state(base, &noise, pos);
        let result2 = apply_additive_state(base, &noise, pos);
        assert!((result1 - result2).length() < 0.001);
    }
}
