# Design: Bevy Physics Prototype

## Overview

The physics prototype is a standalone native Bevy application that validates advanced voxel physics capabilities before integration into the web application. It demonstrates optimized collision detection for voxel objects and realistic physics simulation with character control.

## Architecture

### System Hierarchy

```
Proto Application (Bevy ECS)
├── Startup Systems
│   ├── Config Loading (TOML)
│   ├── World Generation (WorldCube)
│   ├── Camera Setup
│   └── Initial Cube Spawning
├── Update Systems
│   ├── Camera Controls (orbit/free-fly)
│   ├── Player Movement (character controller)
│   └── Debug Info Display
└── Physics Systems (Rapier Plugin)
    ├── Collision Detection
    ├── Rigid Body Simulation
    └── Character Controller
```

### Component Design

**VoxelObject**
```rust
pub struct VoxelObject {
    cube: Rc<Cube<i32>>,  // Voxel data from cube crate
    model_name: String,   // Source .vox filename
    depth: u32,           // Octree depth
}
```

**PlayerCube**
```rust
pub struct PlayerCube {
    move_speed: f32,  // Units per second
    jump_force: f32,  // Impulse magnitude
}
```

**WorldData**
```rust
pub struct WorldData {
    world_cube: WorldCube,  // From world crate
    macro_depth: u32,
    micro_depth: u32,
    seed: u32,
}
```

## Optimized Collision Algorithm

### Problem Statement

Traditional voxel collision generates colliders for **all** exposed faces of a voxel object. For a 32x32x32 cube, this can create thousands of face rectangles, causing performance issues during collision detection.

### Solution: Spatial Filtering

**Key Insight**: When testing collision between two voxel objects, only faces within the **overlapping AABB region** can potentially collide.

**Algorithm**:
1. Calculate AABB (Axis-Aligned Bounding Box) for each voxel object from their rigid body transforms
2. Check for AABB overlap using Rapier's built-in queries
3. If AABBs overlap, calculate the intersection region (min/max bounds)
4. Generate collider faces **only** for voxels within the intersection region
5. Perform collision detection on the reduced face set

**Implementation**:
```rust
pub fn from_cube_region(
    cube: &Rc<Cube<i32>>,
    max_depth: u32,
    region: Option<Aabb>,  // Optional spatial filter
) -> Collider {
    let mut builder = VoxelColliderBuilder::new();

    let grid = NeighborGrid::new(cube, [1, 1, 0, 0]);

    traverse_octree(
        &grid,
        &mut |view, coord, _subleaf| {
            // Spatial filtering: skip voxels outside region
            if let Some(aabb) = region {
                let voxel_pos = coord_to_world_pos(&coord);
                if !aabb.contains_point(voxel_pos) {
                    return false; // Skip this voxel
                }
            }

            builder.process_voxel(view, coord);
            false
        },
        max_depth,
    );

    builder.build_compound_collider()
}
```

**Performance Characteristics**:
- Full collision: O(n) faces for n voxels
- Optimized collision: O(k) faces for k voxels in overlap region
- Typical reduction: 70-90% fewer faces for small overlap regions
- Best case: 95%+ reduction when AABBs barely overlap
- Worst case: No reduction when objects fully overlap

## Configuration System

### config.toml Structure

```toml
[world]
macro_depth = 3      # Terrain generation depth (0-7)
micro_depth = 4      # Detail/edit depth (0-7)
border_depth = 1     # Blending layer depth
seed = 12345         # Procedural generation seed

[physics]
gravity = -9.81      # Y-axis gravity (m/s²)
timestep = 0.016666  # Physics timestep (1/60 second)

[spawning]
spawn_count = 20                    # Number of cubes to spawn
models_path = "packages/app/dist/assets/models/vox/"
min_height = 20.0                   # Minimum spawn Y position
max_height = 50.0                   # Maximum spawn Y position
spawn_radius = 30.0                 # X/Z spawn radius from origin

[player]
move_speed = 5.0     # Movement speed (units/sec)
jump_force = 8.0     # Jump impulse magnitude
camera_distance = 10.0  # Orbit camera distance
```

### Loading Strategy

1. **Parse config.toml** at startup using `toml` crate
2. **Fallback to defaults** if config missing or invalid
3. **Validate parameters** (depth limits, positive values)
4. Store config as Bevy `Resource` for system access

## Voxel-to-Mesh Conversion

Reuses the existing pattern from `crates/editor`:

```rust
fn voxel_to_bevy_mesh(cube: &Rc<Cube<i32>>, max_depth: u32) -> Mesh {
    use cube::{generate_face_mesh, DefaultMeshBuilder};

    let mut builder = DefaultMeshBuilder::new();

    // Color mapping from material ID
    let color_fn = |material_id: i32| -> [f32; 3] {
        // Use material system or simple color map
        material_id_to_rgb(material_id)
    };

    generate_face_mesh(
        cube,
        &mut builder,
        color_fn,
        max_depth,
        [0, 0, 0, 0],  // border_materials
        1,             // base_depth
    );

    // Convert to Bevy mesh (positions, normals, colors, indices)
    convert_to_bevy_mesh(&builder)
}
```

## Camera System

### Dual Camera Modes

**LookAt Mode**:
- Camera orbits around scene center (calculated from voxel bounds)
- Right-click drag rotates around center
- Scroll wheel zooms in/out
- Maintains constant orientation toward center

**Free-Fly Mode**:
- Camera moves independently (FPS-style)
- Right-click drag rotates camera in place
- WASD/QE for translation
- Scroll wheel for forward/backward movement

Toggle between modes with `C` key.

## Character Controller Integration

Uses Rapier's `KinematicCharacterController` for player physics:

```rust
fn setup_player(
    mut commands: Commands,
    config: Res<ProtoConfig>,
) {
    // Create player voxel cube (or load from .vox)
    let player_cube = load_player_model();
    let player_mesh = voxel_to_bevy_mesh(&player_cube, 5);

    commands.spawn((
        PlayerCube {
            move_speed: config.player.move_speed,
            jump_force: config.player.jump_force,
        },
        Mesh3d(meshes.add(player_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial { .. })),
        RigidBody::KinematicPositionBased,
        KinematicCharacterController::default(),
        Collider::capsule_y(0.9, 0.3),  // Simple capsule for player
        Transform::from_xyz(0.0, 10.0, 0.0),
    ));
}
```

**Movement System**:
```rust
fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&PlayerCube, &mut KinematicCharacterController)>,
    time: Res<Time>,
) {
    for (player, mut controller) in query.iter_mut() {
        let mut movement = Vec3::ZERO;

        if keyboard.pressed(KeyCode::KeyW) { movement.z -= 1.0; }
        if keyboard.pressed(KeyCode::KeyS) { movement.z += 1.0; }
        if keyboard.pressed(KeyCode::KeyA) { movement.x -= 1.0; }
        if keyboard.pressed(KeyCode::KeyD) { movement.x += 1.0; }

        if keyboard.just_pressed(KeyCode::Space) {
            // Apply jump impulse
            controller.translation = Some(Vec3::Y * player.jump_force * time.delta_secs());
        }

        if movement.length() > 0.0 {
            movement = movement.normalize() * player.move_speed * time.delta_secs();
            controller.translation = Some(movement);
        }
    }
}
```

## Physics Configuration

### Rapier Integration

```rust
fn setup_physics_plugin(app: &mut App, config: &ProtoConfig) {
    app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
       .insert_resource(RapierConfiguration {
           gravity: Vec3::new(0.0, config.physics.gravity, 0.0),
           timestep_mode: TimestepMode::Fixed {
               dt: config.physics.timestep,
               substeps: 1,
           },
           ..default()
       });
}
```

### Collider Parameters

**World (static)**:
- RigidBody: Fixed
- Collider: Compound (from VoxelColliderBuilder)
- Restitution: 0.2 (slight bounce)
- Friction: 0.6 (moderate grip)

**Dynamic Cubes**:
- RigidBody: Dynamic
- Collider: Compound (optimized with spatial filtering)
- Restitution: 0.4 (noticeable bounce)
- Friction: 0.5
- Mass: Computed from volume (density = 1.0)

**Player**:
- RigidBody: KinematicPositionBased
- Collider: Capsule (simplified for responsiveness)
- Restitution: 0.0 (no bounce)
- Friction: 1.0 (full grip)

## Performance Targets

### Frame Rate
- **60 FPS** with 10 cubes (baseline)
- **45+ FPS** with 50 cubes (stress test)
- **30+ FPS** with 100 cubes (extreme test)

### Collision Optimization
- **70%+** face reduction for typical collisions
- **Sub-millisecond** collider generation per object
- **Real-time** collision updates during simulation

### Memory
- **< 100 MB** memory usage with 50 cubes
- **Linear scaling** with object count

## Testing Strategy

### Unit Tests
- Spatial filtering correctness (verify only overlap faces included)
- AABB intersection calculation
- Config parsing and validation
- Voxel-to-mesh conversion

### Integration Tests
- Spawn 20 cubes and verify all fall to ground
- Player can walk on world terrain
- Player can walk on fallen cubes
- Cubes stack realistically

### Performance Tests
- Benchmark collider generation (full vs. optimized)
- Measure frame time with varying cube counts
- Profile physics step time

## Future Extensions

Potential enhancements for later iterations:

1. **Advanced Collision Shapes**: Use TriMesh or ConvexHull for better accuracy
2. **LOD Collision**: Reduce collider detail for distant objects
3. **Networked Physics**: Sync physics state for multiplayer testing
4. **Destructible Voxels**: Break cubes on high-impact collisions
5. **Voxel Editing**: Allow runtime voxel placement/removal with dynamic collider updates
