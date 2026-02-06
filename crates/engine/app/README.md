# app

Generic application platform interface for building native OpenGL applications with hot-reload support.

## Overview

The `app` crate provides a runtime environment that:
- Creates and manages an OpenGL window and context
- Loads game code from a dynamic library (`.so`, `.dylib`, or `.dll`)
- Watches for file changes and triggers hot-reload automatically
- Preserves window and GL context state across reloads

## Architecture

```
┌─────────────────────────────────────────┐
│          AppRuntime (app crate)         │
├─────────────────────────────────────────┤
│  • Window (winit)                       │
│  • OpenGL Context (glutin + glow)       │
│  • File Watcher (notify)                │
│  • Dynamic Library Loader (libloading)  │
└──────────────┬──────────────────────────┘
               │ loads
               ▼
┌─────────────────────────────────────────┐
│       Game Code (game crate)            │
├─────────────────────────────────────────┤
│  Implements App trait:                  │
│  • init()    - Create GL resources      │
│  • uninit()  - Clean up GL resources    │
│  • event()   - Handle window events     │
│  • update()  - Game logic               │
│  • render()  - Draw frame               │
└─────────────────────────────────────────┘
```

## App Trait

Game code must implement the `App` trait:

```rust
pub trait App {
    unsafe fn init(&mut self, gl: &Context);
    unsafe fn uninit(&mut self, gl: &Context);
    fn event(&mut self, event: &WindowEvent);
    fn update(&mut self, delta_time: f32);
    unsafe fn render(&mut self, gl: &Context);
}
```

### Lifecycle

1. **Initial Load**: `init()` called when library is first loaded
2. **Event Loop**: `event()`, `update()`, `render()` called each frame
3. **Hot-Reload**: When source file changes:
   - `uninit()` called on old version
   - Old library unloaded
   - New library loaded
   - `init()` called on new version
4. **Shutdown**: `uninit()` called before exit

## Usage

### Running the Demo

```bash
# Build the game library
cargo build --package game

# Run the app (watches for changes)
cargo run --bin app

# Or use the justfile command
just hot-reload
```

### Development Workflow

**Terminal 1** - Run the app:
```bash
cargo run --bin app
```

**Terminal 2** - Auto-rebuild on changes:
```bash
cargo watch -x 'build --package game' -w crates/game
```

Now edit `crates/game/src/lib.rs` and save - the app will hot-reload within 100ms!

## How It Works

### File Watching

The runtime uses `notify-debouncer-mini` to watch the compiled game library file:
- Watches: `target/debug/libgame.so` (or `.dylib` / `.dll`)
- Debounce: 100ms (waits for file writes to complete)
- Triggers: Reload sequence when file changes

### Dynamic Loading

Uses `libloading` to load the game library and find the `create_app` symbol:

```rust
#[no_mangle]
pub extern "C" fn create_app() -> *mut dyn App {
    Box::into_raw(Box::new(YourGameStruct::new()))
}
```

### State Preservation

- **Preserved**: Window, GL context, file watcher
- **Reset**: Game state (unless explicitly persisted by game code)
- **Reinitialized**: All GL resources (buffers, shaders, textures)

## Configuration

The runtime expects the game library at:
- Linux: `target/debug/libgame.so`
- macOS: `target/debug/libgame.dylib`
- Windows: `target/debug/game.dll`

## Limitations

- **Dev only**: Hot-reload is for development, not production
- **Platform**: Tested on Linux, should work on macOS/Windows
- **ABI stability**: Changing the `App` trait signature breaks reload
- **State**: Game must manage its own state persistence across reloads
- **Resources**: Game must properly clean up in `uninit()` to avoid leaks

## Example

See `crates/game` for a complete rotating cube example that demonstrates:
- Shader compilation
- VBO/VAO management
- MVP matrix updates
- Resource cleanup
- Hot-reload verification

## Dependencies

- `winit` - Cross-platform windowing
- `glutin` / `glow` - OpenGL context and bindings
- `libloading` - Dynamic library loading
- `notify` / `notify-debouncer-mini` - File watching

## License

Same as parent project.
