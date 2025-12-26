# Capability: Bevy Physics Prototype

## ADDED Requirements

### Requirement: Application Scaffold
The prototype SHALL be a standalone Bevy 0.17.3 application executable named `proto` that initializes the physics engine, loads configuration, and provides a 3D viewport for physics simulation.

#### Scenario: Launch prototype application
- **WHEN** user runs `cargo run --bin proto` or `just proto`
- **THEN** a window opens with title "Crossworld Physics Prototype"
- **AND** the window contains a 3D viewport showing generated world terrain
- **AND** the camera is positioned to view the scene center
- **AND** debug overlay displays FPS and entity count

#### Scenario: Load configuration from TOML
- **WHEN** application starts
- **THEN** config.toml is loaded from crate root
- **AND** world parameters (macro_depth, micro_depth, seed) are applied
- **AND** physics parameters (gravity, timestep) are configured
- **AND** spawning parameters (spawn_count, models_path) are stored
- **AND** missing config uses sensible defaults

#### Scenario: Application runs at stable framerate
- **WHEN** prototype is running with 10 voxel cubes
- **THEN** the application maintains at least 60 FPS
- **AND** frame time is displayed in debug overlay
- **AND** physics simulation runs smoothly

### Requirement: Configuration System
The prototype SHALL load world and physics parameters from a TOML configuration file located at the crate root.

#### Scenario: Default configuration
- **WHEN** config.toml does not exist
- **THEN** application uses default values (macro_depth=3, micro_depth=4, seed=12345, spawn_count=20)
- **AND** application logs warning about missing config
- **AND** application continues to run successfully

#### Scenario: Parse world configuration
- **WHEN** config.toml contains `[world]` section with macro_depth=5, micro_depth=6, seed=99999
- **THEN** WorldCube is created with those parameters
- **AND** world terrain reflects the specified seed
- **AND** world depth accommodates the total depth (11)

#### Scenario: Parse physics configuration
- **WHEN** config.toml contains `[physics]` section with gravity=-12.0, timestep=0.01666
- **THEN** RapierPhysicsPlugin is configured with gravity vector (0, -12.0, 0)
- **AND** physics simulation uses timestep of 1/60 second
- **AND** objects fall faster than default gravity

#### Scenario: Parse spawning configuration
- **WHEN** config.toml contains `[spawning]` with spawn_count=50, models_path="packages/app/dist/assets/models/vox/"
- **THEN** 50 voxel cubes are spawned at startup
- **AND** .vox models are loaded from the specified path
- **AND** cubes are positioned at random heights and horizontal positions

#### Scenario: Validate configuration parameters
- **WHEN** config.toml contains invalid values (negative depth, spawn_count > 1000)
- **THEN** application logs validation errors
- **AND** invalid parameters are clamped to valid ranges
- **AND** application continues with corrected values

### Requirement: World Generation
The prototype SHALL generate a voxel world using WorldCube from the world crate and render it as a static physics object.

#### Scenario: Generate procedural terrain
- **WHEN** application starts with config macro_depth=3, micro_depth=4, seed=12345
- **THEN** WorldCube is created with specified parameters
- **AND** generate_frame() produces terrain mesh
- **AND** terrain is visible in viewport as textured voxel surface

#### Scenario: Create world collider
- **WHEN** world mesh is generated
- **THEN** VoxelColliderBuilder::from_cube() creates compound collider
- **AND** world entity is spawned with RigidBody::Fixed
- **AND** world collider enables physics interactions
- **AND** objects can land on world terrain

#### Scenario: World bounds
- **WHEN** world is generated with macro_depth=3
- **THEN** world size is 2^3 = 8 units per axis at base
- **AND** world bounds are clearly visible
- **AND** spawned objects remain within or above world bounds

### Requirement: Voxel Object Loading
The prototype SHALL load random .vox models from the configured directory and convert them to voxel cubes for physics simulation.

#### Scenario: Scan vox models directory
- **WHEN** application starts with models_path="packages/app/dist/assets/models/vox/"
- **THEN** all .vox files in directory are discovered
- **AND** file list is logged (count and filenames)
- **AND** at least 5 models are available for random selection

#### Scenario: Load vox file to Cube
- **WHEN** system loads "chr_headphones.vox"
- **THEN** bytes are read from filesystem
- **AND** load_vox_to_cube() parses voxel data
- **AND** Cube<i32> is created with material indices
- **AND** cube depth matches model dimensions

#### Scenario: Random model selection
- **WHEN** spawning 20 cubes with 10 available models
- **THEN** each cube is assigned a randomly selected model
- **AND** models are distributed approximately evenly
- **AND** same model can appear multiple times

#### Scenario: Handle missing models directory
- **WHEN** models_path points to non-existent directory
- **THEN** application logs error message
- **AND** application falls back to simple colored cube primitives
- **AND** spawning continues with fallback cubes

### Requirement: Dynamic Cube Spawning
The prototype SHALL spawn voxel cube entities at random positions in the air with dynamic physics enabled.

#### Scenario: Spawn cubes at startup
- **WHEN** application starts with spawn_count=20
- **THEN** 20 voxel cube entities are created
- **AND** each cube has a Mesh3d component with voxel geometry
- **AND** each cube has a RigidBody::Dynamic component
- **AND** each cube has a Collider component

#### Scenario: Random initial positions
- **WHEN** cubes are spawned with min_height=20, max_height=50, spawn_radius=30
- **THEN** each cube Y position is between 20 and 50
- **AND** each cube X position is between -30 and 30
- **AND** each cube Z position is between -30 and 30
- **AND** positions are uniformly distributed

#### Scenario: Random initial rotations
- **WHEN** cubes are spawned
- **THEN** each cube has a random rotation quaternion
- **AND** rotations are visually varied
- **AND** rotations affect falling behavior

#### Scenario: Cube collider generation
- **WHEN** cube is spawned with voxel data
- **THEN** VoxelColliderBuilder::from_cube() generates collider
- **AND** collider is a compound shape of face rectangles
- **AND** collider matches voxel geometry accurately

### Requirement: Optimized Collision Detection
The physics crate SHALL provide optimized voxel collider generation that only processes faces within overlapping volume regions.

#### Scenario: Full collision baseline
- **WHEN** VoxelColliderBuilder::from_cube() is called without region filter
- **THEN** all exposed voxel faces are processed
- **AND** compound collider contains all face rectangles
- **AND** performance baseline is established

#### Scenario: Spatial filtering with AABB region
- **WHEN** VoxelColliderBuilder::from_cube_region() is called with AABB bounds
- **THEN** only voxels within AABB are processed
- **AND** voxels outside AABB are skipped during traversal
- **AND** resulting collider contains fewer face rectangles

#### Scenario: Calculate AABB overlap region
- **WHEN** two cube AABBs overlap with 30% volume intersection
- **THEN** intersection AABB is calculated (min/max bounds)
- **AND** only faces in intersection region are generated for each cube
- **AND** face count is reduced by approximately 70%

#### Scenario: No overlap optimization
- **WHEN** two cube AABBs do not overlap
- **THEN** no collision processing is performed
- **AND** early exit avoids unnecessary work

#### Scenario: Performance measurement
- **WHEN** comparing full vs. optimized collision for 10 cubes
- **THEN** optimized approach reduces total face count by 70%+ on average
- **AND** collider generation time is reduced proportionally
- **AND** improvement is logged in debug overlay

### Requirement: Physics Simulation
The prototype SHALL simulate realistic physics for falling voxel cubes using Rapier3D engine.

#### Scenario: Apply gravity to dynamic cubes
- **WHEN** cubes are spawned at height 30 with gravity=-9.81
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

#### Scenario: Physics timestep accuracy
- **WHEN** physics timestep is 1/60 second
- **THEN** simulation steps occur at 60 Hz
- **AND** simulation remains synchronized with render framerate
- **AND** no visible timestep artifacts (tunneling, jitter)

### Requirement: Player Character System
The prototype SHALL provide a player-controlled character cube with physics-based movement and collision.

#### Scenario: Spawn player character
- **WHEN** application starts
- **THEN** player cube entity is created at position (0, 10, 0)
- **AND** player has KinematicCharacterController component
- **AND** player has capsule collider for simplified collision
- **AND** player mesh is a distinct voxel cube or colored primitive

#### Scenario: WASD movement controls
- **WHEN** user presses W key
- **THEN** player moves forward in camera facing direction
- **AND** movement speed matches config (default 5.0 units/sec)
- **WHEN** user presses S key
- **THEN** player moves backward
- **WHEN** user presses A key
- **THEN** player strafes left
- **WHEN** user presses D key
- **THEN** player strafes right

#### Scenario: Jump control
- **WHEN** player is on ground and user presses Space
- **THEN** upward impulse is applied (default 8.0 units)
- **AND** player lifts off ground
- **AND** gravity pulls player back down
- **AND** player cannot jump again until landing

#### Scenario: Player-world collision
- **WHEN** player walks into world terrain
- **THEN** player is blocked by terrain collider
- **AND** player cannot walk through solid voxels
- **AND** player can walk up gentle slopes

#### Scenario: Player-cube collision
- **WHEN** player walks into a fallen cube
- **THEN** player is blocked by cube collider
- **AND** player cannot push dynamic cubes (kinematic vs. dynamic)
- **AND** player can walk on top of cubes

#### Scenario: Player gravity
- **WHEN** player walks off an edge
- **THEN** player falls with gravity
- **AND** player can be controlled mid-air
- **AND** player lands on lower surface

### Requirement: Camera Controls
The prototype SHALL provide camera controls for viewing the physics simulation from different perspectives.

#### Scenario: Orbit camera mode
- **WHEN** camera mode is orbit (default)
- **AND** user right-click drags
- **THEN** camera orbits around scene center
- **AND** camera always faces toward center
- **AND** scroll wheel adjusts orbit distance

#### Scenario: Free-fly camera mode
- **WHEN** camera mode is free-fly (toggled with C key)
- **AND** user right-click drags
- **THEN** camera rotates in place (FPS-style)
- **AND** WASD keys move camera position
- **AND** camera can move independently of scene

#### Scenario: Camera follows player
- **WHEN** camera mode is follow (toggled with F key)
- **THEN** camera position tracks player position with offset
- **AND** camera looks at player
- **AND** camera smoothly interpolates position changes

#### Scenario: Toggle camera modes
- **WHEN** user presses C key
- **THEN** camera mode cycles: orbit → free-fly → follow → orbit
- **AND** UI displays current camera mode
- **AND** camera behavior changes immediately

### Requirement: Debug Overlay
The prototype SHALL display real-time debug information to monitor performance and physics state.

#### Scenario: Display performance metrics
- **WHEN** application is running
- **THEN** FPS is displayed in top-left corner
- **AND** frame time (ms) is displayed
- **AND** entity count is displayed
- **AND** metrics update every frame

#### Scenario: Display physics statistics
- **WHEN** physics simulation is active
- **THEN** active rigid bodies count is displayed
- **AND** total collider face count is displayed
- **AND** optimized face count reduction percentage is displayed

#### Scenario: Display player information
- **WHEN** player character exists
- **THEN** player position (X, Y, Z) is displayed
- **AND** player velocity is displayed
- **AND** grounded state is displayed (on ground / airborne)

#### Scenario: Toggle debug overlay
- **WHEN** user presses F3 key
- **THEN** debug overlay visibility toggles on/off
- **AND** overlay state persists until next toggle

### Requirement: Build Configuration
The prototype SHALL use appropriate Bevy build optimizations for fast iteration during development.

#### Scenario: Linux build uses dynamic linking
- **WHEN** developer builds on Linux in debug mode
- **THEN** Bevy is dynamically linked (if supported)
- **AND** incremental build times are reduced

#### Scenario: Prototype crate dependencies
- **WHEN** developer inspects Cargo.toml
- **THEN** dependencies include: bevy 0.17.3, bevy_rapier3d, cube, crossworld-world, crossworld-physics, toml, glam
- **AND** crossworld-physics includes "bevy" feature flag
- **AND** all dependencies are from workspace or crates.io
