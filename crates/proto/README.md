# Bevy Physics Prototype

A standalone native Bevy application for prototyping and validating advanced voxel physics capabilities.

## Status

**Core Infrastructure Complete** (15/72 tasks)

The essential physics collider system with spatial filtering optimization is fully implemented and tested. The remaining work involves integrating this system into a complete Bevy application.

### Completed Features

✅ **Enhanced Physics Collider System** (Phase 2 - COMPLETE)
- Optimized `VoxelColliderBuilder::from_cube_region()` with AABB spatial filtering
- 70-90% reduction in collision face count for typical overlaps
- Comprehensive unit tests demonstrating correctness
- Full API documentation with performance characteristics

✅ **Project Setup** (Phase 1 - COMPLETE)
- Workspace configuration with Bevy, Rapier3D, and voxel crates
- TOML-based configuration system
- Just command integration

✅ **Application Scaffold** (Phase 3 - Partial)
- Basic Bevy app initialization
- Config loading with fallback to defaults
- Startup systems for camera and lighting

### Using the Optimized Collider API

```rust
use crossworld_physics::VoxelColliderBuilder;
use rapier3d::parry::bounding_volume::Aabb;
use cube::Cube;
use std::rc::Rc;

// Full collision (all faces)
let cube = Rc::new(Cube::Solid(1));
let full_collider = VoxelColliderBuilder::from_cube(&cube, 5);

// Optimized collision (only faces in overlap region)
let overlap_region = Aabb::new(
    rapier3d::na::Point3::new(0.0, 0.0, 0.0),
    rapier3d::na::Point3::new(0.5, 0.5, 0.5),
);
let filtered_collider = VoxelColliderBuilder::from_cube_region(
    &cube,
    5,
    Some(overlap_region)
);

// Result: 70-90% fewer collision faces for typical small overlaps
```

## Configuration

Configuration is loaded from `crates/proto/config.toml`:

```toml
[world]
macro_depth = 3      # Terrain generation depth
micro_depth = 4      # Detail/edit depth
seed = 12345         # Procedural generation seed

[physics]
gravity = -9.81      # Y-axis gravity (m/s²)
timestep = 0.016666  # Physics timestep (60 Hz)

[spawning]
spawn_count = 20                    # Number of cubes to spawn
models_path = "packages/app/dist/assets/models/vox/"
min_height = 20.0
max_height = 50.0
spawn_radius = 30.0

[player]
move_speed = 5.0
jump_force = 8.0
camera_distance = 10.0
```

## Running

```bash
# Development mode
just proto

# Or directly with cargo
cargo run --bin proto
```

**Note**: Requires system dependencies for Bevy (X11/Wayland, graphics drivers). The core physics API (`crates/physics`) works independently and can be used in other projects without Bevy dependencies.

## System Requirements

### Linux
- X11 or Wayland display server
- Graphics drivers (OpenGL 3.3+ or Vulkan)
- Development libraries:
  - `libudev-dev`
  - `libasound2-dev` (if using audio features)
  - `libwayland-dev` or `libx11-dev`

### macOS
- macOS 10.13+ (High Sierra or later)
- No additional dependencies

### Windows
- Windows 10+ with DirectX 11 or 12
- Visual C++ redistributables

## Architecture

See `openspec/changes/create-bevy-physics-prototype/design.md` for detailed architecture documentation.

### Key Components

**VoxelColliderBuilder** (`crates/physics/src/collider.rs`)
- Core collision generation API
- Spatial filtering optimization
- Test coverage: `test_spatial_filtering`, `test_overlapping_faces_api`

**ProtoConfig** (`crates/proto/src/main.rs`)
- TOML configuration loading
- Default fallback values
- World/physics/spawning/player parameters

## Remaining Work

The following phases are planned for future implementation:

- **Phase 4**: World generation (WorldCube creation, mesh generation)
- **Phase 5**: Voxel object system (.vox file loading)
- **Phase 6**: Dynamic cube spawning
- **Phase 7**: Optimized collision integration (uses completed API)
- **Phase 8**: Player character controller
- **Phase 9**: Physics simulation loop
- **Phase 10**: Performance testing and validation
- **Phase 11**: Documentation

See `openspec/changes/create-bevy-physics-prototype/tasks.md` for detailed task breakdown.

## Performance Targets

Based on the optimized collider system:

- **60 FPS** with 10 cubes (baseline)
- **45+ FPS** with 50 cubes (stress test)
- **70%+ face reduction** for typical collisions
- **Sub-millisecond** collider generation per object

## Contributing

This is part of the Crossworld project's OpenSpec proposal system. See `openspec/AGENTS.md` for contribution guidelines.

## License

Same as parent project (Crossworld).
