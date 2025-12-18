//! Signed Distance Function (SDF) collision interface
//!
//! This module provides a trait for SDF-based collision detection, designed
//! for smooth surfaces like fabric-generated voxel models. SDF collision
//! enables continuous collision detection with smooth contact normals.
//!
//! # SDF Convention
//!
//! The signed distance function follows standard convention:
//! - **Negative values**: Point is inside the surface (solid material)
//! - **Zero**: Point is exactly on the surface
//! - **Positive values**: Point is outside the surface (air/empty)
//!
//! For fabric models, the SDF is derived from quaternion field magnitude:
//! - `|Q| < 1.0`: Inside (solid)
//! - `|Q| = 1.0`: Surface boundary
//! - `|Q| > 1.0`: Outside (air)
//!
//! # Collision Algorithm Overview
//!
//! SDF-based collision uses sphere marching to find contact points:
//!
//! 1. **Broad Phase**: AABB intersection test (see `collision::Aabb`)
//! 2. **Narrow Phase**: For each potential contact point:
//!    a. Start from collision query point
//!    b. Evaluate SDF at current position
//!    c. If SDF < 0, point is penetrating
//!    d. Compute normal from SDF gradient
//!    e. Push back along normal by |sdf| distance
//!
//! # Performance Considerations
//!
//! - SDF evaluation involves sampling the quaternion field (potentially expensive)
//! - Normal calculation requires 6 SDF samples (central differences)
//! - Consider caching SDF values for nearby queries
//! - Use AABB/region filtering to minimize SDF evaluations

use glam::Vec3;

/// Trait for types that can provide signed distance function evaluation
///
/// Implementors provide distance-to-surface queries for collision detection.
/// The SDF must satisfy the eikonal equation: |∇sdf| = 1 (unit gradient magnitude)
/// for accurate collision response.
///
/// # Example
///
/// ```ignore
/// use crossworld_physics::sdf::SdfCollider;
/// use glam::Vec3;
///
/// struct MySphere {
///     center: Vec3,
///     radius: f32,
/// }
///
/// impl SdfCollider for MySphere {
///     fn sdf(&self, point: Vec3) -> f32 {
///         (point - self.center).length() - self.radius
///     }
///
///     fn normal(&self, point: Vec3) -> Vec3 {
///         (point - self.center).normalize_or_zero()
///     }
/// }
/// ```
pub trait SdfCollider {
    /// Compute signed distance from point to surface
    ///
    /// # Arguments
    /// * `point` - World-space position to query
    ///
    /// # Returns
    /// - Negative value if point is inside the surface
    /// - Zero if point is exactly on the surface
    /// - Positive value if point is outside the surface
    ///
    /// The magnitude represents the distance to the nearest surface point.
    fn sdf(&self, point: Vec3) -> f32;

    /// Compute surface normal at a point
    ///
    /// Returns the normalized gradient of the SDF, pointing outward from the surface.
    /// For points not exactly on the surface, this gives the direction to the nearest
    /// surface point (for negative SDF) or the direction away from the surface (positive SDF).
    ///
    /// # Arguments
    /// * `point` - World-space position to query
    ///
    /// # Returns
    /// Normalized vector pointing outward from the surface.
    /// Returns `Vec3::ZERO` if gradient cannot be computed (e.g., at singularities).
    fn normal(&self, point: Vec3) -> Vec3;

    /// Compute SDF and normal together (optimization)
    ///
    /// Some implementations may compute both values more efficiently together.
    /// Default implementation calls `sdf()` and `normal()` separately.
    ///
    /// # Arguments
    /// * `point` - World-space position to query
    ///
    /// # Returns
    /// Tuple of (signed distance, surface normal)
    fn sdf_and_normal(&self, point: Vec3) -> (f32, Vec3) {
        (self.sdf(point), self.normal(point))
    }

    /// Check if a point is inside the surface
    ///
    /// # Arguments
    /// * `point` - World-space position to query
    ///
    /// # Returns
    /// `true` if the point is inside (SDF < 0)
    fn is_inside(&self, point: Vec3) -> bool {
        self.sdf(point) < 0.0
    }

    /// Find penetration depth and direction for a sphere
    ///
    /// Tests if a sphere penetrates the surface and returns contact information.
    ///
    /// # Arguments
    /// * `center` - Sphere center in world space
    /// * `radius` - Sphere radius
    ///
    /// # Returns
    /// `Some((penetration_depth, contact_normal))` if penetrating, `None` otherwise.
    /// Penetration depth is positive when penetrating.
    fn sphere_penetration(&self, center: Vec3, radius: f32) -> Option<(f32, Vec3)> {
        let distance = self.sdf(center);
        let penetration = radius - distance;

        if penetration > 0.0 {
            let normal = self.normal(center);
            Some((penetration, normal))
        } else {
            None
        }
    }
}

/// Stub implementation for fabric-based SDF (placeholder for future implementation)
///
/// This struct wraps a fabric cube's quaternion field and provides SDF collision.
/// The SDF is derived from quaternion magnitude: `sdf = |Q| - 1.0`
///
/// # Future Implementation
///
/// When fabric feature is enabled, this will:
/// 1. Sample the quaternion field at the query point
/// 2. Return `quat.length() - 1.0` as the SDF value
/// 3. Compute normal via central differences on magnitude field
///
/// # Example (Future API)
///
/// ```ignore
/// use crossworld_physics::sdf::FabricSdf;
/// use cube::FabricGenerator;
///
/// let fabric = FabricGenerator::new(config);
/// let sdf = FabricSdf::from_fabric(&fabric, position, scale);
///
/// if let Some((depth, normal)) = sdf.sphere_penetration(point, 0.5) {
///     // Handle collision response
/// }
/// ```
#[cfg(feature = "fabric")]
pub struct FabricSdf {
    /// Position of the fabric cube in world space
    pub position: Vec3,
    /// Scale of the fabric cube
    pub scale: f32,
    /// Step size for normal calculation (typically voxel_size / 2)
    pub step_size: f32,
    // Note: The actual FabricGenerator reference would go here
    // when the full implementation is added
}

#[cfg(feature = "fabric")]
impl FabricSdf {
    /// Create a new FabricSdf wrapper (stub)
    ///
    /// # Arguments
    /// * `position` - World position of the fabric cube origin
    /// * `scale` - World scale of the fabric cube
    pub fn new(position: Vec3, scale: f32) -> Self {
        Self {
            position,
            scale,
            step_size: scale / 64.0, // Default step for depth 6
        }
    }

    /// Transform world point to local [0,1] space
    fn to_local(&self, world_point: Vec3) -> Vec3 {
        (world_point - self.position) / self.scale
    }
}

#[cfg(feature = "fabric")]
impl SdfCollider for FabricSdf {
    fn sdf(&self, point: Vec3) -> f32 {
        let _local = self.to_local(point);

        // Stub: Return positive (outside) for all points
        // Real implementation would sample quaternion field:
        // let quat = self.fabric.sample(local);
        // quat.length() - 1.0
        1.0
    }

    fn normal(&self, point: Vec3) -> Vec3 {
        let _local = self.to_local(point);
        let _h = self.step_size / self.scale;

        // Stub: Return up vector
        // Real implementation would use central differences:
        // cube::fabric::surface::calculate_normal(local, |p| self.magnitude_at(p), h)
        Vec3::Y
    }
}

/// Sphere SDF implementation for testing and simple collision
///
/// A sphere is the simplest SDF: distance to center minus radius.
pub struct SphereSdf {
    /// Center of the sphere in world space
    pub center: Vec3,
    /// Radius of the sphere
    pub radius: f32,
}

impl SphereSdf {
    /// Create a new sphere SDF
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }
}

impl SdfCollider for SphereSdf {
    fn sdf(&self, point: Vec3) -> f32 {
        (point - self.center).length() - self.radius
    }

    fn normal(&self, point: Vec3) -> Vec3 {
        (point - self.center).normalize_or_zero()
    }
}

/// Box SDF implementation for testing
///
/// An axis-aligned box with half-extents from center.
pub struct BoxSdf {
    /// Center of the box in world space
    pub center: Vec3,
    /// Half-extents (half-size) in each dimension
    pub half_extents: Vec3,
}

impl BoxSdf {
    /// Create a new box SDF
    pub fn new(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            center,
            half_extents,
        }
    }

    /// Create a unit box centered at origin
    pub fn unit() -> Self {
        Self {
            center: Vec3::ZERO,
            half_extents: Vec3::splat(0.5),
        }
    }
}

impl SdfCollider for BoxSdf {
    fn sdf(&self, point: Vec3) -> f32 {
        // Transform to local space (centered at origin)
        let local = (point - self.center).abs();

        // Distance to box surface
        let q = local - self.half_extents;

        // Outside: length of positive components
        // Inside: negative distance to nearest face
        q.max(Vec3::ZERO).length() + q.x.max(q.y.max(q.z)).min(0.0)
    }

    fn normal(&self, point: Vec3) -> Vec3 {
        // Use central differences for robust normal calculation
        let h = 0.001;
        let gradient = Vec3::new(
            self.sdf(point + Vec3::X * h) - self.sdf(point - Vec3::X * h),
            self.sdf(point + Vec3::Y * h) - self.sdf(point - Vec3::Y * h),
            self.sdf(point + Vec3::Z * h) - self.sdf(point - Vec3::Z * h),
        );
        gradient.normalize_or_zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== SphereSdf Tests ====================

    #[test]
    fn test_sphere_sdf_outside() {
        let sphere = SphereSdf::new(Vec3::ZERO, 1.0);

        // Point outside sphere
        let dist = sphere.sdf(Vec3::new(2.0, 0.0, 0.0));
        assert!((dist - 1.0).abs() < 0.001); // 2 - 1 = 1
    }

    #[test]
    fn test_sphere_sdf_inside() {
        let sphere = SphereSdf::new(Vec3::ZERO, 1.0);

        // Point inside sphere
        let dist = sphere.sdf(Vec3::new(0.5, 0.0, 0.0));
        assert!((dist - (-0.5)).abs() < 0.001); // 0.5 - 1 = -0.5
    }

    #[test]
    fn test_sphere_sdf_on_surface() {
        let sphere = SphereSdf::new(Vec3::ZERO, 1.0);

        // Point on surface
        let dist = sphere.sdf(Vec3::new(1.0, 0.0, 0.0));
        assert!(dist.abs() < 0.001);
    }

    #[test]
    fn test_sphere_sdf_normal() {
        let sphere = SphereSdf::new(Vec3::ZERO, 1.0);

        // Normal should point radially outward
        let normal = sphere.normal(Vec3::new(1.0, 0.0, 0.0));
        assert!((normal - Vec3::X).length() < 0.001);

        let normal = sphere.normal(Vec3::new(0.0, 1.0, 0.0));
        assert!((normal - Vec3::Y).length() < 0.001);

        let normal = sphere.normal(Vec3::new(0.0, 0.0, 1.0));
        assert!((normal - Vec3::Z).length() < 0.001);
    }

    #[test]
    fn test_sphere_sdf_is_inside() {
        let sphere = SphereSdf::new(Vec3::ZERO, 1.0);

        assert!(sphere.is_inside(Vec3::ZERO));
        assert!(sphere.is_inside(Vec3::splat(0.3)));
        assert!(!sphere.is_inside(Vec3::new(2.0, 0.0, 0.0)));
    }

    #[test]
    fn test_sphere_penetration() {
        let sphere = SphereSdf::new(Vec3::ZERO, 1.0);

        // Sphere center inside, should penetrate
        let result = sphere.sphere_penetration(Vec3::new(0.5, 0.0, 0.0), 0.3);
        assert!(result.is_some());
        let (depth, normal) = result.unwrap();
        assert!(depth > 0.0); // 0.3 - (0.5 - 1.0) = 0.3 + 0.5 = 0.8
        assert!((normal - Vec3::X).length() < 0.001);

        // Sphere center far outside, no penetration
        let result = sphere.sphere_penetration(Vec3::new(3.0, 0.0, 0.0), 0.5);
        assert!(result.is_none()); // 0.5 - (3.0 - 1.0) = 0.5 - 2.0 = -1.5 < 0
    }

    // ==================== BoxSdf Tests ====================

    #[test]
    fn test_box_sdf_outside() {
        let box_sdf = BoxSdf::new(Vec3::ZERO, Vec3::ONE);

        // Point outside box (along X axis)
        let dist = box_sdf.sdf(Vec3::new(2.0, 0.0, 0.0));
        assert!((dist - 1.0).abs() < 0.01); // 2 - 1 = 1
    }

    #[test]
    fn test_box_sdf_inside() {
        let box_sdf = BoxSdf::new(Vec3::ZERO, Vec3::ONE);

        // Point inside box (at center)
        let dist = box_sdf.sdf(Vec3::ZERO);
        assert!(dist < 0.0); // Negative = inside
        assert!((dist - (-1.0)).abs() < 0.01); // Distance to nearest face is 1
    }

    #[test]
    fn test_box_sdf_on_face() {
        let box_sdf = BoxSdf::new(Vec3::ZERO, Vec3::ONE);

        // Point on face
        let dist = box_sdf.sdf(Vec3::new(1.0, 0.0, 0.0));
        assert!(dist.abs() < 0.01);
    }

    #[test]
    fn test_box_sdf_normal() {
        let box_sdf = BoxSdf::new(Vec3::ZERO, Vec3::ONE);

        // Normal should point outward from faces
        let normal = box_sdf.normal(Vec3::new(1.5, 0.0, 0.0));
        assert!((normal - Vec3::X).length() < 0.1, "Expected +X, got {:?}", normal);

        let normal = box_sdf.normal(Vec3::new(-1.5, 0.0, 0.0));
        assert!((normal - (-Vec3::X)).length() < 0.1, "Expected -X, got {:?}", normal);
    }

    #[test]
    fn test_box_sdf_corner() {
        let box_sdf = BoxSdf::new(Vec3::ZERO, Vec3::ONE);

        // Point at corner (outside)
        let dist = box_sdf.sdf(Vec3::new(2.0, 2.0, 2.0));
        // Distance to corner is sqrt(3) ≈ 1.732
        let expected = (Vec3::new(1.0, 1.0, 1.0)).length();
        assert!((dist - expected).abs() < 0.01);
    }

    // ==================== SdfCollider Trait Tests ====================

    #[test]
    fn test_sdf_and_normal_default() {
        let sphere = SphereSdf::new(Vec3::ZERO, 1.0);
        let point = Vec3::new(2.0, 0.0, 0.0);

        let (dist, normal) = sphere.sdf_and_normal(point);

        assert!((dist - 1.0).abs() < 0.001);
        assert!((normal - Vec3::X).length() < 0.001);
    }
}
