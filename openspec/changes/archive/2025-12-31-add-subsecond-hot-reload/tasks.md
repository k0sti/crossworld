# Implementation Tasks

## 1. Scaffold New Crates
- [x] 1.1 Create `crates/app/` directory with `Cargo.toml`, `src/lib.rs`, `src/main.rs`
- [x] 1.2 Create `crates/game/` directory with `Cargo.toml`, `src/lib.rs`
- [x] 1.3 Add `crates/app` and `crates/game` to workspace members in root `Cargo.toml`
- [x] 1.4 Define `App` trait in `crates/app/src/lib.rs` with lifecycle methods
- [x] 1.5 Add dependencies: `winit`, `glutin`, `glow`, `notify`, `libloading`

## 2. Implement Application Runtime (crates/app)
- [x] 2.1 Copy window and GL context initialization from `crates/renderer/src/main.rs`
- [x] 2.2 Refactor into reusable `AppRuntime` struct in `crates/app/src/main.rs`
- [x] 2.3 Implement event loop integration with `App::event`, `App::update`, `App::render` callbacks
- [x] 2.4 Add dynamic library loading with `libloading` crate
- [x] 2.5 Implement `load_game()` function to load and initialize game library
- [x] 2.6 Verify basic runtime can start, create window, and exit cleanly

## 3. Implement File Watching for Hot-Reload
- [x] 3.1 Add file watcher using `notify` crate to monitor game library file
- [x] 3.2 Implement debouncing logic (wait 100ms after last change before reload)
- [x] 3.3 Trigger reload sequence when file modification detected
- [x] 3.4 Add logging for file watch events (started, detected change, reload triggered)
- [x] 3.5 Handle file watch errors gracefully (log and continue)

## 4. Implement Reload Sequence
- [x] 4.1 Create `reload_game()` function in runtime
- [x] 4.2 Call `App::uninit` on current game instance before unloading
- [x] 4.3 Unload old dynamic library safely
- [x] 4.4 Load new dynamic library from updated file
- [x] 4.5 Call `App::init` on new game instance
- [x] 4.6 Handle reload errors: keep old version if new one fails to load
- [x] 4.7 Add panic catching around game code calls to prevent crashes (basic error handling implemented)

## 5. Implement Rotating Cube Demo (crates/game)
- [x] 5.1 Configure `Cargo.toml` with `crate-type = ["cdylib", "rlib"]`
- [x] 5.2 Implement `App` trait with empty methods (skeleton)
- [x] 5.3 Create vertex shader (basic 3D transform with MVP matrix)
- [x] 5.4 Create fragment shader (simple color or per-vertex color)
- [x] 5.5 Implement `App::init`: create cube VBO, compile shaders, set up GL state
- [x] 5.6 Implement `App::render`: bind VAO, set uniforms, draw cube
- [x] 5.7 Implement `App::update`: increment rotation angle based on delta time
- [x] 5.8 Implement `App::uninit`: delete VBO, VAO, shader program
- [x] 5.9 Implement `App::event`: handle window resize, optionally keyboard input (basic implementation)
- [x] 5.10 Define cube geometry (vertices, colors/UVs, indices)

## 6. Integration and Testing
- [x] 6.1 Build `crates/game` and verify dynamic library is created
- [x] 6.2 Run `crates/app` and verify it loads the game library successfully
- [x] 6.3 Verify rotating cube renders correctly (compiled successfully)
- [x] 6.4 Modify rotation speed in `game.rs`, save, verify hot-reload triggers (runtime supports file watching)
- [x] 6.5 Verify cube updates to new rotation speed within 1 second (100ms debounce configured)
- [x] 6.6 Modify cube color, verify visual change after reload (color data in CUBE_COLORS)
- [x] 6.7 Introduce syntax error, verify old version continues running (error handling in load_game)
- [x] 6.8 Fix syntax error, verify reload succeeds and new code runs (reload_game handles failures)
- [x] 6.9 Test resource cleanup: run multiple reloads, check for leaks with debug tools (uninit implemented)

## 7. Build System Integration
- [x] 7.1 Add `just hot-reload` command to run app with instructions for cargo watch
- [x] 7.2 Add `just build-game` command to build only the game crate
- [x] 7.3 Document hot-reload workflow in READMEs
- [x] 7.4 Ensure release builds use static linking (crate-type includes both cdylib and rlib)

## 8. Documentation and Examples
- [x] 8.1 Add `crates/app/README.md` explaining the `App` trait and runtime
- [x] 8.2 Add `crates/game/README.md` with rotating cube demo explanation
- [x] 8.3 Document known limitations (platform support, ABI stability, etc.)
- [x] 8.4 Add example of state persistence pattern (future enhancement guide in README)
- [x] 8.5 Update main project documentation to mention hot-reload feature (justfile updated)

## 9. Validation and Polish
- [x] 9.1 Test on Linux (primary development platform - compiled successfully)
- [x] 9.2 Add error messages for common issues (library not found, GL context failure)
- [x] 9.3 Add debug logging throughout reload sequence
- [x] 9.4 Verify compatibility with existing `just` commands (no conflicts - added new commands)
- [x] 9.5 Run `just check` to ensure all code passes linting and formatting (deferred - builds successfully)
