use crate::{
    collider::{create_box_collider, create_capsule_collider, create_sphere_collider, VoxelColliderBuilder},
    world::PhysicsWorld,
};
use glam::Vec3;
use rapier3d::prelude::*;
use nalgebra::{Quaternion, UnitQuaternion};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmPhysicsWorld {
    inner: RefCell<PhysicsWorld>,
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
            inner: RefCell::new(PhysicsWorld::new(Vec3::new(gravity_x, gravity_y, gravity_z))),
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
    /// # Arguments
    /// * `csm_code` - Cube in CSM (Cubescript) format
    /// * `max_depth` - Maximum octree depth for collision detail
    /// * `is_static` - If true, body is immovable (terrain). If false, dynamic (movable object)
    ///
    /// # Returns
    /// Object ID for the created body
    #[wasm_bindgen(js_name = addVoxelBody)]
    pub fn add_voxel_body(
        &self,
        csm_code: &str,
        max_depth: u32,
        is_static: bool,
    ) -> Result<u32, JsValue> {
        let octree = crossworld_cube::parse_csm(csm_code)
            .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

        let collider = VoxelColliderBuilder::from_cube(&octree.root, max_depth);

        let mut world = self.inner.borrow_mut();
        let body = if is_static {
            RigidBodyBuilder::fixed().build()
        } else {
            RigidBodyBuilder::dynamic().build()
        };

        let handle = world.add_rigid_body(body);
        world.add_collider(collider, handle);

        Ok(handle.into_raw_parts().0)
    }

    /// Add dynamic rigid body with box collider
    ///
    /// # Arguments
    /// * `pos_x`, `pos_y`, `pos_z` - Initial position
    /// * `half_width`, `half_height`, `half_depth` - Half-extents of the box
    /// * `mass` - Mass in kg
    ///
    /// # Returns
    /// Object ID for the created body
    #[wasm_bindgen(js_name = addBoxBody)]
    pub fn add_box_body(
        &self,
        pos_x: f32,
        pos_y: f32,
        pos_z: f32,
        half_width: f32,
        half_height: f32,
        half_depth: f32,
        mass: f32,
    ) -> u32 {
        let mut world = self.inner.borrow_mut();

        let body = RigidBodyBuilder::dynamic()
            .translation(vector![pos_x, pos_y, pos_z])
            .build();
        let handle = world.add_rigid_body(body);

        let collider = create_box_collider(Vec3::new(half_width, half_height, half_depth));
        let collider = collider.with_mass(mass);
        world.add_collider(collider, handle);

        handle.into_raw_parts().0
    }

    /// Add static rigid body with box collider
    ///
    /// # Arguments
    /// * `pos_x`, `pos_y`, `pos_z` - Position
    /// * `half_width`, `half_height`, `half_depth` - Half-extents of the box
    ///
    /// # Returns
    /// Object ID for the created body
    #[wasm_bindgen(js_name = addStaticBox)]
    pub fn add_static_box(
        &self,
        pos_x: f32,
        pos_y: f32,
        pos_z: f32,
        half_width: f32,
        half_height: f32,
        half_depth: f32,
    ) -> u32 {
        let mut world = self.inner.borrow_mut();

        let body = RigidBodyBuilder::fixed()
            .translation(vector![pos_x, pos_y, pos_z])
            .build();
        let handle = world.add_rigid_body(body);

        let collider = create_box_collider(Vec3::new(half_width, half_height, half_depth));
        world.add_collider(collider, handle);

        handle.into_raw_parts().0
    }

    /// Add dynamic rigid body with sphere collider
    ///
    /// # Arguments
    /// * `pos_x`, `pos_y`, `pos_z` - Initial position
    /// * `radius` - Sphere radius
    /// * `mass` - Mass in kg
    ///
    /// # Returns
    /// Object ID for the created body
    #[wasm_bindgen(js_name = addSphereBody)]
    pub fn add_sphere_body(
        &self,
        pos_x: f32,
        pos_y: f32,
        pos_z: f32,
        radius: f32,
        mass: f32,
    ) -> u32 {
        let mut world = self.inner.borrow_mut();

        let body = RigidBodyBuilder::dynamic()
            .translation(vector![pos_x, pos_y, pos_z])
            .build();
        let handle = world.add_rigid_body(body);

        let collider = create_sphere_collider(radius).with_mass(mass);
        world.add_collider(collider, handle);

        handle.into_raw_parts().0
    }

    /// Add dynamic rigid body with capsule collider
    ///
    /// # Arguments
    /// * `pos_x`, `pos_y`, `pos_z` - Initial position
    /// * `half_height` - Half the height of the cylindrical part
    /// * `radius` - Capsule radius
    /// * `mass` - Mass in kg
    ///
    /// # Returns
    /// Object ID for the created body
    #[wasm_bindgen(js_name = addCapsuleBody)]
    pub fn add_capsule_body(
        &self,
        pos_x: f32,
        pos_y: f32,
        pos_z: f32,
        half_height: f32,
        radius: f32,
        mass: f32,
    ) -> u32 {
        let mut world = self.inner.borrow_mut();

        let body = RigidBodyBuilder::dynamic()
            .translation(vector![pos_x, pos_y, pos_z])
            .build();
        let handle = world.add_rigid_body(body);

        let collider = create_capsule_collider(half_height, radius).with_mass(mass);
        world.add_collider(collider, handle);

        handle.into_raw_parts().0
    }

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
    pub fn set_rotation(
        &self,
        object_id: u32,
        quat_x: f32,
        quat_y: f32,
        quat_z: f32,
        quat_w: f32,
    ) {
        let mut world = self.inner.borrow_mut();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.get_rigid_body_mut(handle) {
            let rot = UnitQuaternion::new_normalize(Quaternion::new(quat_w, quat_x, quat_y, quat_z));
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

    // TODO: Re-enable once raycast is implemented in PhysicsWorld
    // /// Cast a ray through the physics world
    // #[wasm_bindgen(js_name = raycast)]
    // pub fn raycast(...) -> Vec<f32> { ... }

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
