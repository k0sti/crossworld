# hot-reload-runtime Specification

## Purpose
TBD - created by archiving change add-subsecond-hot-reload. Update Purpose after archive.
## Requirements
### Requirement: Application Runtime Initialization
The runtime SHALL initialize a window, OpenGL context, and event loop before loading any game code.

#### Scenario: Successful runtime startup
- **WHEN** the application is launched
- **THEN** a window is created with OpenGL 4.3+ context
- **AND** the event loop is ready to process events
- **AND** no game code has been loaded yet

#### Scenario: OpenGL context creation failure
- **WHEN** OpenGL 4.3 is not available
- **THEN** the runtime SHALL attempt to create a context with the highest available version
- **AND** log the actual OpenGL version obtained

### Requirement: App Trait Definition
The runtime SHALL define an `App` trait with lifecycle methods: `init`, `uninit`, `event`, `update`, and `render`.

#### Scenario: App trait lifecycle contract
- **WHEN** a game implements the `App` trait
- **THEN** it MUST provide implementations for all lifecycle methods
- **AND** `init` SHALL be called once after loading the game library
- **AND** `uninit` SHALL be called once before unloading the game library
- **AND** `event` SHALL be called for each window/input event
- **AND** `update` SHALL be called each frame for game logic
- **AND** `render` SHALL be called each frame for rendering

#### Scenario: OpenGL context access in App methods
- **WHEN** `App::init`, `App::render`, or `App::uninit` are called
- **THEN** the game code SHALL have access to a valid OpenGL context
- **AND** the context SHALL remain current for the duration of the method call

### Requirement: Dynamic Library Loading
The runtime SHALL load the game code as a dynamic library (`.so`, `.dylib`, or `.dll`).

#### Scenario: Initial game library load
- **WHEN** the runtime starts
- **THEN** it SHALL load the game dynamic library from a configured path
- **AND** call `App::init` on the loaded game instance

#### Scenario: Game library not found
- **WHEN** the game library file does not exist at the configured path
- **THEN** the runtime SHALL exit with a clear error message
- **AND** indicate the expected library path

### Requirement: Hot-Reload Trigger
The runtime SHALL watch the game library file and trigger a reload when it changes.

#### Scenario: File modification detected
- **WHEN** the game library file is modified on disk
- **THEN** the runtime SHALL initiate the reload sequence within 100 milliseconds
- **AND** debounce multiple rapid writes (only reload after modifications stop)

#### Scenario: File watch error
- **WHEN** the file watcher encounters an error
- **THEN** the runtime SHALL log the error
- **AND** continue running with the currently loaded game code

### Requirement: Reload Sequence
The runtime SHALL execute a safe reload sequence preserving the OpenGL context and window state.

#### Scenario: Successful hot-reload
- **WHEN** a reload is triggered
- **THEN** the runtime SHALL call `App::uninit` on the current game instance
- **AND** unload the old game library
- **AND** load the new game library
- **AND** call `App::init` on the new game instance
- **AND** resume the event loop without disruption

#### Scenario: Reload with compile error
- **WHEN** a reload is triggered but the new library has compilation errors
- **THEN** the runtime SHALL keep the old game library loaded
- **AND** log the error information
- **AND** continue running the previous working version

#### Scenario: State preservation across reload
- **WHEN** a hot-reload occurs
- **THEN** the window SHALL remain open with the same dimensions
- **AND** the OpenGL context SHALL remain valid
- **AND** the game code is responsible for managing its own state persistence

### Requirement: Event Loop Integration
The runtime SHALL integrate the App trait callbacks into the winit event loop.

#### Scenario: Event dispatch to game
- **WHEN** a winit WindowEvent is received
- **THEN** the runtime SHALL call `App::event` with the event
- **AND** the game code MAY handle the event or ignore it

#### Scenario: Frame rendering cycle
- **WHEN** the event loop requests a redraw
- **THEN** the runtime SHALL call `App::update` with delta time
- **AND** call `App::render` with access to the OpenGL context
- **AND** swap the GL buffers after rendering completes

### Requirement: Error Handling and Recovery
The runtime SHALL handle errors gracefully without crashing the application.

#### Scenario: Panic in game code
- **WHEN** game code panics in any App method
- **THEN** the runtime SHALL catch the panic
- **AND** log the panic message and backtrace
- **AND** optionally attempt to reload the game library
- **AND** NOT crash the entire application

#### Scenario: Resource leak detection
- **WHEN** running in debug mode
- **THEN** the runtime SHOULD track OpenGL objects created during `App::init`
- **AND** verify they are deleted during `App::uninit`
- **AND** log warnings for any leaks detected

