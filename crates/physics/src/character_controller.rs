use glam::{Quat, Vec3};
use nalgebra::{Quaternion, UnitQuaternion};
use rapier3d::prelude::*;

use crate::world::PhysicsWorld;

/// Configuration for character controller
#[derive(Debug, Clone, Copy)]
pub struct CharacterControllerConfig {
    /// Height of the character capsule
    pub height: f32,
    /// Radius of the character capsule
    pub radius: f32,
    /// Maximum height of steps the character can climb
    pub step_height: f32,
    /// Maximum slope angle in degrees that the character can walk on
    pub max_slope_angle: f32,
    /// Gravity acceleration (positive value, applied downward)
    pub gravity: f32,
    /// Jump impulse strength
    pub jump_impulse: f32,
    /// Distance to check for ground below character
    pub ground_check_distance: f32,
}

impl Default for CharacterControllerConfig {
    fn default() -> Self {
        Self {
            height: 1.8,
            radius: 0.3,
            step_height: 0.5,
            max_slope_angle: 45.0,
            gravity: 9.8,
            jump_impulse: 5.0,
            ground_check_distance: 0.1,
        }
    }
}

/// Character controller for kinematic avatar movement
///
/// Provides physics-based character movement with collision detection,
/// ground snapping, slope handling, and jump mechanics.
pub struct CharacterController {
    body_handle: RigidBodyHandle,
    collider_handle: ColliderHandle,
    config: CharacterControllerConfig,

    // State
    is_grounded: bool,
    ground_normal: Vec3,
    vertical_velocity: f32,
}

impl CharacterController {
    /// Create a new character controller
    ///
    /// # Arguments
    /// * `world` - Physics world to add the character to
    /// * `position` - Initial position
    /// * `config` - Character configuration
    pub fn new(
        world: &mut PhysicsWorld,
        position: Vec3,
        config: CharacterControllerConfig,
    ) -> Self {
        // Create kinematic rigid body
        let body = RigidBodyBuilder::kinematic_position_based()
            .translation(vector![position.x, position.y, position.z])
            .build();

        let body_handle = world.add_rigid_body(body);

        // Create capsule collider
        // Capsule half-height is total height minus the two hemisphere caps
        let capsule_half_height = (config.height / 2.0 - config.radius).max(0.0);
        let collider = ColliderBuilder::capsule_y(capsule_half_height, config.radius)
            .friction(0.0) // No friction for smooth movement
            .restitution(0.0) // No bounce
            .build();

        let collider_handle = world.add_collider(collider, body_handle);

        Self {
            body_handle,
            collider_handle,
            config,
            is_grounded: false,
            ground_normal: Vec3::Y,
            vertical_velocity: 0.0,
        }
    }

    /// Move the character with the given horizontal velocity
    ///
    /// This applies the velocity, handles collision response, applies gravity,
    /// and updates ground state.
    ///
    /// # Arguments
    /// * `world` - Physics world
    /// * `horizontal_velocity` - Desired horizontal velocity (Y component is ignored)
    /// * `dt` - Time step in seconds
    pub fn move_with_velocity(
        &mut self,
        world: &mut PhysicsWorld,
        horizontal_velocity: Vec3,
        dt: f32,
    ) {
        // Apply gravity to vertical velocity
        if !self.is_grounded {
            self.vertical_velocity -= self.config.gravity * dt;
        }

        // Combine horizontal and vertical velocity
        let velocity = Vec3::new(
            horizontal_velocity.x,
            self.vertical_velocity,
            horizontal_velocity.z,
        );

        // Calculate target position
        let current_pos = self.position(world);
        let mut target_pos = current_pos + velocity * dt;

        // Check for ground collision before moving
        // Raycast from current position downward to find ground
        let capsule_half_height = self.config.height / 2.0;
        let ray_origin = current_pos;
        let ray_dir = Vec3::NEG_Y;

        // Use a long raycast to find any ground below
        let max_ray_distance = 100.0;

        if let Some((_handle, distance, _point, normal)) = world.cast_ray_with_exclusion(
            ray_origin,
            ray_dir,
            max_ray_distance,
            true,
            Some(self.collider_handle),
        ) {
            // Ground is at: current_pos.y - distance
            // Character bottom should be at: ground_y + capsule_half_height
            let ground_y = current_pos.y - distance;
            let min_y = ground_y + capsule_half_height;

            // If target position would put character below ground, clamp it
            if target_pos.y < min_y {
                target_pos.y = min_y;

                // We hit ground - update state
                let distance_from_bottom = distance - capsule_half_height;
                if distance_from_bottom <= self.config.ground_check_distance {
                    self.is_grounded = true;
                    self.ground_normal = normal;

                    // Reset vertical velocity when landing
                    if self.vertical_velocity < 0.0 {
                        self.vertical_velocity = 0.0;
                    }
                }
            } else {
                // Not hitting ground yet
                let distance_from_bottom = distance - capsule_half_height;
                if distance_from_bottom <= self.config.ground_check_distance {
                    self.is_grounded = true;
                    self.ground_normal = normal;
                } else {
                    self.is_grounded = false;
                    self.ground_normal = Vec3::Y;
                }
            }
        } else {
            // No ground detected
            self.is_grounded = false;
            self.ground_normal = Vec3::Y;
        }

        // Update position
        if let Some(body) = world.get_rigid_body_mut(self.body_handle) {
            body.set_next_kinematic_translation(vector![target_pos.x, target_pos.y, target_pos.z]);
        }
    }

    /// Attempt to jump if grounded
    pub fn jump(&mut self) {
        if self.is_grounded {
            self.vertical_velocity = self.config.jump_impulse;
            self.is_grounded = false; // Immediately set to false to prevent double jumps
        }
    }

    /// Get the current position of the character
    pub fn position(&self, world: &PhysicsWorld) -> Vec3 {
        if let Some(body) = world.get_rigid_body(self.body_handle) {
            let pos = body.translation();
            Vec3::new(pos.x, pos.y, pos.z)
        } else {
            Vec3::ZERO
        }
    }

    /// Get the current rotation of the character
    pub fn rotation(&self, world: &PhysicsWorld) -> Quat {
        if let Some(body) = world.get_rigid_body(self.body_handle) {
            let rot = body.rotation();
            Quat::from_xyzw(rot.i, rot.j, rot.k, rot.w)
        } else {
            Quat::IDENTITY
        }
    }

    /// Get the current velocity of the character
    pub fn velocity(&self, world: &PhysicsWorld) -> Vec3 {
        if let Some(body) = world.get_rigid_body(self.body_handle) {
            let vel = body.linvel();
            Vec3::new(vel.x, vel.y, vel.z)
        } else {
            Vec3::ZERO
        }
    }

    /// Check if the character is on the ground
    pub fn is_grounded(&self) -> bool {
        self.is_grounded
    }

    /// Get the ground normal vector
    pub fn ground_normal(&self) -> Vec3 {
        self.ground_normal
    }

    /// Get the vertical velocity
    pub fn vertical_velocity(&self) -> f32 {
        self.vertical_velocity
    }

    /// Set the character's position directly (e.g., for teleportation)
    ///
    /// # Arguments
    /// * `world` - Physics world
    /// * `position` - New position
    pub fn set_position(&mut self, world: &mut PhysicsWorld, position: Vec3) {
        if let Some(body) = world.get_rigid_body_mut(self.body_handle) {
            body.set_translation(vector![position.x, position.y, position.z], true);
        }
    }

    /// Set the character's rotation
    ///
    /// # Arguments
    /// * `world` - Physics world
    /// * `rotation` - New rotation quaternion
    pub fn set_rotation(&mut self, world: &mut PhysicsWorld, rotation: Quat) {
        if let Some(body) = world.get_rigid_body_mut(self.body_handle) {
            body.set_rotation(
                UnitQuaternion::from_quaternion(Quaternion::new(
                    rotation.w, rotation.x, rotation.y, rotation.z,
                )),
                true,
            );
        }
    }

    /// Get the rigid body handle
    pub fn body_handle(&self) -> RigidBodyHandle {
        self.body_handle
    }

    /// Get the collider handle
    pub fn collider_handle(&self) -> ColliderHandle {
        self.collider_handle
    }

    /// Destroy the character controller and remove it from the physics world
    ///
    /// # Arguments
    /// * `world` - Physics world
    pub fn destroy(self, world: &mut PhysicsWorld) {
        world.remove_rigid_body(self.body_handle);
        // Collider is automatically removed with the rigid body
    }

}

/// Result of a raycast query
#[derive(Debug, Clone)]
pub struct RaycastHit {
    pub collider: ColliderHandle,
    pub distance: f32,
    pub point: Vec3,
    pub normal: Vec3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_creation() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let config = CharacterControllerConfig::default();

        let controller = CharacterController::new(&mut world, Vec3::new(0.0, 10.0, 0.0), config);

        assert_eq!(controller.position(&world), Vec3::new(0.0, 10.0, 0.0));
        assert!(!controller.is_grounded());
    }

    #[test]
    fn test_character_falls_with_gravity() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let config = CharacterControllerConfig::default();

        let mut controller =
            CharacterController::new(&mut world, Vec3::new(0.0, 10.0, 0.0), config);

        let initial_y = controller.position(&world).y;

        // Simulate for 1 second (60 frames)
        for _ in 0..60 {
            controller.move_with_velocity(&mut world, Vec3::ZERO, 1.0 / 60.0);
            world.step(1.0 / 60.0);
        }

        let final_y = controller.position(&world).y;

        // Should have fallen
        assert!(final_y < initial_y);
    }

    #[test]
    fn test_character_jump() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let config = CharacterControllerConfig::default();

        let mut controller = CharacterController::new(&mut world, Vec3::new(0.0, 0.0, 0.0), config);

        // Manually set grounded for test
        controller.is_grounded = true;

        // Jump
        controller.jump();

        // Should have positive vertical velocity
        assert!(controller.vertical_velocity() > 0.0);
        assert!(!controller.is_grounded());
    }

    #[test]
    fn test_character_horizontal_movement() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let config = CharacterControllerConfig::default();

        let mut controller = CharacterController::new(&mut world, Vec3::new(0.0, 0.0, 0.0), config);

        let initial_pos = controller.position(&world);

        // Move forward
        let velocity = Vec3::new(5.0, 0.0, 0.0);
        controller.move_with_velocity(&mut world, velocity, 0.1);

        // Step the physics world to apply the movement
        world.step(0.1);

        let final_pos = controller.position(&world);

        // Should have moved in X direction
        assert!(
            final_pos.x > initial_pos.x,
            "Expected character to move from {:.2} to > {:.2}, but got {:.2}",
            initial_pos.x,
            initial_pos.x,
            final_pos.x
        );
    }

    #[test]
    fn test_character_stops_at_ground_plane() {
        use nalgebra::Unit;
        use rapier3d::prelude::*;

        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));

        // Create a ground plane at Y=0
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

        // Create character starting at Y=5 (above ground)
        let config = CharacterControllerConfig::default(); // height=1.8, so capsule half-height=0.9
        let mut controller = CharacterController::new(&mut world, Vec3::new(0.0, 5.0, 0.0), config);

        // Simulate for 5 seconds (should be enough to fall and land)
        for _ in 0..300 {
            controller.move_with_velocity(&mut world, Vec3::ZERO, 1.0 / 60.0);
            world.step(1.0 / 60.0);
        }

        let final_pos = controller.position(&world);

        // Character should have landed and be grounded
        assert!(
            controller.is_grounded(),
            "Character should be grounded after falling"
        );

        // Character center should be at height/2 above ground (0.9 for default config)
        // Allow some tolerance for the ground check distance
        let expected_y = config.height / 2.0;
        assert!(
            (final_pos.y - expected_y).abs() < 0.2,
            "Character Y position should be ~{:.2}, but got {:.2}",
            expected_y,
            final_pos.y
        );

        // Character should not have fallen through the ground
        assert!(
            final_pos.y > 0.0,
            "Character should not fall through ground (Y={:.2})",
            final_pos.y
        );
    }

    #[test]
    fn test_character_lands_on_box() {
        use rapier3d::prelude::*;

        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));

        // Create a box at Y=0 (top surface at Y=0.5)
        let box_body = RigidBodyBuilder::fixed()
            .translation(vector![0.0, 0.0, 0.0])
            .build();
        let box_handle = world.add_rigid_body(box_body);

        let box_collider = ColliderBuilder::cuboid(10.0, 0.5, 10.0) // Half extents: 20x1x20 box
            .friction(0.5)
            .build();
        world.add_collider(box_collider, box_handle);

        // Create character starting at Y=10 (above box)
        let config = CharacterControllerConfig::default();
        let mut controller =
            CharacterController::new(&mut world, Vec3::new(0.0, 10.0, 0.0), config);

        // Simulate for 5 seconds
        for _ in 0..300 {
            controller.move_with_velocity(&mut world, Vec3::ZERO, 1.0 / 60.0);
            world.step(1.0 / 60.0);
        }

        let final_pos = controller.position(&world);

        // Character should land on top of box (box top is at Y=0.5)
        // Character center should be at box_top + capsule_half_height = 0.5 + 0.9 = 1.4
        let expected_y = 0.5 + config.height / 2.0;
        assert!(
            (final_pos.y - expected_y).abs() < 0.2,
            "Character Y position should be ~{:.2}, but got {:.2}",
            expected_y,
            final_pos.y
        );

        assert!(
            controller.is_grounded(),
            "Character should be grounded on box"
        );
    }
}
