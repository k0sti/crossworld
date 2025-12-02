# Capability: Proto-GL Physics Viewer

## ADDED Requirements

### Requirement: Application Scaffold
The proto-gl viewer SHALL be a standalone native OpenGL application executable named `proto-gl` that initializes the physics engine, loads configuration, and provides a 3D viewport for physics simulation.

#### Scenario: Launch viewer application
- **WHEN** user runs `cargo run --bin proto-gl` or `just proto-gl`
- **THEN** a window opens with title "Proto-GL Physics Viewer"
- **AND** the window contains a 3D GL viewport showing generated world terrain
- **AND** the camera is positioned to view the scene center
- **AND** egui side panel displays configuration and statistics

#### Scenario: Load configuration from TOML
- **WHEN** application starts
- **THEN** config.toml is loaded from crate root (`crates/proto-gl/config.toml`)
- **AND** world parameters (macro_depth, micro_depth, border_depth, root_cube) are applied
- **AND** physics parameters (gravity, timestep) are configured
- **AND** spawning parameters (spawn_count, models_path, heights, radius) are stored
- **AND** rendering parameters (viewport size, camera distance) are applied
- **AND** missing config uses sensible defaults

#### Scenario: Fast build times
- **WHEN** developer runs clean build with `cargo build --bin proto-gl`
- **THEN** build completes in less than 20 seconds
- **AND** incremental builds complete in less than 5 seconds
- **AND** build is significantly faster than Bevy-based proto

#### Scenario: Application runs at stable framerate
- **WHEN** viewer is running with 10 cube objects
- **THEN** the application maintains at least 60 FPS
- **AND** frame time is displayed in egui panel
- **AND** physics simulation runs smoothly

### Requirement: Configuration System
The viewer SHALL load world, physics, spawning, and rendering parameters from a TOML configuration file.

#### Scenario: Default configuration
- **WHEN** config.toml does not exist
- **THEN** application uses default values from code
- **AND** defaults match: macro_depth=3, micro_depth=4, border_depth=1, spawn_count=10, gravity=-9.81
- **AND** application logs warning about missing config
- **AND** application continues to run successfully

#### Scenario: Parse world configuration with CSM
- **WHEN** config.toml contains `[world]` with root_cube CSM string
- **THEN** CSM is parsed using `cube::parse_csm()`
- **AND** resulting Cube is used as world root
- **AND** border layers are applied if border_depth > 0
- **AND** world renders in viewport

#### Scenario: Parse rendering configuration
- **WHEN** config.toml contains `[rendering]` section
- **THEN** viewport_width and viewport_height set GL viewport size
- **AND** camera_distance sets initial orbit distance
- **AND** rendering parameters are applied immediately

#### Scenario: Validate configuration parameters
- **WHEN** config.toml contains invalid values (negative depth, spawn_count > 1000)
- **THEN** application logs validation errors
- **AND** invalid parameters are clamped to valid ranges
- **AND** application continues with corrected values

### Requirement: World Generation and Rendering
The viewer SHALL generate a voxel world from CSM using direct Cube types and render it using GlCubeTracer.

#### Scenario: Parse CSM and create Cube
- **WHEN** application starts with config root_cube=">a [5 5 4 9 5 5 0 0]"
- **THEN** `cube::parse_csm()` parses the CSM string
- **AND** octree root is extracted as `Cube<u8>`
- **AND** Cube is stored for rendering and physics

#### Scenario: Apply border layers
- **WHEN** config.border_depth = 1
- **THEN** `add_border_layers()` wraps cube in border structure
- **AND** border materials from config define vertical layers
- **AND** world size doubles per border layer
- **AND** resulting cube includes border geometry

#### Scenario: Generate and render world mesh
- **WHEN** world Cube is ready
- **THEN** `generate_face_mesh()` creates mesh geometry
- **AND** vertices are scaled to world coordinates
- **AND** `GlCubeTracer` renders mesh in GL viewport
- **AND** world appears as solid voxel terrain

#### Scenario: Create world physics collider
- **WHEN** world Cube is generated
- **THEN** `VoxelColliderBuilder::from_cube()` creates compound collider
- **AND** world is added as RigidBody::Fixed to physics world
- **AND** world collider enables physics interactions
- **AND** falling objects can land on world terrain

### Requirement: CubeObject System
The viewer SHALL load voxel models from .vox files and manage them as CubeObjects with physics properties.

#### Scenario: CubeObject structure
- **WHEN** system creates a CubeObject
- **THEN** CubeObject contains Cube<u8> for voxel data
- **AND** contains RigidBodyHandle for physics simulation
- **AND** contains ColliderHandle for collision detection
- **AND** contains model_name for identification
- **AND** contains depth for rendering detail

#### Scenario: Load .vox files from directory
- **WHEN** application starts with models_path="assets/models/"
- **THEN** all .vox files in directory are discovered
- **AND** each file is loaded using `load_vox_to_cube()`
- **AND** resulting Cubes are stored with metadata
- **AND** file list is logged (count and filenames)

#### Scenario: Random model selection
- **WHEN** spawning 10 cubes with 5 available models
- **THEN** each cube is assigned a randomly selected model
- **AND** models are distributed approximately evenly
- **AND** same model can appear multiple times

#### Scenario: Handle missing models directory
- **WHEN** models_path points to non-existent directory
- **THEN** application logs error message
- **AND** falls back to simple colored cube primitives (solid Cube::Solid)
- **AND** spawning continues with fallback cubes
- **AND** physics simulation works normally

### Requirement: Dynamic Cube Spawning
The viewer SHALL spawn CubeObject entities at random positions in the air with dynamic physics enabled.

#### Scenario: Spawn cubes at startup
- **WHEN** application starts with spawn_count=10
- **THEN** 10 CubeObject entities are created
- **AND** each has a Cube<u8> with voxel geometry
- **AND** each has a RigidBody::Dynamic component
- **AND** each has a Collider from VoxelColliderBuilder

#### Scenario: Random initial positions
- **WHEN** cubes are spawned with min_height=10, max_height=30, spawn_radius=20
- **THEN** each cube Y position is between 10 and 30
- **AND** each cube X position is between -20 and 20
- **AND** each cube Z position is between -20 and 20
- **AND** positions are uniformly distributed

#### Scenario: Random initial rotations
- **WHEN** cubes are spawned
- **THEN** each cube has a random rotation quaternion
- **AND** rotations are visually varied
- **AND** rotations affect falling behavior

#### Scenario: Cube collider generation
- **WHEN** cube is spawned with voxel data
- **THEN** `VoxelColliderBuilder::from_cube()` generates collider
- **AND** collider is a compound shape of face rectangles
- **AND** collider matches voxel geometry accurately
- **AND** collider is attached to rigid body

### Requirement: Physics Simulation
The viewer SHALL simulate realistic physics for falling voxel cubes using Rapier3D engine.

#### Scenario: Initialize physics world
- **WHEN** application starts
- **THEN** Rapier3D IntegrationParameters are configured
- **AND** gravity vector is set from config (default [0, -9.81, 0])
- **AND** timestep is set from config (default 1/60 second)
- **AND** RigidBodySet and ColliderSet are initialized

#### Scenario: Fixed timestep accumulator
- **WHEN** render loop runs
- **THEN** frame delta is accumulated
- **AND** physics steps run when accumulator >= timestep
- **AND** physics runs at consistent 60 Hz regardless of frame rate
- **AND** remaining time is carried over to next frame

#### Scenario: Apply gravity to dynamic cubes
- **WHEN** cubes are spawned at height 20 with gravity=-9.81
- **THEN** cubes begin falling immediately
- **AND** falling speed increases over time (acceleration)
- **AND** cubes reach terminal velocity based on physics parameters

#### Scenario: Cube lands on world terrain
- **WHEN** falling cube contacts world collider
- **THEN** cube stops falling and rests on terrain surface
- **AND** cube orientation adjusts based on collision angle
- **AND** cube does not penetrate terrain

#### Scenario: Cube-to-cube collision
- **WHEN** two falling cubes collide in midair
- **THEN** both cubes bounce off each other
- **AND** collision impulse affects velocities
- **AND** rotation is induced by off-center impacts

#### Scenario: Stacking behavior
- **WHEN** multiple cubes land on same terrain region
- **THEN** cubes stack on top of each other
- **AND** stack remains stable (no excessive jitter)
- **AND** bottom cubes support weight of top cubes

### Requirement: OpenGL Rendering
The viewer SHALL render voxel geometry using the GlCubeTracer from the renderer crate.

#### Scenario: Render static world
- **WHEN** world Cube is loaded
- **THEN** `GlCubeTracer.render()` is called with world cube
- **AND** cube is rendered at identity transform (0, 0, 0)
- **AND** voxels appear in correct world positions
- **AND** materials display correct colors

#### Scenario: Render dynamic cubes at physics positions
- **WHEN** physics simulation runs
- **THEN** for each CubeObject, rigid body transform is queried
- **AND** `GlCubeTracer.render_transformed()` renders cube at physics position
- **AND** rotation quaternion is applied to cube rendering
- **AND** cubes appear at correct positions in 3D space

#### Scenario: Camera view matrix
- **WHEN** camera orbit parameters change
- **THEN** view matrix is recalculated from yaw, pitch, distance
- **AND** GlCubeTracer uses updated view matrix
- **AND** scene is rendered from new camera perspective

### Requirement: Camera Controls
The viewer SHALL provide orbit camera controls for viewing the physics simulation.

#### Scenario: Orbit camera mode
- **WHEN** camera is in orbit mode (default and only mode)
- **AND** user right-click drags mouse
- **THEN** camera orbits around scene focus point (0, 0, 0)
- **AND** camera always faces toward focus
- **AND** yaw and pitch are updated from mouse delta

#### Scenario: Mouse wheel zoom
- **WHEN** user scrolls mouse wheel
- **THEN** camera distance increases/decreases
- **AND** distance is clamped to range [5.0, 100.0]
- **AND** zoom is smooth and responsive

#### Scenario: Pitch clamping
- **WHEN** user drags camera vertically
- **THEN** pitch is clamped to range [-1.5, 1.5] radians
- **AND** camera cannot flip upside down
- **AND** gimbal lock is avoided

### Requirement: egui UI Panel
The viewer SHALL display real-time information and controls in an egui side panel.

#### Scenario: Display performance metrics
- **WHEN** application is running
- **THEN** FPS is displayed in side panel
- **AND** frame time (ms) is displayed
- **AND** object count is displayed
- **AND** metrics update every frame

#### Scenario: Display configuration values
- **WHEN** side panel is visible
- **THEN** world depth (macro + micro) is displayed
- **AND** gravity value is displayed
- **AND** timestep value is displayed
- **AND** spawn count is displayed

#### Scenario: Reset scene button
- **WHEN** user clicks "Reset Scene" button
- **THEN** all dynamic CubeObjects are removed from physics world
- **AND** new cubes are spawned at random positions
- **AND** physics simulation continues with new cubes
- **AND** frame counter resets

#### Scenario: Panel layout
- **WHEN** application window is sized
- **THEN** egui side panel width is fixed at reasonable size (~250px)
- **AND** GL viewport fills remaining window space
- **AND** panel does not overlap viewport
- **AND** UI is readable and well-organized

### Requirement: Build Configuration
The viewer SHALL use the cube, crossworld-physics, and renderer crates for core functionality, with minimal additional dependencies for fast build times. The proto crate MAY be referenced for code examples and patterns but SHALL NOT be a runtime dependency.

#### Scenario: Core crate dependencies
- **WHEN** developer inspects Cargo.toml
- **THEN** dependencies include: cube (voxel data), crossworld-physics (Rapier3D), renderer (GL/egui)
- **AND** additional dependencies include: glow, glutin, winit, egui stack, toml, serde, glam, rand
- **AND** proto crate is NOT listed as a dependency
- **AND** proto patterns (physics setup, spawning) are referenced for implementation guidance

#### Scenario: Minimal dependency tree
- **WHEN** developer inspects dependency count
- **THEN** no Bevy dependencies are present
- **AND** only essential GL and physics libraries are included
- **AND** total dependency count < 50

#### Scenario: Incremental build performance
- **WHEN** developer changes a single source file
- **THEN** incremental rebuild completes in < 5 seconds
- **AND** only affected modules recompile
- **AND** build is significantly faster than Bevy proto

## MODIFIED Requirements

None. This is a new capability with no modifications to existing requirements.

## REMOVED Requirements

None. This capability does not remove any existing requirements.
