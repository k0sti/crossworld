# Crossworld Physics

Physics simulation system for Crossworld, integrating [Rapier](https://rapier.rs/) physics engine with octree-based voxel collision detection.

## Features

- **Real-time physics simulation** with rigid body dynamics
- **Voxel collision detection** - automatically generates collision geometry from octree voxel cubes
- **WASM-compatible** - runs in web browsers via WebAssembly (no conditional compilation)
- **Multiple collider types** - box, sphere, capsule, and voxel-based compound colliders
- **Force and impulse application** - apply forces, impulses, and torques to objects
- **Gravity simulation** - configurable gravity vector
- **AABB collision system** - efficient Axis-Aligned Bounding Box collision with OBB support
- **Region-bounded traversal** - generate collision geometry only for overlapping regions (70-90% reduction)
- **SDF collision interface** - signed distance function trait for smooth fabric surfaces

## Usage

### Basic Example

```rust
use crossworld_physics::{PhysicsWorld, CubeObject, create_box_collider};
use glam::Vec3;

// Create physics world with gravity
let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

// Create static ground
let mut ground = CubeObject::new_static(&mut world, Vec3::new(0.0, -0.5, 0.0));
let ground_collider = create_box_collider(Vec3::new(10.0, 0.5, 10.0));
ground.attach_collider(&mut world, ground_collider);

// Create dynamic falling box
let mut falling_box = CubeObject::new_dynamic(&mut world, Vec3::new(0.0, 10.0, 0.0), 1.0);
let box_collider = create_box_collider(Vec3::new(0.5, 0.5, 0.5));
falling_box.attach_collider(&mut world, box_collider);

// Simulate
for _ in 0..180 {
    world.step(1.0 / 60.0);  // 60 FPS
}

// Get final position
let pos = falling_box.position(&world);
println!("Final position: {:?}", pos);
```

### Voxel Collision

```rust
use crossworld_physics::{PhysicsWorld, CubeObject, VoxelColliderBuilder};
use std::rc::Rc;

let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

// Load or create a voxel cube
let cube = Rc::new(crossworld_cube::Cube::Solid(1));

// Generate collision geometry from voxel faces
let voxel_collider = VoxelColliderBuilder::from_cube(&cube, 3);

// Create static terrain body
let mut terrain = CubeObject::new_static(&mut world, Vec3::ZERO);
terrain.attach_collider(&mut world, voxel_collider);
```

### AABB Collision Detection

```rust
use crossworld_physics::collision::{Aabb, CubeCollider, ObjectCollider};
use glam::{Vec3, Quat};

// Create AABBs for two objects
let cube_aabb = Aabb::unit().to_world(Vec3::ZERO, Quat::IDENTITY, 10.0);
let object_aabb = Aabb::unit().to_world(Vec3::new(5.0, 5.0, 5.0), Quat::IDENTITY, 2.0);

// Quick broad-phase collision test
if CubeCollider::might_collide(&cube_aabb, &object_aabb) {
    // Get intersection region in cube's local space [0,1]
    if let Some(local_region) = CubeCollider::intersection_region(
        &cube_aabb, &object_aabb, Vec3::ZERO, 10.0
    ) {
        // Generate collision geometry only for this region
        // (uses visit_faces_in_region internally)
    }
}
```

### CubeObject AABB Support

```rust
use crossworld_physics::{PhysicsWorld, CubeObject};
use crossworld_physics::collision::Aabb;
use glam::Vec3;

let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
let mut obj = CubeObject::new_dynamic(&mut world, Vec3::new(5.0, 10.0, 0.0), 1.0);
obj.set_scale(2.0);

// Get world-space AABB (automatically handles position, rotation, scale)
let world_aabb = obj.world_aabb(&world);
println!("AABB min: {:?}, max: {:?}", world_aabb.min, world_aabb.max);

// Quick intersection test
let other_aabb = Aabb::new(Vec3::ZERO, Vec3::splat(10.0));
if obj.intersects_aabb(&world, &other_aabb) {
    println!("Potential collision detected!");
}
```

### WASM Usage

Enable the `wasm` feature in your Cargo.toml:

```toml
[dependencies]
crossworld-physics = { path = "../physics", features = ["wasm"] }
```

Then use the WASM interface:

```rust
use crossworld_physics::WasmPhysicsWorld;

// Create physics world
let world = WasmPhysicsWorld::new(0.0, -9.81, 0.0);

// Add a dynamic box
let obj_id = world.add_box_body(0.0, 10.0, 0.0, 1.0, 1.0, 1.0, 1.0);

// Step simulation
world.step(0.016);  // 60 FPS

// Get position
let pos = world.get_position(obj_id);

// Apply force
world.apply_force(obj_id, 0.0, 100.0, 0.0);
```

## Building

```bash
# Build the crate
cargo build -p crossworld-physics

# Run tests
cargo test -p crossworld-physics

# Run examples
cargo run --example basic_simulation -p crossworld-physics
cargo run --example voxel_collision -p crossworld-physics

# Build for WASM
cd crates/physics
wasm-pack build --target web --features wasm
```

## Architecture

### Components

- **PhysicsWorld** - Main simulation container managing Rapier's physics pipeline
- **CubeObject** - Wrapper for rigid bodies with position, velocity, rotation, and optional voxel cube reference
- **VoxelColliderBuilder** - Generates collision geometry from octree voxels
- **WasmPhysicsWorld** - JavaScript-accessible WASM interface

### Voxel Collision Detection

The `VoxelColliderBuilder` uses the cube crate's `traverse_with_neighbors` function to:

1. Iterate through all voxels in the octree
2. Check each voxel's 6 faces against neighbors
3. Generate rectangle colliders for exposed faces
4. Combine into a compound collider for efficient physics

This approach provides accurate collision detection while maintaining good performance.

## API Reference

### PhysicsWorld

- `new(gravity: Vec3)` - Create physics world
- `step(dt: f32)` - Step simulation forward
- `add_rigid_body(body: RigidBody)` - Add rigid body
- `remove_rigid_body(handle: RigidBodyHandle)` - Remove rigid body

### CubeObject

- `new_dynamic(world, position, mass)` - Create dynamic body
- `new_kinematic(world, position)` - Create kinematic body
- `new_static(world, position)` - Create static body
- `attach_collider(&mut self, world, collider)` - Attach collider to body
- `set_cube(&mut self, cube)` - Set cube reference used for collision
- `cube(&self)` - Get cube reference if it exists
- `scale(&self)` - Get object scale
- `set_scale(&mut self, scale)` - Set object scale
- `local_aabb(&self)` - Get local-space AABB [0,1]³
- `world_aabb(&self, world)` - Get world-space AABB (handles position, rotation, scale)
- `intersects_aabb(&self, world, other)` - Test AABB intersection
- `position(&self, world)` - Get position
- `set_position(&self, world, position)` - Set position
- `velocity(&self, world)` - Get velocity
- `set_velocity(&self, world, velocity)` - Set velocity
- `apply_force(&self, world, force)` - Apply force
- `apply_impulse(&self, world, impulse)` - Apply impulse

### Collision Module (`collision`)

**Aabb:**
- `unit()` - Create unit cube AABB [0,1]³
- `new(min, max)` - Create AABB from corners
- `to_world(position, rotation, scale)` - Transform to world space (OBB→AABB)
- `intersects(&self, other)` - Test intersection
- `intersection(&self, other)` - Get intersection volume

**CubeCollider:**
- `might_collide(cube_aabb, object_aabb)` - Broad-phase collision test
- `intersection_region(cube_aabb, object_aabb, cube_pos, cube_scale)` - Get local intersection region

**ObjectCollider:**
- `might_collide(aabb_a, aabb_b)` - Broad-phase collision test
- `intersection_regions(aabb_a, aabb_b, pos_a, pos_b, scale_a, scale_b)` - Get both objects' local regions

### SDF Module (`sdf`)

**SdfCollider trait:**
- `sdf(&self, point)` - Signed distance to surface (negative = inside)
- `normal(&self, point)` - Surface normal at point
- `is_inside(&self, point)` - Check if point is inside
- `sphere_penetration(center, radius)` - Get penetration depth and normal

Built-in implementations: `SphereSdf`, `BoxSdf`

### WasmPhysicsWorld (WASM only)

- `new(gx, gy, gz)` - Create world
- `step(dt)` - Step simulation
- `addBoxBody(x, y, z, hw, hh, hd, mass)` - Add box
- `addSphereBody(x, y, z, radius, mass)` - Add sphere
- `addVoxelBody(csm, depth, is_static)` - Add voxel body
- `getPosition(id)` - Get position array [x, y, z]
- `getRotation(id)` - Get rotation quaternion [x, y, z, w]
- `getVelocity(id)` - Get velocity [x, y, z]
- `applyForce(id, fx, fy, fz)` - Apply force
- `applyImpulse(id, ix, iy, iz)` - Apply impulse
- `removeObject(id)` - Remove object

## Performance Considerations

- Use static bodies for terrain (never moves, no overhead)
- Use kinematic bodies for player-controlled objects
- Use dynamic bodies for physics-simulated objects
- Keep voxel collision depth reasonable (3-5 is good balance)
- Consider using simpler colliders (box/sphere) for distant objects

## Future Enhancements

- Raycasting support (once Rapier API is clarified)
- Character controller for player movement
- Joints and constraints
- Convex hull decomposition for complex voxel shapes
- Face merging optimization for fewer collision rectangles
- LOD physics for distant objects

## License

Same as parent project.
