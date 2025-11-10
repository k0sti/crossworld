use crate::{
    character_controller::{CharacterController, CharacterControllerConfig},
    collider::{
        create_box_collider, create_capsule_collider, create_sphere_collider, VoxelColliderBuilder,
    },
    world::PhysicsWorld,
};
use glam::Vec3;
use nalgebra::{Quaternion, Unit, UnitQuaternion};
use rapier3d::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmPhysicsWorld {
    inner: RefCell<PhysicsWorld>,
    characters: RefCell<HashMap<u32, CharacterController>>,
    next_character_id: RefCell<u32>,
}

#[wasm_bindgen]
impl WasmPhysicsWorld {
    /// Create new physics world
    ///
    /// # Arguments
    /// * `gravity_x`, `gravity_y`, `gravity_z` - Gravity vector components
    #[wasm_bindgen(constructor)]
    pub fn new(gravity_x: f32, gravity_y: f32, gravity_z: f32) -> Self {
        Self {
            inner: RefCell::new(PhysicsWorld::new(Vec3::new(
                gravity_x, gravity_y, gravity_z,
            ))),
            characters: RefCell::new(HashMap::new()),
            next_character_id: RefCell::new(1),
        }
    }

    /// Step simulation forward by dt seconds
    ///
    /// # Arguments
    /// * `dt` - Time step in seconds (typically 1/60 = 0.016666...)
    #[wasm_bindgen(js_name = step)]
    pub fn step(&self, dt: f32) {
        self.inner.borrow_mut().step(dt);
    }

    /// Add rigid body from voxel cube (CSM format)
    ///
    // TODO: Implement add_cube method for voxel objects
    // Currently commented out to allow WASM build
    // /// # Arguments
    // /// * `cube` - Cube object
    // /// * `config` - Object configuration
    // ///
    // /// # Returns
    // /// Object ID for the created body
    // #[wasm_bindgen(js_name = addVoxelBody)]
    // pub fn add_cube(&self, cube: &Cube, config: &ObjectConfig) -> Result<u32, JsValue> {
    //     use std::rc::Rc;
    //     //TODO Implement
    // }

    /// Get list of all object IDs
    ///
    /// # Returns
    /// Array of object IDs
    #[wasm_bindgen(js_name = getAllObjects)]
    pub fn get_all_objects(&self) -> Vec<u32> {
        let world = self.inner.borrow();
        world
            .rigid_body_set
            .iter()
            .map(|(handle, _)| handle.into_raw_parts().0)
            .collect()
    }

    /// Get object position
    ///
    /// # Arguments
    /// * `object_id` - Object ID returned from add methods
    ///
    /// # Returns
    /// Array [x, y, z]
    #[wasm_bindgen(js_name = getPosition)]
    pub fn get_position(&self, object_id: u32) -> Vec<f32> {
        let world = self.inner.borrow();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.get_rigid_body(handle) {
            let pos = body.translation();
            vec![pos.x, pos.y, pos.z]
        } else {
            vec![0.0, 0.0, 0.0]
        }
    }

    /// Get object rotation as quaternion
    ///
    /// # Arguments
    /// * `object_id` - Object ID returned from add methods
    ///
    /// # Returns
    /// Array [x, y, z, w] (quaternion)
    #[wasm_bindgen(js_name = getRotation)]
    pub fn get_rotation(&self, object_id: u32) -> Vec<f32> {
        let world = self.inner.borrow();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.get_rigid_body(handle) {
            let rot = body.rotation();
            vec![rot.i, rot.j, rot.k, rot.w]
        } else {
            vec![0.0, 0.0, 0.0, 1.0]
        }
    }

    /// Get object linear velocity
    ///
    /// # Arguments
    /// * `object_id` - Object ID returned from add methods
    ///
    /// # Returns
    /// Array [x, y, z]
    #[wasm_bindgen(js_name = getVelocity)]
    pub fn get_velocity(&self, object_id: u32) -> Vec<f32> {
        let world = self.inner.borrow();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.get_rigid_body(handle) {
            let vel = body.linvel();
            vec![vel.x, vel.y, vel.z]
        } else {
            vec![0.0, 0.0, 0.0]
        }
    }

    /// Get object angular velocity
    ///
    /// # Arguments
    /// * `object_id` - Object ID returned from add methods
    ///
    /// # Returns
    /// Array [x, y, z] (axis-angle representation)
    #[wasm_bindgen(js_name = getAngularVelocity)]
    pub fn get_angular_velocity(&self, object_id: u32) -> Vec<f32> {
        let world = self.inner.borrow();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.get_rigid_body(handle) {
            let angvel = body.angvel();
            vec![angvel.x, angvel.y, angvel.z]
        } else {
            vec![0.0, 0.0, 0.0]
        }
    }

    /// Apply force to object
    ///
    /// Forces are accumulated and applied during the next physics step.
    ///
    /// # Arguments
    /// * `object_id` - Object ID
    /// * `force_x`, `force_y`, `force_z` - Force vector components
    #[wasm_bindgen(js_name = applyForce)]
    pub fn apply_force(&self, object_id: u32, force_x: f32, force_y: f32, force_z: f32) {
        let mut world = self.inner.borrow_mut();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.get_rigid_body_mut(handle) {
            body.add_force(vector![force_x, force_y, force_z], true);
        }
    }

    /// Apply impulse to object
    ///
    /// Impulses cause immediate velocity change.
    ///
    /// # Arguments
    /// * `object_id` - Object ID
    /// * `impulse_x`, `impulse_y`, `impulse_z` - Impulse vector components
    #[wasm_bindgen(js_name = applyImpulse)]
    pub fn apply_impulse(&self, object_id: u32, impulse_x: f32, impulse_y: f32, impulse_z: f32) {
        let mut world = self.inner.borrow_mut();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.get_rigid_body_mut(handle) {
            body.apply_impulse(vector![impulse_x, impulse_y, impulse_z], true);
        }
    }

    /// Apply torque to object
    ///
    /// # Arguments
    /// * `object_id` - Object ID
    /// * `torque_x`, `torque_y`, `torque_z` - Torque vector components
    #[wasm_bindgen(js_name = applyTorque)]
    pub fn apply_torque(&self, object_id: u32, torque_x: f32, torque_y: f32, torque_z: f32) {
        let mut world = self.inner.borrow_mut();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.get_rigid_body_mut(handle) {
            body.add_torque(vector![torque_x, torque_y, torque_z], true);
        }
    }

    /// Set object velocity
    ///
    /// # Arguments
    /// * `object_id` - Object ID
    /// * `vel_x`, `vel_y`, `vel_z` - Velocity vector components
    #[wasm_bindgen(js_name = setVelocity)]
    pub fn set_velocity(&self, object_id: u32, vel_x: f32, vel_y: f32, vel_z: f32) {
        let mut world = self.inner.borrow_mut();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.get_rigid_body_mut(handle) {
            body.set_linvel(vector![vel_x, vel_y, vel_z], true);
        }
    }

    /// Set object angular velocity
    ///
    /// # Arguments
    /// * `object_id` - Object ID
    /// * `angvel_x`, `angvel_y`, `angvel_z` - Angular velocity components
    #[wasm_bindgen(js_name = setAngularVelocity)]
    pub fn set_angular_velocity(
        &self,
        object_id: u32,
        angvel_x: f32,
        angvel_y: f32,
        angvel_z: f32,
    ) {
        let mut world = self.inner.borrow_mut();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.get_rigid_body_mut(handle) {
            body.set_angvel(vector![angvel_x, angvel_y, angvel_z], true);
        }
    }

    /// Set object position
    ///
    /// # Arguments
    /// * `object_id` - Object ID
    /// * `pos_x`, `pos_y`, `pos_z` - Position components
    #[wasm_bindgen(js_name = setPosition)]
    pub fn set_position(&self, object_id: u32, pos_x: f32, pos_y: f32, pos_z: f32) {
        let mut world = self.inner.borrow_mut();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.get_rigid_body_mut(handle) {
            body.set_translation(vector![pos_x, pos_y, pos_z], true);
        }
    }

    /// Set object rotation (quaternion)
    ///
    /// # Arguments
    /// * `object_id` - Object ID
    /// * `quat_x`, `quat_y`, `quat_z`, `quat_w` - Quaternion components
    #[wasm_bindgen(js_name = setRotation)]
    pub fn set_rotation(&self, object_id: u32, quat_x: f32, quat_y: f32, quat_z: f32, quat_w: f32) {
        let mut world = self.inner.borrow_mut();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.get_rigid_body_mut(handle) {
            let rot =
                UnitQuaternion::new_normalize(Quaternion::new(quat_w, quat_x, quat_y, quat_z));
            body.set_rotation(rot, true);
        }
    }

    /// Remove object from simulation
    ///
    /// # Arguments
    /// * `object_id` - Object ID to remove
    #[wasm_bindgen(js_name = removeObject)]
    pub fn remove_object(&self, object_id: u32) {
        let mut world = self.inner.borrow_mut();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);
        world.remove_rigid_body(handle);
    }

    /// Get gravity vector
    ///
    /// # Returns
    /// Array [x, y, z]
    #[wasm_bindgen(js_name = getGravity)]
    pub fn get_gravity(&self) -> Vec<f32> {
        let world = self.inner.borrow();
        let g = world.gravity();
        vec![g.x, g.y, g.z]
    }

    /// Set gravity vector
    ///
    /// # Arguments
    /// * `gravity_x`, `gravity_y`, `gravity_z` - Gravity components
    #[wasm_bindgen(js_name = setGravity)]
    pub fn set_gravity(&self, gravity_x: f32, gravity_y: f32, gravity_z: f32) {
        let mut world = self.inner.borrow_mut();
        world.set_gravity(Vec3::new(gravity_x, gravity_y, gravity_z));
    }

    // ===== Character Controller Methods =====

    /// Create a character controller
    ///
    /// # Arguments
    /// * `pos_x`, `pos_y`, `pos_z` - Initial position
    /// * `height` - Character height (e.g., 1.8 for human)
    /// * `radius` - Character radius (e.g., 0.3)
    ///
    /// # Returns
    /// Character ID
    #[wasm_bindgen(js_name = createCharacter)]
    pub fn create_character(
        &self,
        pos_x: f32,
        pos_y: f32,
        pos_z: f32,
        height: f32,
        radius: f32,
    ) -> u32 {
        let mut world = self.inner.borrow_mut();
        let config = CharacterControllerConfig {
            height,
            radius,
            step_height: 0.5,
            max_slope_angle: 45.0,
            gravity: 9.8,
            jump_impulse: 5.0,
            ground_check_distance: 0.1,
        };

        let controller =
            CharacterController::new(&mut world, Vec3::new(pos_x, pos_y, pos_z), config);

        let mut next_id = self.next_character_id.borrow_mut();
        let id = *next_id;
        *next_id += 1;

        self.characters.borrow_mut().insert(id, controller);
        id
    }

    /// Create a static ground plane at Y=0
    ///
    /// Creates an infinite horizontal plane for character controllers to walk on.
    /// The plane is fixed (static) and cannot be moved.
    ///
    /// # Returns
    /// Object ID for the ground plane body
    #[wasm_bindgen(js_name = createGroundPlane)]
    pub fn create_ground_plane(&self) -> u32 {
        let mut world = self.inner.borrow_mut();

        // Create a fixed (static) rigid body at the origin
        let ground_body = RigidBodyBuilder::fixed()
            .translation(vector![0.0, 0.0, 0.0])
            .build();

        let body_handle = world.add_rigid_body(ground_body);

        // Create a horizontal plane collider (Y=0 plane with normal pointing up)
        let ground_normal = Unit::new_normalize(vector![0.0, 1.0, 0.0]);
        let ground_collider = ColliderBuilder::halfspace(ground_normal)
            .friction(0.5)
            .restitution(0.0)
            .build();

        world.add_collider(ground_collider, body_handle);

        // Return the body handle as object ID
        body_handle.into_raw_parts().0
    }

    /// Move character with horizontal velocity
    ///
    /// # Arguments
    /// * `character_id` - Character ID
    /// * `vel_x`, `vel_z` - Horizontal velocity (Y is ignored)
    /// * `dt` - Time step
    #[wasm_bindgen(js_name = moveCharacter)]
    pub fn move_character(&self, character_id: u32, vel_x: f32, vel_z: f32, dt: f32) {
        let mut world = self.inner.borrow_mut();
        let mut characters = self.characters.borrow_mut();

        if let Some(controller) = characters.get_mut(&character_id) {
            controller.move_with_velocity(&mut world, Vec3::new(vel_x, 0.0, vel_z), dt);
        }
    }

    /// Make character jump
    ///
    /// # Arguments
    /// * `character_id` - Character ID
    #[wasm_bindgen(js_name = jumpCharacter)]
    pub fn jump_character(&self, character_id: u32) {
        let mut characters = self.characters.borrow_mut();
        if let Some(controller) = characters.get_mut(&character_id) {
            controller.jump();
        }
    }

    /// Check if object (character) is grounded
    ///
    /// # Arguments
    /// * `object_id` - Object ID (character ID)
    ///
    /// # Returns
    /// True if on ground
    #[wasm_bindgen(js_name = isObjectGrounded)]
    pub fn is_object_grounded(&self, object_id: u32) -> bool {
        let characters = self.characters.borrow();
        if let Some(controller) = characters.get(&object_id) {
            controller.is_grounded()
        } else {
            false
        }
    }

    /// Get character vertical velocity
    ///
    /// # Arguments
    /// * `character_id` - Character ID
    ///
    /// # Returns
    /// Vertical velocity (positive = upward, negative = falling)
    #[wasm_bindgen(js_name = getCharacterVerticalVelocity)]
    pub fn get_character_vertical_velocity(&self, character_id: u32) -> f32 {
        let characters = self.characters.borrow();
        if let Some(controller) = characters.get(&character_id) {
            controller.vertical_velocity()
        } else {
            0.0
        }
    }

    /// Get object (character) ground normal
    ///
    /// # Arguments
    /// * `object_id` - Object ID (character ID)
    ///
    /// # Returns
    /// Array [x, y, z]
    #[wasm_bindgen(js_name = getObjectGroundNormal)]
    pub fn get_object_ground_normal(&self, object_id: u32) -> Vec<f32> {
        let characters = self.characters.borrow();
        if let Some(controller) = characters.get(&object_id) {
            let normal = controller.ground_normal();
            vec![normal.x, normal.y, normal.z]
        } else {
            vec![0.0, 1.0, 0.0]
        }
    }

    /// Get character position
    ///
    /// # Arguments
    /// * `character_id` - Character ID
    ///
    /// # Returns
    /// Array [x, y, z]
    #[wasm_bindgen(js_name = getCharacterPosition)]
    pub fn get_character_position(&self, character_id: u32) -> Vec<f32> {
        let world = self.inner.borrow();
        let characters = self.characters.borrow();
        if let Some(controller) = characters.get(&character_id) {
            let pos = controller.position(&world);
            vec![pos.x, pos.y, pos.z]
        } else {
            vec![0.0, 0.0, 0.0]
        }
    }
}

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_wasm_physics_world_creation() {
        let world = WasmPhysicsWorld::new(0.0, -9.81, 0.0);
        let gravity = world.get_gravity();
        assert_eq!(gravity, vec![0.0, -9.81, 0.0]);
    }

    #[wasm_bindgen_test]
    fn test_add_box_body() {
        let world = WasmPhysicsWorld::new(0.0, -9.81, 0.0);
        let obj_id = world.add_box_body(0.0, 10.0, 0.0, 1.0, 1.0, 1.0, 1.0);

        let pos = world.get_position(obj_id);
        assert_eq!(pos[1], 10.0);
    }

    #[wasm_bindgen_test]
    fn test_physics_step() {
        let world = WasmPhysicsWorld::new(0.0, -9.81, 0.0);
        let obj_id = world.add_box_body(0.0, 10.0, 0.0, 1.0, 1.0, 1.0, 1.0);

        world.step(0.1);

        let pos = world.get_position(obj_id);
        // Should have fallen
        assert!(pos[1] < 10.0);
    }
}
