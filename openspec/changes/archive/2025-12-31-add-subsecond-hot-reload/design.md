# Design: Subsecond Hot-Reload Architecture

## Context
The Crossworld project currently uses a monolithic architecture where all game code is compiled into a single binary. Any code change requires a full rebuild and restart, losing all runtime state. Dioxus's subsecond package demonstrates a hot-reload approach using dynamic libraries that can be reloaded at runtime.

### Constraints
- Must work with existing OpenGL/windowing infrastructure (glutin/winit)
- Should preserve application state across reloads (OpenGL context, window, etc.)
- Must have clear separation between stable runtime and hot-reloadable game code
- Target platform: Native desktop (Linux/macOS/Windows), not WASM initially

### Stakeholders
- Game developers (primary users, benefit from fast iteration)
- Graphics programmers (need stable GL context across reloads)
- Build system maintainers (need clear compilation targets)

## Goals / Non-Goals

### Goals
- Enable subsecond hot-reload for game logic changes
- Preserve OpenGL context and window state across reloads
- Clear, simple API for game developers (`App` trait with lifecycle hooks)
- Initial proof-of-concept with rotating cube demo
- Foundation for future hot-reload of assets, shaders, and voxel data

### Non-Goals
- Hot-reload for the runtime itself (only game code reloads)
- WASM support in initial implementation (future work)
- Automatic state serialization/migration (game code responsible for state management)
- Production deployment (development tool only)
- Cross-platform dynamic library compatibility guarantees (focus on development environment)

## Decisions

### Decision 1: App Trait with Lifecycle Hooks
**Choice**: Define an `App` trait with explicit lifecycle methods: `init`, `uninit`, `event`, `update`, `render`.

**Rationale**:
- Clear contract between runtime and game code
- Familiar pattern from game engines (Unity Update/FixedUpdate, Godot _process/_physics_process)
- Explicit `uninit` allows cleanup before reload (deallocate GPU resources, close files, etc.)
- Separate `update` (logic) and `render` (drawing) supports fixed timestep patterns

**Alternatives Considered**:
1. Single `run()` method with event polling - Less flexible, harder to reload mid-frame
2. Builder pattern with closures - More complex ownership, harder to reload
3. Component-based ECS - Over-engineered for initial implementation, can be added later

### Decision 2: Copy Application Setup from crates/renderer
**Choice**: Use `crates/renderer/src/main.rs` as the template for `crates/app`, extracting windowing and GL context initialization.

**Rationale**:
- Proven working code (glutin + winit + glow)
- Supports OpenGL 4.3 (compute shaders)
- Handles platform differences (X11 on Linux)
- Already has event loop and window management

**Migration**:
- Extract reusable parts into `crates/app/src/lib.rs`
- Keep `crates/app/src/main.rs` as minimal launcher
- `crates/renderer` remains unchanged (not refactored yet)

### Decision 3: Dynamic Library (.so/.dylib/.dll) for Hot-Reload
**Choice**: Compile `crates/game` as both a static library (for normal builds) and dynamic library (for hot-reload).

**Rationale**:
- Dynamic libraries can be reloaded at runtime using `dlopen`/`LoadLibrary`
- Rust supports `crate-type = ["cdylib", "rlib"]` in Cargo.toml
- File watcher can detect changes to the compiled `.so` file and trigger reload

**Alternatives Considered**:
1. WASM-based reload - Complex, requires JS bridge, not suitable for native OpenGL
2. Interpret scripts (Lua/Rhai) - Limited access to Rust ecosystem, performance overhead
3. Process-based reload (spawn new process) - Loses all state, slow IPC

### Decision 4: File Watching with notify Crate
**Choice**: Use the `notify` crate to watch the compiled game library and trigger reloads.

**Rationale**:
- De facto standard for file watching in Rust
- Cross-platform support
- Handles rapid successive writes (debouncing)

**Flow**:
1. Developer modifies code in `crates/game`
2. Cargo rebuilds `libgame.so` in watch mode (`cargo watch`)
3. File watcher detects `.so` change
4. Runtime calls `app.uninit()` on old version
5. Runtime unloads old library, loads new library
6. Runtime calls `app.init()` on new version

### Decision 5: Initial Demo - Rotating Cube
**Choice**: Implement a simple rotating cube as the first hot-reloadable game.

**Rationale**:
- Minimal complexity (single VBO, simple shader)
- Demonstrates `update()` (rotation logic) and `render()` (GL calls)
- Easy to verify hot-reload (change rotation speed, colors, etc.)
- Can reuse shader code from `crates/renderer`

**Verification**:
- Change rotation speed constant in `game.rs`, see immediate update
- Change cube color, see immediate visual change
- Introduce compile error, verify graceful handling (old version continues running)

## Risks / Trade-offs

### Risk 1: Dynamic Library ABI Stability
**Risk**: Changes to `App` trait signature break reload compatibility.

**Mitigation**:
- Use `#[repr(C)]` for FFI types
- Version the trait (e.g., `App_v1`, `App_v2`) if breaking changes needed
- Document ABI stability requirements in `crates/app/README.md`

### Risk 2: Resource Leaks Across Reloads
**Risk**: Game code forgets to clean up GL resources in `uninit()`, causing leaks.

**Mitigation**:
- Provide clear examples in documentation
- Consider RAII wrappers that track GL objects (future work)
- Add debug mode that validates all resources released

### Risk 3: Platform-Specific Dynamic Library Behavior
**Risk**: `.so` reloading may behave differently on Linux vs macOS vs Windows.

**Mitigation**:
- Focus on Linux development environment initially
- Document known platform differences
- Use abstraction layer (`libloading` crate) to hide platform details

### Risk 4: Compile Time Still Bottleneck
**Risk**: If `cargo build` takes 10+ seconds, hot-reload is less useful.

**Mitigation**:
- Keep `crates/game` minimal and focused
- Use `--timings` to identify slow dependencies
- Consider incremental compilation optimizations
- Accept that initial implementation won't be subsecond if dependencies are heavy

## Migration Plan

### Phase 1: Scaffold New Crates (Non-Breaking)
1. Create `crates/app/` directory structure
2. Copy relevant code from `crates/renderer/src/main.rs`
3. Define `App` trait in `crates/app/src/lib.rs`
4. Create `crates/game/` with stub implementation
5. Add workspace members to root `Cargo.toml`
6. Verify builds successfully (no hot-reload yet)

### Phase 2: Implement Hot-Reload Runtime
1. Add dynamic library loading to `crates/app/src/main.rs`
2. Implement file watcher for `libgame.so`
3. Add reload trigger on file change
4. Handle errors gracefully (keep old version on compile failure)

### Phase 3: Implement Rotating Cube Demo
1. Add vertex/fragment shaders to `crates/game`
2. Implement `App::init()` - create VBO, compile shaders
3. Implement `App::render()` - draw rotating cube
4. Implement `App::update()` - update rotation angle
5. Implement `App::uninit()` - cleanup GL resources

### Phase 4: Verification
1. Run application, verify cube renders
2. Modify rotation speed in source, save file
3. Verify cube updates within subseconds
4. Introduce compile error, verify old version continues
5. Fix error, verify recovery

### Rollback Plan
If hot-reload proves unstable:
- Keep `crates/app` as a standard runtime (no reload logic)
- Use `crates/game` as a normal static library
- Delete file watching and dynamic loading code
- Result: Clean separation of concerns, but no hot-reload

## Open Questions

### Q1: Should the App trait be object-safe (use `dyn App`)?
**Options**:
- A: Yes, use trait objects for dynamic dispatch
- B: No, use function pointers exported from dylib

**Recommendation**: Start with option B (function pointers) as it's simpler for FFI across dynamic library boundary. Can revisit if trait objects are needed.

### Q2: How to handle state persistence across reloads?
**Options**:
- A: Game code manages its own state serialization
- B: Runtime provides state storage API
- C: Use static variables (unsafe but simple)

**Recommendation**: Start with option A (manual state management). Game can use static mut or global allocator for state that survives reload.

### Q3: Integration with existing justfile commands?
**Proposed**:
- `just dev-hot` - Run app with hot-reload enabled + cargo watch for game crate
- `just game` - Build game crate only (faster iteration)

### Q4: Should crates/renderer be refactored to use crates/app?
**Recommendation**: No, keep as separate implementation initially. Future refactor can consolidate if hot-reload proves valuable.
