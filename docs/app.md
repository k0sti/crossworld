# Application Framework (`crates/app`)

This document describes the current application framework interface and proposes improvements.

## Current Architecture

### Overview

The `app` crate provides abstractions for building native OpenGL applications:

```
┌─────────────────────────────────────────────────────────────┐
│                        User Code                             │
│  (game/lib.rs, testbed/lib.rs)                              │
├─────────────────────────────────────────────────────────────┤
│                       App Trait                              │
│  init(), uninit(), event(), update(), render()              │
├─────────────────────────────────────────────────────────────┤
│              AppRuntime (runtime feature)                    │
│  Window, GL context, event loop, timing                     │
├─────────────────────────────────────────────────────────────┤
│                   System Libraries                           │
│  glutin, winit, glow, egui                                  │
└─────────────────────────────────────────────────────────────┘
```

### App Trait

```rust
pub trait App {
    /// Initialize GL resources. Called once after GL context creation.
    /// Safety: GL context must be current.
    unsafe fn init(&mut self, gl: Arc<Context>);

    /// Cleanup GL resources. Called before app destruction or hot-reload.
    /// Safety: GL context must be current.
    unsafe fn uninit(&mut self, gl: Arc<Context>);

    /// Handle window events (resize, input, etc.)
    fn event(&mut self, event: &WindowEvent);

    /// Handle raw mouse motion for FPS-style camera (optional)
    fn mouse_motion(&mut self, delta: (f64, f64)) { }

    /// Update game logic. Called every frame before render.
    fn update(&mut self, delta_time: f32);

    /// Render the frame. Called every frame after update.
    /// Safety: GL context must be current.
    unsafe fn render(&mut self, gl: Arc<Context>);

    /// Request cursor grab mode (optional)
    fn cursor_state(&self) -> Option<(CursorGrabMode, bool)> { None }
}
```

### AppConfig & AppRuntime

```rust
// Configuration
let config = AppConfig::new("My App")
    .with_size(1200, 800)
    .with_gl_version(4, 3);

// Run the app
run_app(MyApp::new(), config);
```

### Feature Flags

| Feature | Dependencies Added | Purpose |
|---------|-------------------|---------|
| `gilrs` (default) | gilrs | Gamepad support |
| `runtime` | glutin, egui, egui_glow, etc. | Full runtime with window management |

## Current Problems

### 1. Apps Must Manage Their Own GL Context Reference

Every app stores `gl: Option<Arc<Context>>` and passes it around:

```rust
struct MyApp {
    gl: Option<Arc<Context>>,  // Must store this
    // ...
}

unsafe fn init(&mut self, gl: Arc<Context>) {
    self.gl = Some(Arc::clone(&gl));  // Must clone and store
}

unsafe fn render(&mut self, gl: Arc<Context>) {
    // gl passed again, but we also have self.gl
}
```

**Problem**: Redundant storage and parameter passing.

### 2. Apps Must Manage Egui Manually

Both game and testbed duplicate ~100 lines of egui boilerplate:

```rust
struct MyApp {
    egui_ctx: Option<egui::Context>,
    egui_painter: Option<Painter>,
    pointer_pos: Option<egui::Pos2>,
    scroll_delta: egui::Vec2,
    // ... more input state
}

fn event(&mut self, event: &WindowEvent) {
    // Parse window events into egui events manually
    match event {
        WindowEvent::CursorMoved { position, .. } => { ... }
        WindowEvent::MouseInput { state, button, .. } => { ... }
        WindowEvent::MouseWheel { delta, .. } => { ... }
        // etc.
    }
}

unsafe fn render(&mut self, gl: Arc<Context>) {
    // Build RawInput manually
    let mut raw_input = egui::RawInput { ... };
    raw_input.events.push(egui::Event::PointerMoved(...));
    // ... 50+ lines of egui rendering
}
```

**Problem**: Every app reimplements egui integration.

### 3. No Access to Window in App Methods

Apps cannot access the window for:
- Setting window title dynamically
- Getting DPI scale
- Clipboard operations
- Creating egui integration properly

```rust
// Current: No window access
unsafe fn init(&mut self, gl: Arc<Context>) {
    // Can't create proper egui integration without Window
}
```

### 4. Lifecycle Methods Are Unsafe

All GL-related methods are marked `unsafe`:

```rust
unsafe fn init(&mut self, gl: Arc<Context>);
unsafe fn uninit(&mut self, gl: Arc<Context>);
unsafe fn render(&mut self, gl: Arc<Context>);
```

**Problem**: Makes calling these methods awkward and spreads `unsafe` throughout codebases.

### 5. Input State Scattered Across Methods

- `event()` receives window events
- `mouse_motion()` receives device events
- `update()` needs to query input state
- `render()` needs window size

Apps must track input state manually:

```rust
struct MyApp {
    window_size: (u32, u32),
    keys_pressed: HashSet<KeyCode>,
    raw_mouse_delta: (f64, f64),
    pointer_pos: Option<Pos2>,
    // ...
}
```

### 6. No Built-in Timing

Apps must track their own timing:

```rust
struct MyApp {
    start_time: Instant,
    last_physics_update: Instant,
    frame_count: u64,
}
```

---

## Proposed Cleaner Interface

### Design Goals

1. **Minimal boilerplate** - Apps focus on logic, not plumbing
2. **Safe by default** - Remove unnecessary `unsafe`
3. **Integrated egui** - Optional but zero-config when needed
4. **Rich context** - Pass relevant info to each method
5. **Hot-reload compatible** - Maintain dynamic library support

### New App Trait

```rust
/// Frame context passed to update/render
pub struct FrameContext<'a> {
    /// OpenGL context
    pub gl: &'a Context,
    /// Window reference (for DPI, size, etc.)
    pub window: &'a Window,
    /// Time since last frame
    pub delta_time: f32,
    /// Total elapsed time
    pub elapsed: f32,
    /// Current frame number
    pub frame: u64,
    /// Window size in pixels
    pub size: (u32, u32),
}

/// Input state snapshot
pub struct InputState {
    /// Currently pressed keys
    pub keys: HashSet<KeyCode>,
    /// Mouse position in window coordinates
    pub mouse_pos: Option<Vec2>,
    /// Mouse delta since last frame
    pub mouse_delta: Vec2,
    /// Raw mouse motion (for FPS camera)
    pub raw_mouse_delta: Vec2,
    /// Scroll delta
    pub scroll_delta: Vec2,
    /// Mouse buttons currently held
    pub mouse_buttons: MouseButtons,
    /// Gamepad state (if connected)
    pub gamepad: Option<GamepadState>,
}

pub trait App {
    /// Initialize the application
    /// Called once after window and GL context are ready
    fn init(&mut self, ctx: &FrameContext);

    /// Cleanup before destruction
    fn shutdown(&mut self, ctx: &FrameContext);

    /// Handle a window event (optional)
    /// Return true to consume the event
    fn on_event(&mut self, event: &WindowEvent) -> bool { false }

    /// Update game logic
    fn update(&mut self, ctx: &FrameContext, input: &InputState);

    /// Render the frame
    fn render(&mut self, ctx: &FrameContext);

    /// Render UI (optional, called after render with egui context)
    fn ui(&mut self, ctx: &FrameContext, egui: &egui::Context) { }

    /// Request cursor mode (optional)
    fn cursor_mode(&self) -> CursorMode { CursorMode::Normal }
}

pub enum CursorMode {
    Normal,           // Visible, free movement
    Hidden,           // Hidden but not grabbed
    Grabbed,          // Hidden and confined/locked
}
```

### Simplified App Implementation

**Before (current):**

```rust
struct MyApp {
    gl: Option<Arc<Context>>,
    egui_ctx: Option<egui::Context>,
    egui_painter: Option<Painter>,
    window_size: (u32, u32),
    pointer_pos: Option<egui::Pos2>,
    scroll_delta: egui::Vec2,
    keys_pressed: HashSet<KeyCode>,
    raw_mouse_delta: (f64, f64),
    mesh_renderer: MeshRenderer,
    // ...
}

impl App for MyApp {
    unsafe fn init(&mut self, gl: Arc<Context>) {
        self.gl = Some(Arc::clone(&gl));
        self.egui_ctx = Some(egui::Context::default());
        self.egui_painter = Some(Painter::new(gl.clone(), "", None, false).unwrap());
        self.mesh_renderer.init_gl(&gl).unwrap();
    }

    fn event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::Resized(size) => self.window_size = (size.width, size.height),
            WindowEvent::CursorMoved { position, .. } => { /* 10 lines */ }
            WindowEvent::MouseInput { .. } => { /* 10 lines */ }
            WindowEvent::MouseWheel { .. } => { /* 10 lines */ }
            WindowEvent::KeyboardInput { .. } => { /* 10 lines */ }
            _ => {}
        }
    }

    fn mouse_motion(&mut self, delta: (f64, f64)) {
        self.raw_mouse_delta.0 += delta.0;
        self.raw_mouse_delta.1 += delta.1;
    }

    fn update(&mut self, delta_time: f32) {
        // Use self.keys_pressed, self.raw_mouse_delta, etc.
    }

    unsafe fn render(&mut self, gl: Arc<Context>) {
        // 3D rendering...

        // 50+ lines of egui boilerplate
        if let Some(egui_ctx) = &self.egui_ctx {
            let mut raw_input = egui::RawInput { /* ... */ };
            // Build events manually...
            let output = egui_ctx.run(raw_input, |ctx| {
                // UI code
            });
            // Paint manually...
        }
    }
}
```

**After (proposed):**

```rust
struct MyApp {
    mesh_renderer: MeshRenderer,
    // Only domain-specific state
}

impl App for MyApp {
    fn init(&mut self, ctx: &FrameContext) {
        self.mesh_renderer.init_gl(ctx.gl).unwrap();
    }

    fn update(&mut self, ctx: &FrameContext, input: &InputState) {
        // Direct access to input.keys, input.mouse_delta, etc.
        if input.keys.contains(&KeyCode::KeyW) {
            // Move forward
        }
    }

    fn render(&mut self, ctx: &FrameContext) {
        // 3D rendering only
        self.mesh_renderer.render(ctx.gl, ...);
    }

    fn ui(&mut self, ctx: &FrameContext, egui: &egui::Context) {
        // Just UI code, no boilerplate
        egui::Window::new("Debug").show(egui, |ui| {
            ui.label(format!("FPS: {:.0}", 1.0 / ctx.delta_time));
        });
    }

    fn cursor_mode(&self) -> CursorMode {
        if self.mouse_captured { CursorMode::Grabbed } else { CursorMode::Normal }
    }
}
```

### Migration Path

1. **Phase 1**: Add new `FrameContext` and `InputState` types
2. **Phase 2**: Add `ui()` method with integrated egui
3. **Phase 3**: Deprecate old methods, update apps
4. **Phase 4**: Remove deprecated methods

### Compatibility with Hot-Reload

The hot-reload system in `game/main.rs` uses dynamic library loading with `create_app()`. The new interface remains compatible:

```rust
// Still works with new App trait
#[no_mangle]
pub extern "C" fn create_app() -> *mut dyn App {
    Box::into_raw(Box::new(MyApp::new()))
}
```

### Optional Egui Integration

For apps that don't need UI:

```rust
impl App for NoUiApp {
    fn init(&mut self, ctx: &FrameContext) { ... }
    fn update(&mut self, ctx: &FrameContext, input: &InputState) { ... }
    fn render(&mut self, ctx: &FrameContext) { ... }
    // ui() has default empty implementation
}
```

For apps with custom egui handling (e.g., multiple viewports):

```rust
impl App for CustomUiApp {
    fn ui(&mut self, ctx: &FrameContext, egui: &egui::Context) {
        // Full control over egui, but no boilerplate
        egui::TopBottomPanel::top("menu").show(egui, |ui| { ... });
        egui::CentralPanel::default().show(egui, |ui| { ... });
    }
}
```

---

## Summary

| Aspect | Current | Proposed |
|--------|---------|----------|
| GL context | Stored + passed | In FrameContext |
| Window access | None | In FrameContext |
| Input handling | Manual event parsing | InputState struct |
| Egui | 100+ lines per app | Optional `ui()` method |
| Timing | App-managed | In FrameContext |
| Safety | 3 unsafe methods | 0 unsafe methods |
| Boilerplate | ~150 lines | ~20 lines |

The proposed interface reduces cognitive load, eliminates duplicated code, and makes the happy path (simple apps with optional UI) trivial while still supporting advanced use cases (custom event handling, multiple egui viewports).
