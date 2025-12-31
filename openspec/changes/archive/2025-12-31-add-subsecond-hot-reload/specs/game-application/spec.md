# Spec: Game Application

## ADDED Requirements

### Requirement: Game Crate as Dynamic Library
The game crate SHALL compile as both a dynamic library (for hot-reload) and a static library (for release builds).

#### Scenario: Dynamic library compilation
- **WHEN** building in development mode
- **THEN** `crates/game` SHALL produce a `libgame.so` (Linux), `libgame.dylib` (macOS), or `game.dll` (Windows)
- **AND** export the necessary symbols for the runtime to load

#### Scenario: Static library compilation
- **WHEN** building in release mode
- **THEN** `crates/game` SHALL produce a static library for linking
- **AND** the runtime SHALL use static linking instead of dynamic loading

### Requirement: App Trait Implementation
The game crate SHALL implement the `App` trait from `crates/app`.

#### Scenario: Rotating cube demo implementation
- **WHEN** the rotating cube demo is loaded
- **THEN** it SHALL implement all five App trait methods
- **AND** render a textured or colored 3D cube
- **AND** rotate the cube continuously in the update method

### Requirement: OpenGL Resource Management
The game code SHALL properly initialize and cleanup all OpenGL resources.

#### Scenario: Resource initialization in init
- **WHEN** `App::init` is called
- **THEN** the game SHALL create vertex buffers, shaders, and textures needed for rendering
- **AND** store their handles for use in subsequent frames
- **AND** verify all GL calls succeed (check `glGetError` in debug mode)

#### Scenario: Resource cleanup in uninit
- **WHEN** `App::uninit` is called before a hot-reload
- **THEN** the game SHALL delete all OpenGL objects it created
- **AND** free any allocated memory
- **AND** ensure no resources leak across reloads

### Requirement: Rotating Cube Rendering
The rotating cube demo SHALL render a visible 3D cube with rotation animation.

#### Scenario: Initial cube rendering
- **WHEN** the game is first loaded
- **THEN** a cube SHALL be visible in the center of the window
- **AND** the cube SHALL have distinct colors or textures on each face
- **AND** basic perspective projection SHALL be applied

#### Scenario: Rotation animation
- **WHEN** `App::update` is called each frame
- **THEN** the cube's rotation angle SHALL increment based on delta time
- **AND** the rotation SHALL be smooth and visible

#### Scenario: Camera view
- **WHEN** rendering the cube
- **THEN** a camera SHALL be positioned to view the cube from an angle
- **AND** the camera position MAY be fixed or controlled by keyboard/mouse (future enhancement)

### Requirement: Hot-Reload Verification
The rotating cube demo SHALL serve as a test case for hot-reload functionality.

#### Scenario: Modify rotation speed
- **WHEN** the rotation speed constant is changed in source code
- **THEN** the change SHALL be visible within 1 second of saving the file
- **AND** the cube SHALL rotate at the new speed without restarting the application

#### Scenario: Modify cube color
- **WHEN** the cube's color or texture is changed in source code
- **THEN** the visual change SHALL be reflected immediately after hot-reload
- **AND** the cube's rotation SHALL continue smoothly

#### Scenario: Introduce compile error
- **WHEN** a syntax error is introduced in the game code
- **THEN** the hot-reload SHALL fail gracefully
- **AND** the previous working version SHALL continue running
- **AND** an error message SHALL be displayed in the console

### Requirement: Shader Management
The game SHALL compile and use GLSL shaders for rendering.

#### Scenario: Shader compilation in init
- **WHEN** `App::init` is called
- **THEN** the game SHALL compile vertex and fragment shaders
- **AND** link them into a shader program
- **AND** log any shader compilation errors

#### Scenario: Shader hot-reload support
- **WHEN** shader source code is modified (future enhancement)
- **THEN** the shader MAY be recompiled without reloading the entire game library
- **AND** shader compilation errors SHALL be reported without crashing

### Requirement: State Reset Behavior on Reload
The game SHALL reset to initial state on hot-reload unless explicit persistence is implemented.

#### Scenario: Default state reset
- **WHEN** a hot-reload occurs without state persistence implementation
- **THEN** the cube SHALL reset to initial rotation angle
- **AND** this is the expected default behavior

#### Scenario: Optional state persistence (future enhancement)
- **WHEN** state persistence is implemented in future versions
- **THEN** the cube's current rotation angle SHALL be preserved across reloads
- **AND** rotation SHALL continue from the same position after reload
