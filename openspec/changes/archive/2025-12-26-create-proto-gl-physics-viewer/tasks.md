# Implementation Tasks: Proto-GL Physics Viewer

## Phase 1: Project Setup (Estimated: 30min)
- [x] Create `crates/proto-gl` directory structure
- [x] Add `crates/proto-gl/Cargo.toml` with dependencies (glow, glutin, winit, egui stack, cube, crossworld-physics, toml, glam, rand)
- [x] Create `crates/proto-gl/config.toml` with default configuration
- [x] Add proto-gl crate to workspace in root `Cargo.toml`
- [x] Add `just proto-gl` task to `justfile` for running the viewer
- [x] Verify clean build completes in < 20 seconds

## Phase 2: Application Scaffold (Estimated: 1-2hr)
- [x] Create `crates/proto-gl/src/main.rs` with winit event loop
- [x] Initialize OpenGL context using glutin (copy pattern from crates/renderer)
- [x] Setup egui integration (egui-glow)
- [x] Create basic window with title "Proto-GL Physics Viewer"
- [x] Implement config loading from `config.toml` with fallback to defaults
- [x] Add `ProtoGlConfig` struct matching design.md structure
- [x] Verify application launches and shows empty window

## Phase 3: Rendering Integration (Estimated: 2-3hr)
- [x] Add `renderer` crate as dependency
- [x] Create `ProtoGlRenderer` struct wrapping `GlCubeTracer`
- [x] Implement `OrbitCamera` struct with view matrix calculation
- [x] Add mouse controls for camera orbit (right-click drag)
- [x] Add mouse wheel zoom
- [x] Create render loop that clears and presents frame
- [x] Verify camera controls work smoothly

## Phase 4: World Generation and Rendering (Estimated: 1-2hr)
- [x] Parse CSM from config.root_cube using `cube::parse_csm()`
- [x] Implement `add_border_layers()` function (copy from crates/proto)
- [x] Apply border layers based on config.border_depth
- [x] Generate mesh from cube using `generate_face_mesh()`
- [x] Render world cube in GL viewport using `GlCubeTracer`
- [x] Scale vertices to world coordinates
- [x] Verify world renders correctly in viewport
- [x] Fix shader compilation error (bvec3 logical operations)
- [x] Initialize GL tracer with init_gl() call
- [x] Add camera position/rotation calculation methods

## Phase 5: Physics World Setup (Estimated: 2-3hr)
- [x] Initialize Rapier3D physics world with gravity from config
- [x] Create `PhysicsWorld` struct (RigidBodySet, ColliderSet, etc.)
- [x] Generate world collider using `VoxelColliderBuilder::from_cube()`
- [x] Add world as Fixed rigid body to physics world
- [x] Implement fixed timestep accumulator (60 Hz)
- [x] Add physics step to main loop
- [x] Verify physics world initialized without errors

## Phase 6: CubeObject System (Estimated: 2-3hr)
- [x] Create `CubeObject` struct with cube, handles, metadata
- [x] Implement .vox file loading from config.models_path
- [x] Create `VoxModel` struct (cube, name, depth)
- [x] Implement directory scanning for .vox files
- [x] Add random model selection logic
- [x] Handle missing models directory gracefully (fallback to simple cubes)
- [x] Verify .vox models load correctly

## Phase 7: Dynamic Cube Spawning (Estimated: 1-2hr)
- [x] Implement `spawn_cube_objects()` function
- [x] Generate random positions (x, y, z) within configured bounds
- [x] Create RigidBody::Dynamic for each cube
- [x] Generate collider using `VoxelColliderBuilder::from_cube()`
- [ ] Add cubes to physics world with random rotations
- [x] Store CubeObjects in application state
- [ ] Verify spawn_count cubes appear in scene

## Phase 8: Physics Simulation (Estimated: 1-2hr)
- [x] Implement physics step in main loop (fixed timestep)
- [ ] Extract rigid body transforms (position, rotation)
- [ ] Update cube render positions from physics state
- [ ] Render dynamic cubes at their physics positions
- [ ] Verify cubes fall with gravity
- [ ] Test cube-to-world collisions (landing)
- [ ] Test cube-to-cube collisions (stacking)
- [ ] Tune physics parameters for realistic behavior

## Phase 9: UI Panel (Estimated: 1hr)
- [x] Create egui side panel with controls
- [x] Display FPS counter (calculate from frame delta)
- [x] Display object count
- [x] Display configuration values (world depth, gravity, etc.)
- [x] Add "Reset Scene" button
- [ ] Implement scene reset (respawn cubes)
- [x] Style UI panel for readability

## Phase 10: Testing and Validation (Estimated: 1-2hr)
- [ ] Test with 5, 10, 20, 50 cubes and measure FPS
- [ ] Verify build time < 15 seconds (clean build)
- [ ] Test with different world depths (3-7)
- [ ] Test with different .vox model sizes
- [ ] Test edge cases (zero cubes, huge spawn_radius)
- [ ] Test config file missing/invalid scenarios
- [ ] Document performance characteristics in README

## Phase 11: Documentation (Estimated: 30min)
- [ ] Create `crates/proto-gl/README.md` with overview
- [ ] Document config.toml parameters
- [ ] Add usage instructions (just proto-gl)
- [ ] Document camera controls
- [ ] Add example configurations
- [ ] Note differences from crates/proto

## Phase 12: Integration Testing (Estimated: 1hr)
- [ ] Compare physics behavior with crates/proto (Bevy version)
- [ ] Verify collision accuracy matches VoxelColliderBuilder tests
- [ ] Test with same configs as Bevy proto
- [ ] Document any behavioral differences
- [ ] Update root CLAUDE.md with proto-gl information

Total estimated time: 12-18 hours

## Dependencies

- Phase 3 depends on Phase 2 (app scaffold)
- Phase 4 depends on Phase 3 (rendering)
- Phase 5 runs in parallel with Phase 4 (physics can be setup independently)
- Phase 6 runs in parallel with Phase 5 (model loading independent)
- Phase 7 depends on Phase 5 and 6 (needs physics + models)
- Phase 8 depends on Phase 7 (needs spawned objects)
- Phase 9 depends on Phase 2 (needs egui setup)
- Phase 10 depends on Phase 8 (needs full system)
- Phase 11 and 12 depend on Phase 10 (needs validated system)

## Parallelizable Work

The following can be worked on simultaneously:
- Phase 4 (world rendering) + Phase 5 (physics setup)
- Phase 6 (model loading) + Phase 5 (physics setup)
- Phase 9 (UI) can start after Phase 2 (independent of rendering/physics)
