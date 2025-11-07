use glam::Vec3;
use rapier3d::prelude::*;

/// Physics simulation world
///
/// Manages all rigid bodies, colliders, and physics simulation state.
/// This is a wrapper around Rapier's physics pipeline.
pub struct PhysicsWorld {
    pub(crate) rigid_body_set: RigidBodySet,
    pub(crate) collider_set: ColliderSet,
    pub(crate) impulse_joint_set: ImpulseJointSet,
    pub(crate) multibody_joint_set: MultibodyJointSet,
    pub(crate) integration_parameters: IntegrationParameters,
    pub(crate) physics_pipeline: PhysicsPipeline,
    pub(crate) island_manager: IslandManager,
    pub(crate) broad_phase: DefaultBroadPhase,
    pub(crate) narrow_phase: NarrowPhase,
    pub(crate) ccd_solver: CCDSolver,
    gravity: Vector<Real>,
}

impl PhysicsWorld {
    /// Create a new physics world with specified gravity
    ///
    /// # Arguments
    /// * `gravity` - Gravity vector (e.g., Vec3::new(0.0, -9.81, 0.0))
    pub fn new(gravity: Vec3) -> Self {
        Self {
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            ccd_solver: CCDSolver::new(),
            gravity: vector![gravity.x, gravity.y, gravity.z],
        }
    }

    /// Step the physics simulation forward by dt seconds
    ///
    /// # Arguments
    /// * `dt` - Time step in seconds (typically 1/60 = 0.016666...)
    pub fn step(&mut self, dt: f32) {
        self.integration_parameters.dt = dt;

        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            &(),
            &(),
        );
    }

    /// Add a rigid body to the world
    ///
    /// # Arguments
    /// * `body` - The rigid body to add
    ///
    /// # Returns
    /// Handle to the added rigid body
    pub fn add_rigid_body(&mut self, body: RigidBody) -> RigidBodyHandle {
        self.rigid_body_set.insert(body)
    }

    /// Remove a rigid body from the world
    ///
    /// Also removes all associated colliders automatically.
    ///
    /// # Arguments
    /// * `handle` - Handle to the rigid body to remove
    pub fn remove_rigid_body(&mut self, handle: RigidBodyHandle) {
        self.rigid_body_set.remove(
            handle,
            &mut self.island_manager,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            true,
        );
    }

    /// Add a collider to the world, attached to a rigid body
    ///
    /// # Arguments
    /// * `collider` - The collider to add
    /// * `parent` - Handle to the parent rigid body
    ///
    /// # Returns
    /// Handle to the added collider
    pub fn add_collider(
        &mut self,
        collider: Collider,
        parent: RigidBodyHandle,
    ) -> ColliderHandle {
        self.collider_set.insert_with_parent(collider, parent, &mut self.rigid_body_set)
    }

    /// Get a reference to a rigid body
    pub fn get_rigid_body(&self, handle: RigidBodyHandle) -> Option<&RigidBody> {
        self.rigid_body_set.get(handle)
    }

    /// Get a mutable reference to a rigid body
    pub fn get_rigid_body_mut(&mut self, handle: RigidBodyHandle) -> Option<&mut RigidBody> {
        self.rigid_body_set.get_mut(handle)
    }

    /// Get the current gravity vector
    pub fn gravity(&self) -> Vec3 {
        Vec3::new(self.gravity.x, self.gravity.y, self.gravity.z)
    }

    /// Set the gravity vector
    pub fn set_gravity(&mut self, gravity: Vec3) {
        self.gravity = vector![gravity.x, gravity.y, gravity.z];
    }

    // TODO: Implement raycast once we understand the new Rapier API better
    // /// Perform a raycast through the physics world
    // pub fn raycast(...) -> Option<(ColliderHandle, f32)> { ... }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_creation() {
        let world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        assert_eq!(world.gravity(), Vec3::new(0.0, -9.81, 0.0));
    }

    #[test]
    fn test_add_rigid_body() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        let body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 10.0, 0.0])
            .build();

        let handle = world.add_rigid_body(body);

        let body_ref = world.get_rigid_body(handle).unwrap();
        assert_eq!(body_ref.translation().y, 10.0);
    }

    #[test]
    fn test_gravity_simulation() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        let body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 10.0, 0.0])
            .build();

        let handle = world.add_rigid_body(body);

        let collider = ColliderBuilder::ball(0.5).build();
        world.add_collider(collider, handle);

        // Simulate for 1 second
        world.step(1.0);

        let body_ref = world.get_rigid_body(handle).unwrap();
        // Should have fallen due to gravity
        assert!(body_ref.translation().y < 10.0);
    }
}
