//! World collision using direct octree queries
//!
//! This module provides world collision handling by directly querying the octree
//! for solid voxels. This bypasses Rapier for world collision, using Rapier only
//! for dynamic↔dynamic collision detection.

use crate::collision::Aabb;
use crate::PhysicsWorld;
use cube::{visit_voxels_in_region, Cube, RegionBounds};
use glam::Vec3;
use rapier3d::prelude::*;
use std::rc::Rc;
use std::time::Instant;

/// Performance metrics for the world collider
#[derive(Debug, Clone, Default)]
pub struct ColliderMetrics {
    /// Name of the collision strategy
    pub strategy_name: &'static str,
    /// Time taken to initialize in milliseconds
    pub init_time_ms: f32,
    /// Average update time per frame in microseconds
    pub update_time_us: f32,
    /// Number of active colliders in physics world
    pub active_colliders: usize,
    /// Total number of faces represented
    pub total_faces: usize,
}

/// World collider using direct octree queries
///
/// Uses Rapier only for dynamic↔dynamic collision. World collision is
/// resolved directly via octree queries, avoiding compound collider overhead.
pub struct WorldCollider {
    cube: Option<Rc<Cube<u8>>>,
    world_size: f32,
    border_materials: [u8; 4],
    init_time_ms: f32,
}

impl WorldCollider {
    pub fn new() -> Self {
        Self {
            cube: None,
            world_size: 0.0,
            border_materials: [1, 1, 0, 0],
            init_time_ms: 0.0,
        }
    }

    /// Initialize the collider with world cube and physics world
    ///
    /// # Arguments
    /// * `cube` - The world octree cube
    /// * `world_size` - World size in units (collider spans [-world_size/2, world_size/2])
    /// * `border_materials` - Materials for border traversal [bottom_inner, bottom_outer, top_inner, top_outer]
    /// * `_physics` - Physics world (unused, but kept for API consistency)
    pub fn init(
        &mut self,
        cube: &Rc<Cube<u8>>,
        world_size: f32,
        border_materials: [u8; 4],
        _physics: &mut PhysicsWorld,
    ) {
        let start = Instant::now();

        self.cube = Some(cube.clone());
        self.world_size = world_size;
        self.border_materials = border_materials;

        // No Rapier colliders created!

        self.init_time_ms = start.elapsed().as_secs_f32() * 1000.0;
    }

    /// Update colliders based on dynamic object positions
    ///
    /// No-op for this implementation since no colliders are managed.
    pub fn update(
        &mut self,
        _dynamic_aabbs: &[(RigidBodyHandle, Aabb)],
        _physics: &mut PhysicsWorld,
    ) {
        // No collider management needed
    }

    /// Resolve world collisions for a body
    ///
    /// Computes and returns the penetration correction vector by directly
    /// querying the octree for solid voxels.
    ///
    /// # Arguments
    /// * `_body_handle` - Handle to the body to resolve (unused)
    /// * `body_aabb` - World AABB of the body
    ///
    /// # Returns
    /// Correction vector to apply to body position
    pub fn resolve_collision(&self, _body_handle: RigidBodyHandle, body_aabb: &Aabb) -> Vec3 {
        let cube = match &self.cube {
            Some(c) => c,
            None => return Vec3::ZERO,
        };

        // Convert body AABB to octree local space [0,1]
        let half_world = self.world_size / 2.0;
        let local_min = (body_aabb.min + Vec3::splat(half_world)) / self.world_size;
        let local_max = (body_aabb.max + Vec3::splat(half_world)) / self.world_size;

        // Clamp to [0,1] bounds and add a small margin to catch nearby faces
        let margin = 0.01 / self.world_size; // 1cm margin in world units
        let local_min = (local_min - Vec3::splat(margin)).max(Vec3::ZERO);
        let local_max = (local_max + Vec3::splat(margin)).min(Vec3::ONE);

        // Get region bounds for octree query
        let depth = 3; // Reasonable granularity for collision
        let bounds = match RegionBounds::from_local_aabb(local_min, local_max, depth) {
            Some(b) => b,
            None => return Vec3::ZERO,
        };

        // Query voxels in region and compute penetration
        // Track maximum penetration per axis to avoid over-correction from multiple voxels
        let mut max_correction = Vec3::ZERO;

        visit_voxels_in_region(
            cube,
            &bounds,
            |voxel_info| {
                // Voxel AABB in world space
                let voxel_size_world = voxel_info.size * self.world_size;
                let voxel_center_local = voxel_info.position + Vec3::splat(voxel_info.size * 0.5);
                let voxel_center_world =
                    voxel_center_local * self.world_size - Vec3::splat(half_world);

                let voxel_min = voxel_center_world - Vec3::splat(voxel_size_world * 0.5);
                let voxel_max = voxel_center_world + Vec3::splat(voxel_size_world * 0.5);

                // Check AABB overlap
                let overlap_min = body_aabb.min.max(voxel_min);
                let overlap_max = body_aabb.max.min(voxel_max);

                if overlap_min.x < overlap_max.x
                    && overlap_min.y < overlap_max.y
                    && overlap_min.z < overlap_max.z
                {
                    // Calculate penetration depths (push out distances)
                    let dx1 = voxel_max.x - body_aabb.min.x; // Distance to push positive X
                    let dx2 = body_aabb.max.x - voxel_min.x; // Distance to push negative X
                    let dy1 = voxel_max.y - body_aabb.min.y; // Distance to push positive Y (UP)
                    let dy2 = body_aabb.max.y - voxel_min.y; // Distance to push negative Y (DOWN)
                    let dz1 = voxel_max.z - body_aabb.min.z; // Distance to push positive Z
                    let dz2 = body_aabb.max.z - voxel_min.z; // Distance to push negative Z

                    // Find minimum absolute penetration axis
                    let pen_x = if dx1 < dx2 { dx1 } else { -dx2 };
                    let pen_y = if dy1 < dy2 { dy1 } else { -dy2 };
                    let pen_z = if dz1 < dz2 { dz1 } else { -dz2 };

                    let abs_x = pen_x.abs();
                    let abs_y = pen_y.abs();
                    let abs_z = pen_z.abs();

                    if abs_x < abs_y && abs_x < abs_z {
                        if abs_x > max_correction.x.abs() {
                            max_correction.x = pen_x;
                        }
                    } else if abs_y < abs_z {
                        if abs_y > max_correction.y.abs() {
                            max_correction.y = pen_y;
                        }
                    } else if abs_z > max_correction.z.abs() {
                        max_correction.z = pen_z;
                    }
                }
            },
            self.border_materials,
        );

        max_correction
    }

    /// Get performance metrics
    pub fn metrics(&self) -> ColliderMetrics {
        ColliderMetrics {
            strategy_name: "world",
            init_time_ms: self.init_time_ms,
            update_time_us: 0.0,
            active_colliders: 0, // No Rapier colliders
            total_faces: 0,
        }
    }
}

impl Default for WorldCollider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_collider_metrics() {
        let collider = WorldCollider::new();
        let metrics = collider.metrics();
        assert_eq!(metrics.strategy_name, "world");
        assert_eq!(metrics.active_colliders, 0);
    }

    #[test]
    fn test_resolve_collision_with_solid_cube() {
        use cube::Cube;
        use std::rc::Rc;

        // Create a solid cube (represents ground)
        let cube = Rc::new(Cube::Solid(1u8));

        // Create world collider with world size 100, world centered at origin
        let mut collider = WorldCollider::new();
        collider.world_size = 100.0;
        collider.cube = Some(cube);
        // Border materials: solid at bottom (y=0,1), empty at top (y=2,3)
        collider.border_materials = [1, 1, 0, 0];

        // Test 1: Box above solid cube (no penetration expected)
        // Solid cube fills [0,1] in local space → [-50, 50] in world space
        // World top surface is at Y=50
        // Box at Y=55 (well above surface) should not penetrate
        let box_above = Aabb::new(Vec3::new(-5.0, 55.0, -5.0), Vec3::new(5.0, 65.0, 5.0));
        let correction_above =
            collider.resolve_collision(RigidBodyHandle::from_raw_parts(0, 0), &box_above);
        assert!(
            correction_above.length() < 0.1,
            "Box above surface should not get correction, got {:?}",
            correction_above
        );

        // Test 2: Box penetrating into solid cube
        // Box at Y=45 to Y=55 (10 units tall, center at Y=50)
        // This should penetrate 5 units into the top surface at Y=50
        let box_penetrating = Aabb::new(Vec3::new(-5.0, 45.0, -5.0), Vec3::new(5.0, 55.0, 5.0));
        let correction =
            collider.resolve_collision(RigidBodyHandle::from_raw_parts(0, 0), &box_penetrating);

        // Correction should be non-zero (pushing out of solid)
        assert!(
            correction.length() > 0.0,
            "Correction should be non-zero, got {:?}",
            correction
        );
    }

    #[test]
    fn test_with_half_solid_world() {
        use cube::{Cube, IVec3Ext};
        use glam::IVec3;
        use std::rc::Rc;

        // Create a world where bottom half is solid, top half is empty
        // Octant index = x + y*2 + z*4
        // Y=0 (bottom): octants 0,1,4,5
        // Y=1 (top): octants 2,3,6,7
        let cube = Rc::new(Cube::tabulate(|octant| {
            let pos = IVec3::from_octant_index(octant);
            if pos.y == 0 {
                Cube::Solid(1) // Ground (bottom half)
            } else {
                Cube::Solid(0) // Air (top half)
            }
        }));

        let mut collider = WorldCollider::new();
        collider.world_size = 100.0;
        collider.cube = Some(cube);
        collider.border_materials = [1, 1, 0, 0];

        // Test: Box at world center (Y=0) should be at the ground surface
        // In world space: bottom is Y=-50, top is Y=50, ground surface is at Y=0
        // Box from Y=-5 to Y=5 should penetrate 5 units into ground
        let box_at_surface = Aabb::new(Vec3::new(-5.0, -5.0, -5.0), Vec3::new(5.0, 5.0, 5.0));
        let correction =
            collider.resolve_collision(RigidBodyHandle::from_raw_parts(0, 0), &box_at_surface);

        // Should get a non-zero correction
        println!("Half-solid world correction: {:?}", correction);
        assert!(
            correction.length() > 0.0,
            "Box penetrating ground should get non-zero correction, got {:?}",
            correction
        );
    }

    #[test]
    fn test_query_depth_scaling() {
        use cube::{Cube, IVec3Ext};
        use glam::IVec3;
        use std::rc::Rc;

        // Test with a large world (like proto-gl's 8192 units)
        let cube = Rc::new(Cube::tabulate(|octant| {
            let pos = IVec3::from_octant_index(octant);
            if pos.y == 0 {
                Cube::Solid(1) // Ground
            } else {
                Cube::Solid(0) // Air
            }
        }));

        let mut collider = WorldCollider::new();
        collider.world_size = 8192.0; // Large world
        collider.cube = Some(cube);
        collider.border_materials = [32, 32, 0, 0];

        // Small object (10 units) penetrating the surface
        // Object from Y=-5 to Y=5 at world center
        let small_box = Aabb::new(Vec3::new(-5.0, -5.0, -5.0), Vec3::new(5.0, 5.0, 5.0));
        let correction =
            collider.resolve_collision(RigidBodyHandle::from_raw_parts(0, 0), &small_box);

        println!("Large world correction: {:?}", correction);
        // With proper depth scaling, we should still detect the collision
        assert!(
            correction.length() > 0.0,
            "Should detect collision even in large world, got {:?}",
            correction
        );
    }

    #[test]
    fn test_face_info_position_debug() {
        use cube::{visit_faces, Cube, IVec3Ext};
        use glam::IVec3;
        use std::rc::Rc;

        // Create ground cube (bottom half solid)
        let cube = Rc::new(Cube::tabulate(|octant| {
            let pos = IVec3::from_octant_index(octant);
            if pos.y == 0 {
                Cube::Solid(1)
            } else {
                Cube::Solid(0)
            }
        }));

        println!("\n=== Face positions from octree ===");
        visit_faces(
            &cube,
            |face_info| {
                let normal = Vec3::from(face_info.face.normal());
                let voxel_center = face_info.position + Vec3::splat(face_info.size * 0.5);
                let face_center = voxel_center + normal * face_info.size * 0.5;
                println!(
                    "Face {:?}: voxel_pos={:?}, size={}, normal={:?}, face_center={:?}",
                    face_info.face, face_info.position, face_info.size, normal, face_center
                );
            },
            [1, 1, 0, 0],
        );
    }
}
