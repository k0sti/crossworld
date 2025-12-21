//! Collision geometry generation from voxel cubes
//!
//! This module provides utilities for generating Rapier3D colliders from
//! voxel octree structures. It uses the cube crate's face traversal to
//! identify exposed faces and creates compound colliders from them.

use crate::collision::Aabb;
use cube::{visit_faces, visit_faces_in_region, Cube, FaceInfo, RegionBounds};
use glam::{Quat, Vec3};
use rapier3d::prelude::*;
use std::rc::Rc;

/// Builder for generating collision geometry from voxel cubes
///
/// Uses the cube crate's face traversal to iterate through exposed voxel faces
/// and generates thin cuboid colliders for each face.
pub struct VoxelColliderBuilder {
    rectangles: Vec<FaceRectangle>,
}

/// Represents a face rectangle for collision generation
#[derive(Debug, Clone)]
struct FaceRectangle {
    center: Vec3,
    normal: Vec3,
    size: f32,
}

impl VoxelColliderBuilder {
    /// Create a new collider builder
    pub fn new() -> Self {
        Self {
            rectangles: Vec::new(),
        }
    }

    /// Generate a compound collider from a voxel cube
    ///
    /// Processes all exposed faces and creates thin cuboid colliders for each.
    /// The collider is generated in [0,1] normalized space.
    ///
    /// # Arguments
    /// * `cube` - The octree cube to generate collision from
    /// * `_max_depth` - Deprecated parameter (kept for API compatibility)
    ///
    /// # Returns
    /// Rapier Collider containing compound shape of all exposed faces
    pub fn from_cube(cube: &Rc<Cube<u8>>, _max_depth: u32) -> Collider {
        Self::from_cube_with_region(cube, None)
    }

    /// Generate a compound collider from a voxel cube with world-space scaling
    ///
    /// Processes all exposed faces and creates thin cuboid colliders for each,
    /// scaled and positioned to world coordinates.
    ///
    /// # Arguments
    /// * `cube` - The octree cube to generate collision from
    /// * `_max_depth` - Deprecated parameter (kept for API compatibility)
    /// * `world_size` - The world size (collider will span [-world_size/2, world_size/2])
    ///
    /// # Returns
    /// Rapier Collider containing compound shape of all exposed faces in world space
    pub fn from_cube_scaled(cube: &Rc<Cube<u8>>, _max_depth: u32, world_size: f32) -> Collider {
        Self::from_cube_with_region_scaled(cube, None, world_size)
    }

    /// Generate a compound collider from a voxel cube with optional spatial filtering
    ///
    /// This is an optimized version that allows filtering voxels to a specific region.
    /// Only voxels within the region bounds will be processed, significantly reducing
    /// collision complexity when only a subset of the voxel object participates
    /// in collision (e.g., when AABBs barely overlap).
    ///
    /// # Arguments
    /// * `cube` - The octree cube to generate collision from
    /// * `region` - Optional region bounds to filter voxels. If None, processes all voxels.
    ///
    /// # Returns
    /// Rapier Collider containing compound shape of exposed faces in the region
    ///
    /// # Performance
    /// - Full collision (region = None): O(n) faces for n voxels
    /// - Filtered collision: O(k) faces for k voxels in overlap region
    /// - Typical reduction: 70-90% fewer faces for small overlap regions
    pub fn from_cube_with_region(cube: &Rc<Cube<u8>>, region: Option<&RegionBounds>) -> Collider {
        let mut builder = Self::new();

        // Border materials: solid at bottom, empty at top
        let border_materials = [1, 1, 0, 0];

        match region {
            Some(bounds) => {
                // Use region-bounded traversal for efficiency
                visit_faces_in_region(
                    cube,
                    bounds,
                    |face_info| {
                        builder.add_face_from_info(face_info);
                    },
                    border_materials,
                );
            }
            None => {
                // Full traversal
                visit_faces(
                    cube,
                    |face_info| {
                        builder.add_face_from_info(face_info);
                    },
                    border_materials,
                );
            }
        }

        builder.build_compound_collider()
    }

    /// Generate a compound collider from a voxel cube with optional spatial filtering and world-space scaling
    ///
    /// # Arguments
    /// * `cube` - The octree cube to generate collision from
    /// * `region` - Optional region bounds to filter voxels. If None, processes all voxels.
    /// * `world_size` - The world size (collider will span [-world_size/2, world_size/2])
    ///
    /// # Returns
    /// Rapier Collider containing compound shape of exposed faces in world space
    pub fn from_cube_with_region_scaled(
        cube: &Rc<Cube<u8>>,
        region: Option<&RegionBounds>,
        world_size: f32,
    ) -> Collider {
        let mut builder = Self::new();

        // Border materials: solid at bottom, empty at top
        let border_materials = [1, 1, 0, 0];

        match region {
            Some(bounds) => {
                // Use region-bounded traversal for efficiency
                visit_faces_in_region(
                    cube,
                    bounds,
                    |face_info| {
                        builder.add_face_from_info(face_info);
                    },
                    border_materials,
                );
            }
            None => {
                // Full traversal
                visit_faces(
                    cube,
                    |face_info| {
                        builder.add_face_from_info(face_info);
                    },
                    border_materials,
                );
            }
        }

        builder.build_compound_collider_scaled(world_size)
    }

    /// Generate a compound collider with an AABB filter (convenience wrapper)
    ///
    /// # Arguments
    /// * `cube` - The octree cube to generate collision from
    /// * `_max_depth` - Deprecated parameter (kept for API compatibility)
    /// * `region` - Optional AABB in local [0,1] space to filter voxels
    ///
    /// # Returns
    /// Rapier Collider containing compound shape of exposed faces in the region
    pub fn from_cube_region(
        cube: &Rc<Cube<u8>>,
        _max_depth: u32,
        region: Option<&Aabb>,
    ) -> Collider {
        match region {
            Some(aabb) => {
                // Convert AABB to RegionBounds
                // Use depth 3 for a reasonable granularity (8x8x8 cells)
                let depth = 3;
                match RegionBounds::from_local_aabb(aabb.min, aabb.max, depth) {
                    Some(bounds) => Self::from_cube_with_region(cube, Some(&bounds)),
                    None => {
                        // AABB doesn't intersect cube - return minimal collider
                        ColliderBuilder::ball(0.001).build()
                    }
                }
            }
            None => Self::from_cube_with_region(cube, None),
        }
    }

    /// Add a face rectangle from FaceInfo
    fn add_face_from_info(&mut self, face_info: &FaceInfo) {
        let normal_array = face_info.face.normal();
        let normal = Vec3::from(normal_array);

        // Calculate face center position
        // face_info.position is the voxel's base position in [0,1] space
        // Face center is at voxel center + half size in normal direction
        let voxel_center = face_info.position + Vec3::splat(face_info.size * 0.5);
        let face_offset = normal * face_info.size * 0.5;
        let face_center = voxel_center + face_offset;

        self.rectangles.push(FaceRectangle {
            center: face_center,
            normal,
            size: face_info.size,
        });
    }

    /// Build a compound collider from all collected face rectangles
    fn build_compound_collider(self) -> Collider {
        self.build_compound_collider_with_scale(1.0, Vec3::ZERO)
    }

    /// Build a compound collider from all collected face rectangles with world-space scaling
    ///
    /// # Arguments
    /// * `world_size` - The world size (collider will span [-world_size/2, world_size/2])
    fn build_compound_collider_scaled(self, world_size: f32) -> Collider {
        // Transform from [0,1] to [-half_world, half_world]
        let half_world = world_size / 2.0;
        let offset = Vec3::splat(-half_world); // Shift [0,1] to [-0.5,0.5] then scale
        self.build_compound_collider_with_scale(world_size, offset)
    }

    /// Build a compound collider with scale and offset
    fn build_compound_collider_with_scale(self, scale: f32, offset: Vec3) -> Collider {
        if self.rectangles.is_empty() {
            // Empty collider - just use a tiny sphere
            return ColliderBuilder::ball(0.001).build();
        }

        // Create thin cuboid colliders for each face
        // Thickness is fixed (thin shell), face size scales with world
        let thickness = 0.5; // Fixed thin shell thickness in world units
        let shapes: Vec<_> = self
            .rectangles
            .iter()
            .map(|rect| {
                let half_size = (rect.size * scale) / 2.0;

                // Create cuboid shape
                // The cuboid is oriented with Z as the normal direction initially
                let shape = SharedShape::cuboid(half_size, half_size, thickness);

                // Calculate rotation to align Z-axis with face normal
                let rotation = Self::rotation_from_normal(rect.normal);

                // Create isometry (position + rotation)
                // Transform position from [0,1] to world coords
                let pos = rect.center * scale + offset;
                let isometry = Isometry::new(
                    vector![pos.x, pos.y, pos.z],
                    vector![rotation.x, rotation.y, rotation.z],
                );

                (isometry, shape)
            })
            .collect();

        ColliderBuilder::compound(shapes).build()
    }

    /// Calculate rotation axis-angle from a normal vector
    ///
    /// Rotates from Z-axis to align with the given normal
    fn rotation_from_normal(normal: Vec3) -> Vec3 {
        let z_axis = Vec3::Z;

        // If normal is already aligned with Z, no rotation needed
        if (normal - z_axis).length() < 0.001 {
            return Vec3::ZERO;
        }

        // If normal is opposite to Z, rotate 180 degrees around X
        if (normal + z_axis).length() < 0.001 {
            return Vec3::new(std::f32::consts::PI, 0.0, 0.0);
        }

        // Calculate rotation quaternion
        let quat = Quat::from_rotation_arc(z_axis, normal);

        // Convert to axis-angle representation for Rapier
        let (axis, angle) = quat.to_axis_angle();
        axis * angle
    }

    /// Get the number of face rectangles collected
    pub fn face_count(&self) -> usize {
        self.rectangles.len()
    }
}

impl Default for VoxelColliderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create a simple box collider
///
/// # Arguments
/// * `half_extents` - Half the size in each dimension
pub fn create_box_collider(half_extents: Vec3) -> Collider {
    ColliderBuilder::cuboid(half_extents.x, half_extents.y, half_extents.z).build()
}

/// Helper function to create a sphere collider
///
/// # Arguments
/// * `radius` - Sphere radius
pub fn create_sphere_collider(radius: f32) -> Collider {
    ColliderBuilder::ball(radius).build()
}

/// Helper function to create a capsule collider
///
/// # Arguments
/// * `half_height` - Half the height of the cylindrical part
/// * `radius` - Radius of the capsule
pub fn create_capsule_collider(half_height: f32, radius: f32) -> Collider {
    ColliderBuilder::capsule_y(half_height, radius).build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collider_from_solid_cube() {
        let cube = Rc::new(Cube::Solid(1));
        let collider = VoxelColliderBuilder::from_cube(&cube, 3);

        // A solid cube should generate colliders
        assert!(collider.shape().as_compound().is_some());
    }

    #[test]
    fn test_collider_from_empty_cube() {
        let cube = Rc::new(Cube::Solid(0)); // Empty
        let collider = VoxelColliderBuilder::from_cube(&cube, 3);

        // Empty cube should generate minimal collider (ball)
        assert!(collider.shape().as_ball().is_some());
    }

    #[test]
    fn test_rotation_from_normal() {
        // Test Z-axis (no rotation)
        let rot = VoxelColliderBuilder::rotation_from_normal(Vec3::Z);
        assert!(rot.length() < 0.01);

        // Test -Z-axis (180 degree rotation)
        let rot = VoxelColliderBuilder::rotation_from_normal(-Vec3::Z);
        assert!((rot.length() - std::f32::consts::PI).abs() < 0.01);

        // Test other axes
        let _rot_x = VoxelColliderBuilder::rotation_from_normal(Vec3::X);
        let _rot_y = VoxelColliderBuilder::rotation_from_normal(Vec3::Y);
        // Just verify they don't panic
    }

    #[test]
    fn test_simple_colliders() {
        let box_collider = create_box_collider(Vec3::ONE);
        assert!(box_collider.shape().as_cuboid().is_some());

        let sphere_collider = create_sphere_collider(1.0);
        assert!(sphere_collider.shape().as_ball().is_some());

        let capsule_collider = create_capsule_collider(1.0, 0.5);
        assert!(capsule_collider.shape().as_capsule().is_some());
    }

    #[test]
    fn test_from_cube_with_region() {
        let cube = Rc::new(Cube::Solid(1));

        // Full traversal
        let full_collider = VoxelColliderBuilder::from_cube_with_region(&cube, None);
        assert!(full_collider.shape().as_compound().is_some());

        // Partial region (corner)
        let bounds = RegionBounds::from_local_aabb(Vec3::ZERO, Vec3::splat(0.4), 2).unwrap();
        let partial_collider = VoxelColliderBuilder::from_cube_with_region(&cube, Some(&bounds));

        // Both should produce valid colliders
        assert!(
            partial_collider.shape().as_compound().is_some()
                || partial_collider.shape().as_ball().is_some()
        );
    }

    #[test]
    fn test_from_cube_region_with_aabb() {
        let cube = Rc::new(Cube::Solid(1));

        // Test with AABB filter
        let aabb = Aabb::new(Vec3::ZERO, Vec3::splat(0.5));
        let collider = VoxelColliderBuilder::from_cube_region(&cube, 3, Some(&aabb));

        assert!(collider.shape().as_compound().is_some() || collider.shape().as_ball().is_some());
    }

    #[test]
    fn test_empty_region() {
        let cube = Rc::new(Cube::Solid(1));

        // AABB completely outside the cube
        let aabb = Aabb::new(Vec3::splat(10.0), Vec3::splat(11.0));
        let collider = VoxelColliderBuilder::from_cube_region(&cube, 3, Some(&aabb));

        // Should return minimal collider
        assert!(collider.shape().as_ball().is_some());
    }

    #[test]
    fn test_region_reduces_faces() {
        // Create a subdivided cube
        let cube = Rc::new(Cube::tabulate(|_| Cube::Solid(1)));

        // Full traversal
        let mut full_builder = VoxelColliderBuilder::new();
        visit_faces(&cube, |f| full_builder.add_face_from_info(f), [1, 1, 0, 0]);
        let full_count = full_builder.face_count();

        // Partial region
        let bounds = RegionBounds::from_local_aabb(Vec3::ZERO, Vec3::splat(0.4), 2).unwrap();
        let mut partial_builder = VoxelColliderBuilder::new();
        visit_faces_in_region(
            &cube,
            &bounds,
            |f| partial_builder.add_face_from_info(f),
            [1, 1, 0, 0],
        );
        let partial_count = partial_builder.face_count();

        // Region should have fewer faces
        assert!(
            partial_count < full_count,
            "Partial count {} should be < full count {}",
            partial_count,
            full_count
        );
    }
}
