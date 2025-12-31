# Game - Rotating Cube Demo

A simple rotating cube demo that showcases hot-reload capabilities with the `app` runtime.

## What It Does

Renders a colored 3D cube that rotates continuously. Each face has a different color:
- Front: Red
- Back: Green
- Top: Blue
- Bottom: Yellow
- Right: Magenta
- Left: Cyan

## Implementation

### App Trait

Implements all five lifecycle methods from the `App` trait:

```rust
impl App for RotatingCube {
    unsafe fn init(&mut self, gl: &Context) {
        // Create VAO, VBOs for vertices/colors, EBO for indices
        // Compile vertex/fragment shaders
        // Link shader program
        // Enable depth testing
    }

    unsafe fn uninit(&mut self, gl: &Context) {
        // Delete VAO, VBOs, EBO, shader program
    }

    fn event(&mut self, event: &WindowEvent) {
        // No event handling in this simple demo
    }

    fn update(&mut self, delta_time: f32) {
        // Rotate at 45 degrees per second
        self.rotation += 45.0_f32.to_radians() * delta_time;
    }

    unsafe fn render(&mut self, gl: &Context) {
        // Clear screen
        // Create MVP matrix (model, view, projection)
        // Set uniform, draw cube
    }
}
```

### Shaders

**Vertex Shader** (`VERTEX_SHADER`):
- Transforms vertices with MVP matrix
- Passes per-vertex color to fragment shader

**Fragment Shader** (`FRAGMENT_SHADER`):
- Simply outputs the interpolated vertex color

### Geometry

- 24 vertices (4 per face, 6 faces)
- 36 indices (2 triangles per face, 6 faces)
- Per-vertex color attributes

## Hot-Reload Testing

Try modifying these values and saving to see instant updates:

### Change Rotation Speed

In `update()`:
```rust
// Change this value:
self.rotation += 45.0_f32.to_radians() * delta_time;

// Try:
self.rotation += 90.0_f32.to_radians() * delta_time;  // Faster
self.rotation += 15.0_f32.to_radians() * delta_time;  // Slower
```

### Change Colors

In `CUBE_COLORS`:
```rust
// Front face (currently red)
1.0, 0.0, 0.0,  // Try: 1.0, 0.5, 0.0 (orange)
```

### Change Camera Position

In `render()`:
```rust
let view = Mat4::look_at_rh(
    Vec3::new(0.0, 0.0, 3.0),  // Try: Vec3::new(2.0, 2.0, 4.0)
    Vec3::new(0.0, 0.0, 0.0),
    Vec3::new(0.0, 1.0, 0.0),
);
```

## Compilation

Configured as both a dynamic and static library:

```toml
[lib]
crate-type = ["cdylib", "rlib"]
```

- `cdylib` - For hot-reload (dynamic library)
- `rlib` - For static linking in release builds

## Export Symbol

The `create_app` function is exported for the runtime to find:

```rust
#[no_mangle]
pub extern "C" fn create_app() -> *mut dyn App {
    Box::into_raw(Box::new(RotatingCube::new()))
}
```

## Dependencies

- `app` - Provides the `App` trait
- `glow` - OpenGL bindings
- `glam` - Math library (vectors, matrices)
- `bytemuck` - Safe casting for buffer data
- `winit` - Window event types

## Future Enhancements

- State persistence across reloads (preserve rotation angle)
- Shader hot-reload (detect shader file changes)
- Multiple cubes
- Interactive camera controls
- Texture mapping

## Resource Management

**Critical**: Always clean up GL resources in `uninit()`:
- Delete VAOs, VBOs, EBOs
- Delete shader programs
- Free any allocated memory

Failing to clean up causes resource leaks that accumulate with each reload.

## License

Same as parent project.
