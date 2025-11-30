# Implementation Tasks

## Phase 1: Project Setup
- [ ] Create `crates/proto` directory structure
- [ ] Add `crates/proto/Cargo.toml` with dependencies (bevy, bevy_rapier3d, cube, world, crossworld-physics, toml, glam)
- [ ] Create `crates/proto/config.toml` with default configuration (seed, macro_depth, micro_depth, spawn_count, models_path)
- [ ] Add proto crate to workspace in root `Cargo.toml`
- [ ] Add `just proto` task to `justfile` for running the prototype

## Phase 2: Enhanced Physics Collider System
- [ ] Add `bevy` feature flag to `crates/physics/Cargo.toml` for native-only code
- [ ] Create `crates/physics/src/native.rs` module for Bevy-specific physics utilities
- [ ] Implement optimized `VoxelColliderBuilder::from_cube_region()` that accepts AABB bounds for spatial filtering
- [ ] Add `VoxelColliderBuilder::overlapping_faces()` to traverse only faces within overlap region
- [ ] Write unit tests for spatial filtering collision generation
- [ ] Add documentation for optimized collider generation API

## Phase 3: Application Scaffold
- [ ] Create `crates/proto/src/main.rs` with Bevy app initialization
- [ ] Add Bevy default plugins and RapierPhysicsPlugin
- [ ] Implement config loading from `config.toml`
- [ ] Create startup system to initialize world, camera, and lighting
- [ ] Add camera orbit/free-fly controls (reuse patterns from editor)
- [ ] Implement debug info overlay (FPS, entity count, physics stats)

## Phase 4: World Generation
- [ ] Create `WorldCube` from config parameters (macro_depth, micro_depth, seed)
- [ ] Generate world mesh using `generate_frame()`
- [ ] Spawn world as static Bevy entity with mesh and collider
- [ ] Use `VoxelColliderBuilder::from_cube()` for world collision
- [ ] Add visual ground plane for reference
- [ ] Verify world renders correctly

## Phase 5: Voxel Object System
- [ ] Implement system to load .vox files from `packages/app/dist/assets/models/vox/`
- [ ] Create `VoxelObject` component with Cube data and metadata
- [ ] Implement random .vox model selection from directory
- [ ] Create helper function to convert Cube to Bevy Mesh (reuse editor pattern)
- [ ] Add material/color conversion for voxel rendering
- [ ] Test .vox loading with 3-5 different models

## Phase 6: Dynamic Cube Spawning
- [ ] Create spawn system to place cubes at random positions in air
- [ ] Generate random Y positions (e.g., 20-50 units above ground)
- [ ] Generate random X/Z positions within world bounds
- [ ] Add random rotation for visual variety
- [ ] Spawn specified count from config (`spawn_count` parameter)
- [ ] Attach RigidBody (Dynamic), Collider, and Mesh to each cube entity

## Phase 7: Optimized Collision Integration
- [ ] Integrate `VoxelColliderBuilder::from_cube_region()` for cube objects
- [ ] Initially use full collision (no optimization) as baseline
- [ ] Implement AABB calculation for each cube entity
- [ ] Add system to detect potential collisions via AABB overlap
- [ ] Apply spatial filtering: only generate collider faces in overlap region
- [ ] Benchmark performance difference (full vs. optimized collision)
- [ ] Log collision face count reduction in debug overlay

## Phase 8: Player Character System
- [ ] Create `PlayerCube` component to mark player entity
- [ ] Load a specific .vox model for player (or use simple colored cube)
- [ ] Implement character controller using Rapier's KinematicCharacterController
- [ ] Add WASD movement controls
- [ ] Add jump with spacebar
- [ ] Apply gravity to character controller
- [ ] Add camera follow system (orbit around player or first-person)
- [ ] Prevent player from falling through world

## Phase 9: Physics Simulation
- [ ] Configure RapierPhysicsPlugin with appropriate timestep
- [ ] Verify cubes fall with gravity
- [ ] Test cube-to-cube collisions (stacking behavior)
- [ ] Test cube-to-world collisions (landing on terrain)
- [ ] Test player-to-cube collisions
- [ ] Test player-to-world collisions
- [ ] Add restitution/friction parameters to colliders
- [ ] Tune physics parameters for realistic behavior

## Phase 10: Testing and Validation
- [ ] Run with 10 cubes and verify stable 60 FPS
- [ ] Run with 50 cubes and measure performance
- [ ] Test with different world depths (3-7)
- [ ] Test with different .vox model sizes (small/medium/large)
- [ ] Verify character can walk on fallen cubes
- [ ] Verify character can navigate world terrain
- [ ] Test edge cases (cubes spawned at world boundary)
- [ ] Document performance characteristics in README

## Phase 11: Documentation
- [ ] Create `crates/proto/README.md` with overview and usage
- [ ] Document config.toml parameters
- [ ] Add example configurations (minimal, stress-test, demo)
- [ ] Document optimized collision algorithm in physics crate
- [ ] Add inline code comments for complex systems
- [ ] Update root CLAUDE.md with proto information
