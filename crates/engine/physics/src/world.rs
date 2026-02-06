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
    pub fn add_collider(&mut self, collider: Collider, parent: RigidBodyHandle) -> ColliderHandle {
        self.collider_set
            .insert_with_parent(collider, parent, &mut self.rigid_body_set)
    }

    /// Remove a collider from the world
    ///
    /// # Arguments
    /// * `handle` - Handle to the collider to remove
    ///
    /// # Returns
    /// The removed collider, or None if not found
    pub fn remove_collider(&mut self, handle: ColliderHandle) -> Option<Collider> {
        self.collider_set.remove(
            handle,
            &mut self.island_manager,
            &mut self.rigid_body_set,
            true,
        )
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

    /// Get the number of rigid bodies in the world
    pub fn rigid_body_count(&self) -> usize {
        self.rigid_body_set.len()
    }

    /// Get the number of colliders in the world
    pub fn collider_count(&self) -> usize {
        self.collider_set.len()
    }

    /// Perform a raycast through the physics world
    ///
    /// # Arguments
    /// * `origin` - Starting point of the ray
    /// * `direction` - Direction vector (will be normalized)
    /// * `max_distance` - Maximum distance to check
    /// * `solid_only` - If true, ignores sensor colliders
    /// * `exclude_collider` - Optional collider to exclude from the raycast
    ///
    /// # Returns
    /// Optional tuple of (ColliderHandle, hit_distance, hit_point, hit_normal)
    pub fn cast_ray(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        solid_only: bool,
    ) -> Option<(ColliderHandle, f32, Vec3, Vec3)> {
        self.cast_ray_with_exclusion(origin, direction, max_distance, solid_only, None)
    }

    /// Add a static collider (no parent body, fixed in place)
    ///
    /// Creates a fixed rigid body at the origin and attaches the collider to it.
    /// Useful for terrain and other immovable world geometry.
    ///
    /// # Arguments
    /// * `collider` - The collider to add
    ///
    /// # Returns
    /// Tuple of (RigidBodyHandle, ColliderHandle)
    pub fn add_static_collider(&mut self, collider: Collider) -> (RigidBodyHandle, ColliderHandle) {
        let body = RigidBodyBuilder::fixed().build();
        let body_handle = self.rigid_body_set.insert(body);
        let collider_handle =
            self.collider_set
                .insert_with_parent(collider, body_handle, &mut self.rigid_body_set);
        (body_handle, collider_handle)
    }

    /// Update a terrain collider with new geometry
    ///
    /// Removes the old collider and adds a new one with the updated shape.
    /// The rigid body remains unchanged.
    ///
    /// # Arguments
    /// * `body_handle` - Handle to the fixed terrain body
    /// * `old_collider` - Handle to the old collider to remove
    /// * `new_collider` - New collider with updated geometry
    ///
    /// # Returns
    /// Handle to the new collider
    pub fn update_terrain_collider(
        &mut self,
        body_handle: RigidBodyHandle,
        old_collider: ColliderHandle,
        new_collider: Collider,
    ) -> ColliderHandle {
        // Remove old collider
        self.collider_set.remove(
            old_collider,
            &mut self.island_manager,
            &mut self.rigid_body_set,
            true,
        );

        // Add new collider to same body
        self.collider_set
            .insert_with_parent(new_collider, body_handle, &mut self.rigid_body_set)
    }

    /// Check if a collider is currently in contact with any other collider
    ///
    /// Uses the narrow phase contact data to determine if there are active contacts.
    ///
    /// # Arguments
    /// * `collider_handle` - Handle to the collider to check
    ///
    /// # Returns
    /// `true` if the collider has at least one active contact
    pub fn is_colliding(&self, collider_handle: ColliderHandle) -> bool {
        // Iterate through all contact pairs involving this collider
        for contact_pair in self.narrow_phase.contact_pairs_with(collider_handle) {
            // Check if there are any active contact manifolds
            if contact_pair.has_any_active_contact {
                return true;
            }
        }
        false
    }

    /// Perform a raycast through the physics world with an optional collider exclusion
    ///
    /// # Arguments
    /// * `origin` - Starting point of the ray
    /// * `direction` - Direction vector (will be normalized)
    /// * `max_distance` - Maximum distance to check
    /// * `solid_only` - If true, ignores sensor colliders
    /// * `exclude_collider` - Optional collider to exclude from the raycast
    ///
    /// # Returns
    /// Optional tuple of (ColliderHandle, hit_distance, hit_point, hit_normal)
    pub fn cast_ray_with_exclusion(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        solid_only: bool,
        exclude_collider: Option<ColliderHandle>,
    ) -> Option<(ColliderHandle, f32, Vec3, Vec3)> {
        let dir = direction.normalize();
        let ray = Ray::new(
            point![origin.x, origin.y, origin.z],
            vector![dir.x, dir.y, dir.z],
        );

        // Simple raycast using collider_set (manually filtering sensors)
        let mut closest_hit: Option<(ColliderHandle, f32, Vec3)> = None;

        for (handle, collider) in self.collider_set.iter() {
            // Skip excluded collider
            if let Some(excluded) = exclude_collider {
                if handle == excluded {
                    continue;
                }
            }

            if let Some(toi) = collider.shape().cast_ray_and_get_normal(
                collider.position(),
                &ray,
                max_distance,
                solid_only,
            ) {
                let distance = toi.time_of_impact;
                if closest_hit.is_none() || distance < closest_hit.as_ref().unwrap().1 {
                    let _hit_point = ray.point_at(distance);
                    closest_hit = Some((
                        handle,
                        distance,
                        Vec3::new(toi.normal.x, toi.normal.y, toi.normal.z),
                    ));
                }
            }
        }

        closest_hit.map(|(handle, distance, normal)| {
            let hit_point = ray.point_at(distance);
            (
                handle,
                distance,
                Vec3::new(hit_point.x, hit_point.y, hit_point.z),
                normal,
            )
        })
    }
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

    #[test]
    fn test_dynamic_body_stops_at_ground() {
        use nalgebra::Unit;

        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        // Create ground plane at Y=0
        let ground_body = RigidBodyBuilder::fixed()
            .translation(vector![0.0, 0.0, 0.0])
            .build();
        let ground_handle = world.add_rigid_body(ground_body);

        let ground_normal = Unit::new_normalize(vector![0.0, 1.0, 0.0]);
        let ground_collider = ColliderBuilder::halfspace(ground_normal)
            .friction(0.5)
            .restitution(0.0)
            .build();
        world.add_collider(ground_collider, ground_handle);

        // Create a dynamic box at Y=5
        let box_body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 5.0, 0.0])
            .build();
        let box_handle = world.add_rigid_body(box_body);

        let box_collider = ColliderBuilder::cuboid(0.5, 0.5, 0.5) // 1x1x1 box
            .density(1.0)
            .friction(0.5)
            .restitution(0.0)
            .build();
        world.add_collider(box_collider, box_handle);

        // Simulate for 5 seconds (300 frames at 60fps)
        for _ in 0..300 {
            world.step(1.0 / 60.0);
        }

        let box_ref = world.get_rigid_body(box_handle).unwrap();
        let final_y = box_ref.translation().y;

        // Box should have landed on ground - center at Y=0.5 (half-height above ground)
        assert!(
            (final_y - 0.5).abs() < 0.1,
            "Box should rest at Y=0.5, got Y={:.2}",
            final_y
        );

        // Should not have fallen through
        assert!(final_y > 0.0, "Box fell through ground! Y={:.2}", final_y);
    }
}
