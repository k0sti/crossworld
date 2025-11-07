# Physics System Design for Crossworld

## Overview

This document outlines the design and implementation plan for a physics system for Crossworld, integrating Rapier physics engine with the existing octree-based voxel system.

## Goals

- Real-time physics simulation for dynamic objects in voxel world
- Collision detection between objects and voxel terrain
- WASM-compatible interface for web deployment
- Efficient collision geometry generation from octree voxel data
- Support for rigid body dynamics with forces, velocities, and rotations

## Architecture

### Project Structure

```
crates/physics/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Core physics engine wrapper
│   ├── collider.rs         # Voxel collision geometry generation
│   ├── rigid_body.rs       # Rigid body management
│   ├── world.rs            # Physics world state
│   └── wasm.rs             # WASM bindings (feature-gated)
└── examples/
    └── basic_simulation.rs
```

### Dependencies

```toml
[dependencies]
# Physics engine - use git version for latest features
rapier3d = { git = "https://github.com/dimforge/rapier.git", features = ["wasm-bindgen"] }

# Voxel model integration
crossworld-cube = { path = "../cube" }

# Math library (shared with rest of project)
glam = { version = "0.29", features = ["serde"] }

# WASM bindings (optional)
wasm-bindgen = { version = "0.2", optional = true }
serde = { version = "1.0", features = ["derive"], optional = true }
serde-wasm-bindgen = { version = "0.6", optional = true }

[features]
default = []
wasm = ["dep:wasm-bindgen", "dep:serde", "dep:serde-wasm-bindgen"]

[target.'cfg(target_arch = "wasm32")'.dependencies]
getrandom = { version = "0.3", features = ["wasm_js"] }
```

## Core Components

### 1. Physics World (`world.rs`)

The main physics simulation container managing all rigid bodies and collision detection.

**Key responsibilities:**
- Maintain Rapier physics world state
- Step simulation forward in time
- Query collision events
- Manage rigid bodies and colliders

**API:**
```rust
pub struct PhysicsWorld {
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    gravity: Vector3<f32>,
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
}

impl PhysicsWorld {
    pub fn new(gravity: Vec3) -> Self;
    pub fn step(&mut self, dt: f32);
    pub fn add_rigid_body(&mut self, body: RigidBody) -> RigidBodyHandle;
    pub fn remove_rigid_body(&mut self, handle: RigidBodyHandle);
    pub fn add_collider(&mut self, collider: Collider, parent: RigidBodyHandle) -> ColliderHandle;
}
```

### 2. Voxel Collision Generator (`collider.rs`)

Generates collision geometry from octree voxel data by traversing faces and creating compound colliders.

**Strategy:**

The cube crate provides `traverse_with_neighbors` which visits each leaf voxel with access to its 6 face neighbors. We use this to:

1. Iterate through all voxels in the octree
2. For each voxel face, check if neighbor is empty or different material
3. If face is exposed, generate a rectangle collider for that face
4. Combine all face rectangles into a compound collider

**Face Detection:**

```rust
use crossworld_cube::{
    Face, NeighborView, CubeCoord, traverse_with_neighbors,
    OFFSET_LEFT, OFFSET_RIGHT, OFFSET_UP, OFFSET_DOWN, OFFSET_FRONT, OFFSET_BACK
};

pub struct VoxelColliderBuilder {
    rectangles: Vec<(Vec3, Vec3, f32)>, // (position, normal, size)
}

impl VoxelColliderBuilder {
    /// Generate colliders from cube octree
    pub fn from_cube(cube: &Cube<i32>, max_depth: u32) -> Collider {
        let mut builder = Self { rectangles: Vec::new() };

        // Create neighbor grid with appropriate border materials
        let border_materials = [1, 1, 0, 0]; // solid bottom, empty top
        let grid = NeighborGrid::new(cube, border_materials);

        // Traverse all voxels
        traverse_with_neighbors(&grid, &mut |view, coord, _subleaf| {
            builder.process_voxel(view, coord);
            false // don't subdivide further
        }, max_depth);

        builder.build_compound_collider()
    }

    fn process_voxel(&mut self, view: NeighborView, coord: CubeCoord) {
        let center = view.center();

        // Skip empty voxels
        if center.is_empty() {
            return;
        }

        // Check each of 6 faces
        let faces = [
            (OFFSET_LEFT, Face::Left),
            (OFFSET_RIGHT, Face::Right),
            (OFFSET_DOWN, Face::Bottom),
            (OFFSET_UP, Face::Top),
            (OFFSET_BACK, Face::Back),
            (OFFSET_FRONT, Face::Front),
        ];

        for (offset, face) in faces {
            if let Some(neighbor) = view.get(offset) {
                // Face is exposed if neighbor is empty or different material
                if neighbor.is_empty() || neighbor.id() != center.id() {
                    self.add_face_rectangle(coord, face);
                }
            }
        }
    }

    fn add_face_rectangle(&mut self, coord: CubeCoord, face: Face) {
        // Calculate world position from octree coordinate
        let voxel_size = 1.0 / (1 << coord.depth) as f32;
        let world_pos = coord.pos.as_vec3() * voxel_size;

        // Get face center position
        let face_offset = Vec3::from(face.normal()) * voxel_size * 0.5;
        let face_center = world_pos + Vec3::splat(voxel_size * 0.5) + face_offset;

        self.rectangles.push((
            face_center,
            Vec3::from(face.normal()),
            voxel_size
        ));
    }

    fn build_compound_collider(self) -> Collider {
        // Create thin cuboid colliders for each face
        let shapes: Vec<_> = self.rectangles.iter().map(|(pos, normal, size)| {
            let half_size = size / 2.0;
            let thickness = 0.01; // Thin collider for face

            // Create cuboid aligned with face normal
            let shape = SharedShape::cuboid(half_size, half_size, thickness);

            // Calculate rotation to align with normal
            let rotation = Quat::from_rotation_arc(Vec3::Z, *normal);
            let isometry = Isometry::new(*pos, rotation);

            (isometry, shape)
        }).collect();

        ColliderBuilder::compound(shapes).build()
    }
}
```

**Optimization strategies:**

1. **Face merging**: Combine adjacent coplanar rectangles into larger rectangles
2. **LOD colliders**: Generate simpler collision geometry for distant objects
3. **Caching**: Cache generated colliders and invalidate only when voxels change
4. **Spatial partitioning**: Only generate colliders for active regions near dynamic objects

### 3. Rigid Body Management (`rigid_body.rs`)

Manages individual physics objects with properties like mass, velocity, forces.

**API:**

```rust
pub struct RigidBodyObject {
    handle: RigidBodyHandle,
    collider_handle: ColliderHandle,
}

impl RigidBodyObject {
    pub fn new_dynamic(world: &mut PhysicsWorld, position: Vec3, mass: f32) -> Self;
    pub fn new_kinematic(world: &mut PhysicsWorld, position: Vec3) -> Self;
    pub fn position(&self, world: &PhysicsWorld) -> Vec3;
    pub fn rotation(&self, world: &PhysicsWorld) -> Quat;
    pub fn velocity(&self, world: &PhysicsWorld) -> Vec3;
    pub fn angular_velocity(&self, world: &PhysicsWorld) -> Vec3;
    pub fn apply_force(&self, world: &mut PhysicsWorld, force: Vec3);
    pub fn apply_impulse(&self, world: &mut PhysicsWorld, impulse: Vec3);
    pub fn set_velocity(&self, world: &mut PhysicsWorld, velocity: Vec3);
}
```

### 4. WASM Interface (`wasm.rs`)

Provides JavaScript-accessible API for web integration.

**API Design:**

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmPhysicsWorld {
    inner: RefCell<PhysicsWorld>,
}

#[wasm_bindgen]
impl WasmPhysicsWorld {
    /// Create new physics world
    #[wasm_bindgen(constructor)]
    pub fn new(gravity_x: f32, gravity_y: f32, gravity_z: f32) -> Self {
        Self {
            inner: RefCell::new(PhysicsWorld::new(Vec3::new(gravity_x, gravity_y, gravity_z)))
        }
    }

    /// Step simulation forward by dt seconds
    #[wasm_bindgen(js_name = step)]
    pub fn step(&self, dt: f32) {
        self.inner.borrow_mut().step(dt);
    }

    /// Add rigid body from voxel cube (CSM format)
    #[wasm_bindgen(js_name = addVoxelBody)]
    pub fn add_voxel_body(&self, csm_code: &str, max_depth: u32, is_static: bool) -> Result<u32, JsValue> {
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
    #[wasm_bindgen(js_name = addBoxBody)]
    pub fn add_box_body(
        &self,
        pos_x: f32, pos_y: f32, pos_z: f32,
        half_width: f32, half_height: f32, half_depth: f32,
        mass: f32
    ) -> u32 {
        let mut world = self.inner.borrow_mut();

        let body = RigidBodyBuilder::dynamic()
            .translation(vector![pos_x, pos_y, pos_z])
            .build();
        let handle = world.add_rigid_body(body);

        let collider = ColliderBuilder::cuboid(half_width, half_height, half_depth)
            .density(mass / (8.0 * half_width * half_height * half_depth))
            .build();
        world.add_collider(collider, handle);

        handle.into_raw_parts().0
    }

    /// Get list of all object IDs
    #[wasm_bindgen(js_name = getAllObjects)]
    pub fn get_all_objects(&self) -> Vec<u32> {
        let world = self.inner.borrow();
        world.rigid_body_set.iter()
            .map(|(handle, _)| handle.into_raw_parts().0)
            .collect()
    }

    /// Get object position
    #[wasm_bindgen(js_name = getPosition)]
    pub fn get_position(&self, object_id: u32) -> Vec<f32> {
        let world = self.inner.borrow();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.rigid_body_set.get(handle) {
            let pos = body.translation();
            vec![pos.x, pos.y, pos.z]
        } else {
            vec![0.0, 0.0, 0.0]
        }
    }

    /// Get object rotation (as quaternion)
    #[wasm_bindgen(js_name = getRotation)]
    pub fn get_rotation(&self, object_id: u32) -> Vec<f32> {
        let world = self.inner.borrow();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.rigid_body_set.get(handle) {
            let rot = body.rotation();
            vec![rot.i, rot.j, rot.k, rot.w]
        } else {
            vec![0.0, 0.0, 0.0, 1.0]
        }
    }

    /// Get object linear velocity
    #[wasm_bindgen(js_name = getVelocity)]
    pub fn get_velocity(&self, object_id: u32) -> Vec<f32> {
        let world = self.inner.borrow();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.rigid_body_set.get(handle) {
            let vel = body.linvel();
            vec![vel.x, vel.y, vel.z]
        } else {
            vec![0.0, 0.0, 0.0]
        }
    }

    /// Get object angular velocity
    #[wasm_bindgen(js_name = getAngularVelocity)]
    pub fn get_angular_velocity(&self, object_id: u32) -> Vec<f32> {
        let world = self.inner.borrow();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.rigid_body_set.get(handle) {
            let angvel = body.angvel();
            vec![angvel.x, angvel.y, angvel.z]
        } else {
            vec![0.0, 0.0, 0.0]
        }
    }

    /// Apply force to object
    #[wasm_bindgen(js_name = applyForce)]
    pub fn apply_force(&self, object_id: u32, force_x: f32, force_y: f32, force_z: f32) {
        let mut world = self.inner.borrow_mut();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.rigid_body_set.get_mut(handle) {
            body.add_force(vector![force_x, force_y, force_z], true);
        }
    }

    /// Apply impulse to object
    #[wasm_bindgen(js_name = applyImpulse)]
    pub fn apply_impulse(&self, object_id: u32, impulse_x: f32, impulse_y: f32, impulse_z: f32) {
        let mut world = self.inner.borrow_mut();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.rigid_body_set.get_mut(handle) {
            body.apply_impulse(vector![impulse_x, impulse_y, impulse_z], true);
        }
    }

    /// Set object velocity
    #[wasm_bindgen(js_name = setVelocity)]
    pub fn set_velocity(&self, object_id: u32, vel_x: f32, vel_y: f32, vel_z: f32) {
        let mut world = self.inner.borrow_mut();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);

        if let Some(body) = world.rigid_body_set.get_mut(handle) {
            body.set_linvel(vector![vel_x, vel_y, vel_z], true);
        }
    }

    /// Remove object from simulation
    #[wasm_bindgen(js_name = removeObject)]
    pub fn remove_object(&self, object_id: u32) {
        let mut world = self.inner.borrow_mut();
        let handle = RigidBodyHandle::from_raw_parts(object_id, 0);
        world.remove_rigid_body(handle);
    }
}
```

## Integration with Existing Codebase

### Workspace Integration

Update `/home/k0/work/crossworld/world-cw/Cargo.toml`:

```toml
[workspace]
members = ["crates/world", "crates/worldtool", "crates/cube", "crates/assets", "crates/physics"]
resolver = "2"
```

### World Crate Integration

The `world` crate can optionally depend on `physics` crate:

```toml
# In crates/world/Cargo.toml
[dependencies]
crossworld-physics = { path = "../physics", optional = true }

[features]
physics = ["dep:crossworld-physics"]
```

## Implementation Phases

### Phase 1: Core Physics Engine (Week 1)

**Deliverables:**
- [x] Create `crates/physics` directory structure
- [x] Set up `Cargo.toml` with rapier dependency
- [x] Implement `PhysicsWorld` wrapper
- [x] Implement basic `RigidBodyObject`
- [x] Write unit tests for basic physics operations
- [x] Example: Drop a box with gravity

**Files to create:**
- `crates/physics/Cargo.toml`
- `crates/physics/src/lib.rs`
- `crates/physics/src/world.rs`
- `crates/physics/src/rigid_body.rs`
- `crates/physics/examples/basic_simulation.rs`

### Phase 2: Voxel Collision Generation (Week 2)

**Deliverables:**
- [x] Implement `VoxelColliderBuilder`
- [x] Integrate with `traverse_with_neighbors`
- [x] Face rectangle generation from voxel faces
- [x] Compound collider building
- [x] Unit tests with simple voxel shapes
- [x] Example: Drop voxel object onto terrain

**Files to create:**
- `crates/physics/src/collider.rs`
- `crates/physics/examples/voxel_collision.rs`

### Phase 3: WASM Bindings (Week 3)

**Deliverables:**
- [x] Feature-gated WASM module
- [x] `WasmPhysicsWorld` wrapper
- [x] All JavaScript-accessible methods
- [x] WASM build configuration
- [x] TypeScript type definitions
- [x] Web example/demo

**Files to create:**
- `crates/physics/src/wasm.rs`
- `crates/physics/physics.d.ts` (TypeScript definitions)
- Example HTML/JS page demonstrating physics

### Phase 4: Optimization & Polish (Week 4)

**Deliverables:**
- [x] Face merging optimization
- [x] Collision geometry caching
- [x] Performance profiling
- [x] Memory optimization for WASM
- [x] Documentation and examples
- [x] Integration tests

**Focus areas:**
- Profile physics step time
- Optimize collider generation
- Reduce WASM binary size
- Add debug visualization helpers

## Performance Considerations

### WASM-Specific Optimizations

1. **Minimize heap allocations**: Use object pooling for frequently created/destroyed objects
2. **Batch operations**: Group multiple physics queries together to reduce JS/WASM boundary crossings
3. **LOD physics**: Use simpler collision shapes for distant objects
4. **Fixed timestep**: Run physics at consistent rate (e.g., 60Hz) independent of render rate

### Collision Geometry Optimization

1. **Voxel chunk system**: Generate colliders per chunk, not entire world
2. **Lazy generation**: Only create colliders for chunks near dynamic objects
3. **Convex decomposition**: For complex voxel shapes, decompose into convex hulls (faster than compound)
4. **Simplified collision**: Use bounding boxes for far objects, detailed geometry up close

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rigid_body_gravity() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let body = RigidBodyObject::new_dynamic(&mut world, Vec3::ZERO, 1.0);

        world.step(1.0);

        let pos = body.position(&world);
        assert!(pos.y < 0.0); // Should fall
    }

    #[test]
    fn test_voxel_collider_generation() {
        let cube = Cube::Solid(1);
        let collider = VoxelColliderBuilder::from_cube(&cube, 3);
        // Verify 6 faces generated
        assert_eq!(collider.shape().as_compound().unwrap().shapes().len(), 6);
    }
}
```

### Integration Tests

Test physics with actual voxel terrain from CSM files.

### WASM Tests

```rust
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test]
fn test_wasm_physics_world() {
    let world = WasmPhysicsWorld::new(0.0, -9.81, 0.0);
    let obj_id = world.add_box_body(0.0, 10.0, 0.0, 1.0, 1.0, 1.0, 1.0);

    world.step(0.016); // 60 FPS

    let pos = world.get_position(obj_id);
    assert!(pos[1] < 10.0); // Should have fallen
}
```

## Build Commands

```bash
# Build physics crate
cargo build -p crossworld-physics

# Run tests
cargo test -p crossworld-physics

# Build for WASM
cd crates/physics
wasm-pack build --target web --features wasm

# Run examples
cargo run --example basic_simulation
```

## Future Enhancements

1. **Character controller**: Kinematic controller for player movement with collision
2. **Raycasting**: Query physics world for line-of-sight, shooting, etc.
3. **Joints and constraints**: Connect objects with hinges, springs, etc.
4. **Soft body dynamics**: Deformable objects
5. **Particle physics**: Debris, smoke, etc.
6. **Network synchronization**: Deterministic physics for multiplayer
7. **Physics-based destruction**: Break voxels on impact
8. **Sound integration**: Collision sounds based on material properties

## References

- [Rapier Documentation](https://rapier.rs/docs/)
- [Rapier GitHub](https://github.com/dimforge/rapier)
- [wasm-bindgen Guide](https://rustwasm.github.io/wasm-bindgen/)
- Crossworld cube crate: `crates/cube/src/neighbor_traversal.rs`
- Face definition: `crates/cube/src/face.rs`

## Questions & Decisions

### Q: Should we use shared shapes or individual colliders per face?
**A:** Use compound colliders with shared shapes for memory efficiency. Profile to see if face merging is needed.

### Q: How to handle dynamic voxel updates?
**A:** Regenerate colliders when voxels change. Use spatial hashing to only update affected regions.

### Q: Fixed or variable timestep?
**A:** Fixed timestep (1/60s) for deterministic physics, with accumulator pattern for smooth rendering.

### Q: How detailed should voxel colliders be?
**A:** Configurable LOD - high detail for player nearby, simplified (bounding box) for distant terrain.

---

**Document Status**: Initial draft - Ready for review and implementation
**Last Updated**: 2025-11-07
**Author**: Physics System Team
