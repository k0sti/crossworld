use crate::config::SpawningConfig;
use crate::models::{SpawnedObject, VoxModel};
use crossworld_physics::rapier3d::prelude::*;
use crossworld_physics::{CubeObject, PhysicsWorld};
use cube::CubeBox;
use glam::Vec3;
use rand::Rng;
use std::rc::Rc;

/// Camera object with physics body for first-person movement
pub struct CameraObject {
    pub body_handle: RigidBodyHandle,
    pub collider_handle: ColliderHandle,
}

impl CameraObject {
    /// Create a new camera object with physics
    ///
    /// # Arguments
    /// * `physics_world` - Physics world to add camera to
    /// * `position` - Initial position
    /// * `height` - Camera capsule height
    /// * `radius` - Camera capsule radius
    pub fn new(physics_world: &mut PhysicsWorld, position: Vec3, height: f32, radius: f32) -> Self {
        // Use kinematic body for direct control with collision detection
        let rb = RigidBodyBuilder::kinematic_position_based()
            .translation(vector![position.x, position.y, position.z])
            .build();
        let rb_handle = physics_world.add_rigid_body(rb);

        // Create capsule collider for the camera body
        let capsule_half_height = (height / 2.0 - radius).max(0.0);
        let collider = ColliderBuilder::capsule_y(capsule_half_height, radius)
            .friction(0.0)
            .restitution(0.0)
            .build();
        let coll_handle = physics_world.add_collider(collider, rb_handle);

        Self {
            body_handle: rb_handle,
            collider_handle: coll_handle,
        }
    }

    /// Get current position from physics body
    pub fn position(&self, physics_world: &PhysicsWorld) -> Vec3 {
        if let Some(body) = physics_world.get_rigid_body(self.body_handle) {
            let pos = body.translation();
            Vec3::new(pos.x, pos.y, pos.z)
        } else {
            Vec3::ZERO
        }
    }

    /// Move the camera with physics-based collision
    ///
    /// # Arguments
    /// * `physics_world` - Physics world
    /// * `velocity` - Desired velocity
    /// * `dt` - Delta time
    /// * `gravity` - Gravity value (negative for downward)
    pub fn move_with_velocity(
        &mut self,
        physics_world: &mut PhysicsWorld,
        velocity: Vec3,
        dt: f32,
        _gravity: f32,
    ) {
        let current_pos = self.position(physics_world);

        // For now, simple direct movement (no gravity - fly mode)
        // This allows free movement including up/down with F/V keys
        let target_pos = current_pos + velocity * dt;

        // Set the kinematic body's next position
        if let Some(body) = physics_world.get_rigid_body_mut(self.body_handle) {
            body.set_next_kinematic_translation(vector![target_pos.x, target_pos.y, target_pos.z]);
        }
    }

    /// Teleport camera to a specific position
    pub fn set_position(&mut self, physics_world: &mut PhysicsWorld, position: Vec3) {
        if let Some(body) = physics_world.get_rigid_body_mut(self.body_handle) {
            body.set_translation(vector![position.x, position.y, position.z], true);
        }
    }
}

/// Spawn dynamic cube objects with physics
pub fn spawn_cube_objects(
    config: &SpawningConfig,
    models: &[VoxModel],
    physics_world: &mut PhysicsWorld,
) -> Vec<SpawnedObject> {
    let mut objects = Vec::new();
    let mut rng = rand::rng();

    for i in 0..config.spawn_count {
        // Random position centered at origin (0, 0, 0)
        // X and Z: random within spawn_radius of center
        // Y: random between min_height and max_height (above ground which is at y < 0)
        let x = rng.random_range(-config.spawn_radius..config.spawn_radius);
        let y = rng.random_range(config.min_height..config.max_height);
        let z = rng.random_range(-config.spawn_radius..config.spawn_radius);

        // Random model
        let model = &models[i as usize % models.len()];

        // Calculate effective size with scale exponent
        // actual_scale = 2^scale_exp (positive = bigger, negative = smaller)
        let scale_factor = 2.0_f32.powi(model.scale_exp);
        let base_scale = config.object_size * scale_factor;

        // Create physics object using the physics crate's CubeObject
        let mut physics_obj = CubeObject::new_dynamic(
            physics_world,
            Vec3::new(x, y, z),
            1.0, // mass
        );

        // Create CubeBox from the model for accurate AABB calculations
        let cubebox = CubeBox::new(model.cube().clone(), model.size(), model.depth());
        physics_obj.set_cube(Rc::new(cubebox));
        physics_obj.set_scale(base_scale);

        // Create collider - use CubeBox dimensions for accurate bounding box
        //
        // PERFORMANCE NOTE: VoxelColliderBuilder::from_cube() generates compound colliders
        // with one cuboid per exposed voxel face. For depth 4-7 models, this creates
        // thousands of collision primitives per object. With 100 objects, this results
        // in hundreds of thousands of collision shapes, causing extremely slow physics.
        //
        // Solution: Use simple box colliders sized to CubeBox bounds - visual detail
        // doesn't need to match collision shape exactly.
        let octree_size = (1 << model.depth()) as f32;
        let model_size = model.size();
        let half_extents = Vec3::new(
            (model_size.x as f32 / octree_size) * base_scale * 0.5,
            (model_size.y as f32 / octree_size) * base_scale * 0.5,
            (model_size.z as f32 / octree_size) * base_scale * 0.5,
        );
        let collider = ColliderBuilder::cuboid(half_extents.x, half_extents.y, half_extents.z)
            .density(1.0)
            .friction(0.5)
            .restitution(0.3)
            .build();
        physics_obj.attach_collider(physics_world, collider);

        objects.push(SpawnedObject {
            physics: physics_obj,
            model_name: model.name.clone(),
            scale_exp: model.scale_exp,
            is_colliding_world: false,
            collision_aabb: None,
        });
    }

    println!("Spawned {} dynamic cubes", objects.len());
    objects
}
