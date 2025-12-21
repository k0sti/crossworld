use crate::collision::Aabb;
use crate::world::PhysicsWorld;
use cube::CubeBox;
use glam::{Quat, Vec3};
use nalgebra::{Quaternion, UnitQuaternion};
use rapier3d::prelude::*;
use std::rc::Rc;

/// Represents a physics object with a rigid body, collider, and voxel cube
///
/// This is a convenience wrapper that combines a rigid body handle,
/// its primary collider handle, and a reference to the voxel CubeBox used
/// for collision geometry. CubeBox preserves actual model dimensions,
/// enabling accurate bounding boxes and collision generation.
#[derive(Debug, Clone)]
pub struct CubeObject {
    pub(crate) body_handle: RigidBodyHandle,
    pub(crate) collider_handle: Option<ColliderHandle>,
    /// The voxel CubeBox used for collision geometry (optional)
    /// Contains both the octree data and actual model dimensions
    pub cube: Option<Rc<CubeBox<u8>>>,
    /// Scale of the cube object (default 1.0)
    scale: f32,
}

impl CubeObject {
    /// Create a new dynamic rigid body
    ///
    /// Dynamic bodies are affected by forces and gravity.
    ///
    /// # Arguments
    /// * `world` - The physics world to add the body to
    /// * `position` - Initial position
    /// * `mass` - Mass of the object (affects inertia)
    ///
    /// # Returns
    /// New CubeObject (without collider - add one separately)
    pub fn new_dynamic(world: &mut PhysicsWorld, position: Vec3, mass: f32) -> Self {
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![position.x, position.y, position.z])
            .additional_mass(mass)
            .build();

        let body_handle = world.add_rigid_body(body);

        Self {
            body_handle,
            collider_handle: None,
            cube: None,
            scale: 1.0,
        }
    }

    /// Create a new kinematic rigid body
    ///
    /// Kinematic bodies are not affected by forces but can be moved programmatically.
    /// They affect dynamic bodies but are not affected by them.
    ///
    /// # Arguments
    /// * `world` - The physics world to add the body to
    /// * `position` - Initial position
    pub fn new_kinematic(world: &mut PhysicsWorld, position: Vec3) -> Self {
        let body = RigidBodyBuilder::kinematic_position_based()
            .translation(vector![position.x, position.y, position.z])
            .build();

        let body_handle = world.add_rigid_body(body);

        Self {
            body_handle,
            collider_handle: None,
            cube: None,
            scale: 1.0,
        }
    }

    /// Create a new static rigid body
    ///
    /// Static bodies never move and are not affected by any forces.
    /// Useful for terrain and immovable obstacles.
    ///
    /// # Arguments
    /// * `world` - The physics world to add the body to
    /// * `position` - Position
    pub fn new_static(world: &mut PhysicsWorld, position: Vec3) -> Self {
        let body = RigidBodyBuilder::fixed()
            .translation(vector![position.x, position.y, position.z])
            .build();

        let body_handle = world.add_rigid_body(body);

        Self {
            body_handle,
            collider_handle: None,
            cube: None,
            scale: 1.0,
        }
    }

    /// Attach a collider to this rigid body
    ///
    /// # Arguments
    /// * `world` - The physics world
    /// * `collider` - The collider to attach
    pub fn attach_collider(&mut self, world: &mut PhysicsWorld, collider: Collider) {
        let handle = world.add_collider(collider, self.body_handle);
        self.collider_handle = Some(handle);
    }

    /// Set the CubeBox reference for this object
    ///
    /// # Arguments
    /// * `cubebox` - Reference to the voxel CubeBox used for collision
    pub fn set_cube(&mut self, cubebox: Rc<CubeBox<u8>>) {
        self.cube = Some(cubebox);
    }

    /// Get the CubeBox reference if it exists
    pub fn cube(&self) -> Option<&Rc<CubeBox<u8>>> {
        self.cube.as_ref()
    }

    /// Get the scale of the cube object
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Set the scale of the cube object
    ///
    /// Note: This does not automatically update the collider. You may need
    /// to regenerate the collider after changing scale.
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    /// Get the local-space AABB for this cube object
    ///
    /// If a CubeBox is attached, returns an AABB sized to the actual model dimensions
    /// normalized by the octree size. For a 16x30x12 model in a depth-5 octree (32³),
    /// returns AABB from (0,0,0) to (0.5, 0.9375, 0.375).
    ///
    /// If no CubeBox is attached, returns the unit cube [0,1]³ as fallback.
    pub fn local_aabb(&self) -> Aabb {
        match &self.cube {
            Some(cubebox) => {
                let octree_size = cubebox.octree_size() as f32;
                let max = Vec3::new(
                    cubebox.size.x as f32 / octree_size,
                    cubebox.size.y as f32 / octree_size,
                    cubebox.size.z as f32 / octree_size,
                );
                Aabb::new(Vec3::ZERO, max)
            }
            None => Aabb::unit(), // Fallback for objects without cube data
        }
    }

    /// Get the world-space AABB for this cube object
    ///
    /// Computes a tight AABB by transforming the local AABB using the object's
    /// position, rotation, and scale. The result is an axis-aligned bounding box
    /// that fully contains the rotated object.
    ///
    /// # Arguments
    /// * `world` - The physics world to query position/rotation from
    ///
    /// # Example
    /// ```ignore
    /// let aabb = cube_object.world_aabb(&world);
    /// if aabb.intersects(&other_aabb) {
    ///     // Potential collision - do narrow phase
    /// }
    /// ```
    pub fn world_aabb(&self, world: &PhysicsWorld) -> Aabb {
        let position = self.position(world);
        let rotation = self.rotation(world);
        self.local_aabb().to_world(position, rotation, self.scale)
    }

    /// Check if this object's AABB intersects with another AABB
    ///
    /// This is a broad-phase collision test. A positive result means
    /// the objects might be colliding and require narrow-phase testing.
    ///
    /// # Arguments
    /// * `world` - The physics world
    /// * `other` - The other AABB to test against
    pub fn intersects_aabb(&self, world: &PhysicsWorld, other: &Aabb) -> bool {
        self.world_aabb(world).intersects(other)
    }

    /// Get the current position of the rigid body
    pub fn position(&self, world: &PhysicsWorld) -> Vec3 {
        if let Some(body) = world.get_rigid_body(self.body_handle) {
            let pos = body.translation();
            Vec3::new(pos.x, pos.y, pos.z)
        } else {
            Vec3::ZERO
        }
    }

    /// Set the position of the rigid body
    ///
    /// For kinematic and static bodies, this moves them directly.
    /// For dynamic bodies, prefer using velocities or forces.
    pub fn set_position(&self, world: &mut PhysicsWorld, position: Vec3) {
        if let Some(body) = world.get_rigid_body_mut(self.body_handle) {
            body.set_translation(vector![position.x, position.y, position.z], true);
        }
    }

    /// Get the current rotation of the rigid body as a quaternion
    pub fn rotation(&self, world: &PhysicsWorld) -> Quat {
        if let Some(body) = world.get_rigid_body(self.body_handle) {
            let rot = body.rotation();
            Quat::from_xyzw(rot.i, rot.j, rot.k, rot.w)
        } else {
            Quat::IDENTITY
        }
    }

    /// Set the rotation of the rigid body
    pub fn set_rotation(&self, world: &mut PhysicsWorld, rotation: Quat) {
        if let Some(body) = world.get_rigid_body_mut(self.body_handle) {
            let rot = UnitQuaternion::new_normalize(Quaternion::new(
                rotation.w, rotation.x, rotation.y, rotation.z,
            ));
            body.set_rotation(rot, true);
        }
    }

    /// Get the linear velocity of the rigid body
    pub fn velocity(&self, world: &PhysicsWorld) -> Vec3 {
        if let Some(body) = world.get_rigid_body(self.body_handle) {
            let vel = body.linvel();
            Vec3::new(vel.x, vel.y, vel.z)
        } else {
            Vec3::ZERO
        }
    }

    /// Set the linear velocity of the rigid body
    ///
    /// Only works for dynamic bodies.
    pub fn set_velocity(&self, world: &mut PhysicsWorld, velocity: Vec3) {
        if let Some(body) = world.get_rigid_body_mut(self.body_handle) {
            body.set_linvel(vector![velocity.x, velocity.y, velocity.z], true);
        }
    }

    /// Get the angular velocity of the rigid body
    pub fn angular_velocity(&self, world: &PhysicsWorld) -> Vec3 {
        if let Some(body) = world.get_rigid_body(self.body_handle) {
            let angvel = body.angvel();
            Vec3::new(angvel.x, angvel.y, angvel.z)
        } else {
            Vec3::ZERO
        }
    }

    /// Set the angular velocity of the rigid body
    ///
    /// Only works for dynamic bodies.
    pub fn set_angular_velocity(&self, world: &mut PhysicsWorld, angular_velocity: Vec3) {
        if let Some(body) = world.get_rigid_body_mut(self.body_handle) {
            body.set_angvel(
                vector![angular_velocity.x, angular_velocity.y, angular_velocity.z],
                true,
            );
        }
    }

    /// Apply a force to the rigid body
    ///
    /// Forces are accumulated and applied during the next physics step.
    /// Only works for dynamic bodies.
    ///
    /// # Arguments
    /// * `world` - The physics world
    /// * `force` - Force vector to apply
    pub fn apply_force(&self, world: &mut PhysicsWorld, force: Vec3) {
        if let Some(body) = world.get_rigid_body_mut(self.body_handle) {
            body.add_force(vector![force.x, force.y, force.z], true);
        }
    }

    /// Apply an impulse to the rigid body
    ///
    /// Impulses cause an immediate change in velocity.
    /// Only works for dynamic bodies.
    ///
    /// # Arguments
    /// * `world` - The physics world
    /// * `impulse` - Impulse vector to apply
    pub fn apply_impulse(&self, world: &mut PhysicsWorld, impulse: Vec3) {
        if let Some(body) = world.get_rigid_body_mut(self.body_handle) {
            body.apply_impulse(vector![impulse.x, impulse.y, impulse.z], true);
        }
    }

    /// Apply a torque to the rigid body
    ///
    /// Torques cause rotational acceleration.
    /// Only works for dynamic bodies.
    ///
    /// # Arguments
    /// * `world` - The physics world
    /// * `torque` - Torque vector to apply
    pub fn apply_torque(&self, world: &mut PhysicsWorld, torque: Vec3) {
        if let Some(body) = world.get_rigid_body_mut(self.body_handle) {
            body.add_torque(vector![torque.x, torque.y, torque.z], true);
        }
    }

    /// Get the body handle
    pub fn body_handle(&self) -> RigidBodyHandle {
        self.body_handle
    }

    /// Get the collider handle
    pub fn collider_handle(&self) -> Option<ColliderHandle> {
        self.collider_handle
    }

    /// Check if this rigid body is still valid in the world
    pub fn is_valid(&self, world: &PhysicsWorld) -> bool {
        world.get_rigid_body(self.body_handle).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cube::Cube;
    use glam::IVec3;

    #[test]
    fn test_dynamic_body_creation() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let body = CubeObject::new_dynamic(&mut world, Vec3::new(0.0, 10.0, 0.0), 1.0);

        assert_eq!(body.position(&world), Vec3::new(0.0, 10.0, 0.0));
        assert!(body.is_valid(&world));
    }

    #[test]
    fn test_velocity_setting() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let body = CubeObject::new_dynamic(&mut world, Vec3::ZERO, 1.0);

        body.set_velocity(&mut world, Vec3::new(1.0, 2.0, 3.0));
        let vel = body.velocity(&world);

        assert_eq!(vel, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_force_application() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut body = CubeObject::new_dynamic(&mut world, Vec3::ZERO, 1.0);

        let collider = ColliderBuilder::ball(0.5).build();
        body.attach_collider(&mut world, collider);

        body.apply_force(&mut world, Vec3::new(0.0, 100.0, 0.0));

        world.step(0.1);

        // Should have moved upward due to applied force
        assert!(body.position(&world).y > 0.0);
    }

    #[test]
    fn test_local_aabb_without_cubebox() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let body = CubeObject::new_dynamic(&mut world, Vec3::ZERO, 1.0);

        // Without a CubeBox, falls back to unit cube
        let aabb = body.local_aabb();
        assert_eq!(aabb.min, Vec3::ZERO);
        assert_eq!(aabb.max, Vec3::ONE);
    }

    #[test]
    fn test_local_aabb_with_cubebox() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut body = CubeObject::new_dynamic(&mut world, Vec3::ZERO, 1.0);

        // Create a 16x30x12 model in depth-5 octree (32³)
        let cube = Cube::Solid(1u8);
        let cubebox = CubeBox::new(cube, IVec3::new(16, 30, 12), 5);
        body.set_cube(Rc::new(cubebox));

        let aabb = body.local_aabb();
        assert_eq!(aabb.min, Vec3::ZERO);
        // Expected: (16/32, 30/32, 12/32) = (0.5, 0.9375, 0.375)
        assert!((aabb.max.x - 0.5).abs() < 0.0001);
        assert!((aabb.max.y - 0.9375).abs() < 0.0001);
        assert!((aabb.max.z - 0.375).abs() < 0.0001);
    }

    #[test]
    fn test_local_aabb_uniform_model() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut body = CubeObject::new_dynamic(&mut world, Vec3::ZERO, 1.0);

        // A 32x32x32 model in depth-5 octree fills entire space
        let cube = Cube::Solid(1u8);
        let cubebox = CubeBox::new(cube, IVec3::splat(32), 5);
        body.set_cube(Rc::new(cubebox));

        let aabb = body.local_aabb();
        assert_eq!(aabb.min, Vec3::ZERO);
        assert_eq!(aabb.max, Vec3::ONE); // 32/32 = 1.0
    }

    #[test]
    fn test_world_aabb_at_origin() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let body = CubeObject::new_dynamic(&mut world, Vec3::ZERO, 1.0);

        let aabb = body.world_aabb(&world);
        assert_eq!(aabb.min, Vec3::ZERO);
        assert_eq!(aabb.max, Vec3::ONE);
    }

    #[test]
    fn test_world_aabb_with_translation() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let body = CubeObject::new_dynamic(&mut world, Vec3::new(10.0, 5.0, 3.0), 1.0);

        let aabb = body.world_aabb(&world);
        assert_eq!(aabb.min, Vec3::new(10.0, 5.0, 3.0));
        assert_eq!(aabb.max, Vec3::new(11.0, 6.0, 4.0));
    }

    #[test]
    fn test_world_aabb_with_scale() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut body = CubeObject::new_dynamic(&mut world, Vec3::ZERO, 1.0);
        body.set_scale(2.0);

        let aabb = body.world_aabb(&world);
        assert_eq!(aabb.min, Vec3::ZERO);
        assert_eq!(aabb.max, Vec3::splat(2.0));
    }

    #[test]
    fn test_scale_getter_setter() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut body = CubeObject::new_dynamic(&mut world, Vec3::ZERO, 1.0);

        assert_eq!(body.scale(), 1.0);

        body.set_scale(3.5);
        assert_eq!(body.scale(), 3.5);
    }

    #[test]
    fn test_intersects_aabb() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let body = CubeObject::new_dynamic(&mut world, Vec3::ZERO, 1.0);

        // Overlapping AABB
        let overlapping = Aabb::new(Vec3::splat(0.5), Vec3::splat(1.5));
        assert!(body.intersects_aabb(&world, &overlapping));

        // Non-overlapping AABB
        let distant = Aabb::new(Vec3::splat(10.0), Vec3::splat(11.0));
        assert!(!body.intersects_aabb(&world, &distant));
    }

    #[test]
    fn test_world_aabb_with_cubebox_and_scale() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut body = CubeObject::new_dynamic(&mut world, Vec3::new(10.0, 0.0, 10.0), 1.0);

        // 16x30x12 avatar in depth-5 octree (32³), scaled by 2.0
        let cube = Cube::Solid(1u8);
        let cubebox = CubeBox::new(cube, IVec3::new(16, 30, 12), 5);
        body.set_cube(Rc::new(cubebox));
        body.set_scale(2.0);

        let aabb = body.world_aabb(&world);
        // Local AABB: (0,0,0) to (0.5, 0.9375, 0.375)
        // Scaled: (0,0,0) to (1.0, 1.875, 0.75)
        // Translated: (10, 0, 10) to (11.0, 1.875, 10.75)
        assert_eq!(aabb.min, Vec3::new(10.0, 0.0, 10.0));
        assert!((aabb.max.x - 11.0).abs() < 0.0001);
        assert!((aabb.max.y - 1.875).abs() < 0.0001);
        assert!((aabb.max.z - 10.75).abs() < 0.0001);
    }
}
