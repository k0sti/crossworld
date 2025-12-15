# Design: Unified Renderer Interface

## Context

The renderer crate currently contains 5 different renderer implementations with inconsistent interfaces:
- **CpuTracer**: Software raytracer, implements Renderer trait, outputs to image buffer
- **BcfTracer**: BCF-based CPU raytracer, implements Renderer trait, outputs to image buffer
- **GlTracer**: Fragment shader raytracer, implements Renderer trait, requires GL context
- **ComputeTracer**: Compute shader raytracer, implements Renderer trait, requires GL context
- **MeshRenderer**: Triangle mesh renderer, does NOT implement Renderer trait, requires GL context

The proto-gl crate and other applications need to use these renderers but face challenges:
1. Inconsistent initialization patterns (some have `init_gl()`, some don't)
2. Inconsistent rendering patterns (some render to framebuffers, some to image buffers)
3. MeshRenderer cannot be used polymorphically via Renderer trait
4. No capability discovery mechanism (which renderers support GL, which support image output)

## Goals / Non-Goals

**Goals:**
- Unified interface for all renderers via extended Renderer trait
- Clear separation between software and GL-based renderers
- Support for both direct framebuffer rendering and image buffer output
- Backward compatibility with existing code
- Better module organization for discoverability
- Usable from other crates without internal knowledge

**Non-Goals:**
- Changing renderer behavior or output quality
- Adding new renderer features beyond interface unification
- Modifying shader code or raytracing algorithms
- Performance optimizations (except incidental improvements)

## Decisions

### Decision: Extended Renderer Trait

Extend the existing Renderer trait with optional GL-specific methods:

```rust
pub trait Renderer {
    // Existing methods (software rendering)
    fn render(&mut self, width: u32, height: u32, time: f32);
    fn render_with_camera(&mut self, width: u32, height: u32, camera: &CameraConfig);
    fn name(&self) -> &str;

    // New: Capability queries
    fn supports_gl(&self) -> bool { false }
    fn supports_image_output(&self) -> bool { false }

    // New: GL lifecycle (default impls panic if not supported)
    fn init_gl(&mut self, gl: &Context) -> Result<(), String> {
        let _ = gl;
        Err(format!("{} does not support GL rendering", self.name()))
    }

    fn destroy_gl(&mut self, gl: &Context) {
        let _ = gl;
        // No-op for renderers without GL support
    }

    fn render_to_framebuffer(
        &mut self,
        gl: &Context,
        width: u32,
        height: u32,
        camera: Option<&CameraConfig>,
        time: Option<f32>,
    ) -> Result<(), String> {
        let _ = (gl, width, height, camera, time);
        Err(format!("{} does not support framebuffer rendering", self.name()))
    }

    // Optional: Image buffer access (for software renderers)
    fn image_buffer(&self) -> Option<&ImageBuffer<Rgb<u8>, Vec<u8>>> { None }

    // New: File output (different signatures for software vs GL renderers)
    // Software renderers: save from internal buffer
    fn save_to_file(&self, path: &str) -> Result<(), String> {
        if let Some(buffer) = self.image_buffer() {
            buffer.save(path).map_err(|e| e.to_string())
        } else {
            Err(format!("{} does not have an image buffer", self.name()))
        }
    }

    // GL renderers: read back from framebuffer
    fn save_framebuffer_to_file(&self, gl: &Context, width: u32, height: u32, path: &str) -> Result<(), String> {
        let _ = (gl, width, height, path);
        Err(format!("{} does not support framebuffer readback", self.name()))
    }
}
```

**Rationale:**
- Provides unified interface while allowing specialization
- Default implementations prevent breaking existing code
- Capability queries allow runtime feature detection
- Explicit GL context passing is safer than implicit state

**Alternatives considered:**
1. Separate `GlRenderer` trait - Rejected: Creates trait hierarchy complexity
2. Single required interface - Rejected: Forces all renderers to implement unused methods
3. Builder pattern for renderers - Rejected: Over-engineered for current needs

### Decision: File Output Support for All Renderers

Add file saving capability to all renderers via trait methods:

```rust
// Software renderers (CpuTracer, BcfTracer)
fn save_to_file(&self, path: &str) -> Result<(), String> {
    self.image_buffer()
        .ok_or("No image buffer")?
        .save(path)
        .map_err(|e| e.to_string())
}

// GL renderers (GlTracer, ComputeTracer, MeshRenderer)
fn save_framebuffer_to_file(&self, gl: &Context, width: u32, height: u32, path: &str) -> Result<(), String> {
    // Read pixels from currently bound framebuffer
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    unsafe {
        gl.read_pixels(
            0, 0,
            width as i32, height as i32,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelPackData::Slice(&mut pixels),
        );
    }

    // Convert from RGBA to RGB (drop alpha channel)
    let rgb_pixels: Vec<u8> = pixels.chunks(4)
        .flat_map(|rgba| [rgba[0], rgba[1], rgba[2]])
        .collect();

    // Flip Y-axis (GL origin is bottom-left, image origin is top-left)
    let mut flipped = vec![0u8; rgb_pixels.len()];
    for y in 0..height {
        let src_row = &rgb_pixels[(y * width * 3) as usize..((y + 1) * width * 3) as usize];
        let dst_y = height - 1 - y;
        let dst_row = &mut flipped[(dst_y * width * 3) as usize..((dst_y + 1) * width * 3) as usize];
        dst_row.copy_from_slice(src_row);
    }

    // Save to file
    image::save_buffer(path, &flipped, width, height, image::ColorType::Rgb8)
        .map_err(|e| e.to_string())
}
```

**Rationale:**
- Enables testing and debugging of all renderers via file output
- Software renderers use existing image buffers (zero overhead)
- GL renderers use framebuffer readback (necessary overhead for file save)
- Different method names clarify different requirements (GL context needed for framebuffer readback)
- Useful for screenshot functionality in applications like proto-gl

**Alternatives considered:**
1. Single `save()` method with Option<&Context> parameter - Rejected: Confusing API, unclear when context is needed
2. Helper function outside trait - Rejected: Not discoverable, inconsistent interface
3. Only support file output for software renderers - Rejected: Limits usefulness, GL renderers need screenshots too
4. Always maintain internal image buffer - Rejected: Memory overhead, performance cost of constant readback

### Decision: Module Structure

Reorganize renderer crate for clarity while keeping it simple:

```
crates/renderer/src/
├── lib.rs              # Public API and re-exports
├── renderer.rs         # Renderer trait (extended with GL methods)
├── camera.rs           # CameraConfig (split from renderer.rs)
├── lighting.rs         # Lighting constants (split from renderer.rs)
├── renderers/          # Renderer implementations module
│   ├── mod.rs
│   ├── cpu_tracer.rs
│   ├── bcf_cpu_tracer.rs
│   ├── gl_tracer.rs
│   ├── gpu_tracer.rs
│   └── mesh_renderer.rs
├── shader_utils.rs     # Shader utilities (unchanged)
├── bcf_raycast.rs      # BCF algorithm (unchanged)
├── scenes/             # Scene definitions (unchanged)
├── shaders/            # GLSL shaders (unchanged)
├── egui_app.rs         # GUI application (unchanged)
└── main.rs             # Binary entry point (unchanged)
```

**Rationale:**
- Groups all renderer implementations in `renderers/` module for clarity
- Splits large `renderer.rs` into focused modules (renderer trait, camera, lighting)
- Keeps rendering-only concerns in renderer crate (Entity/CubeObject are in physics crate)
- Flat structure at top level avoids over-engineering
- Preserves existing structure for shader_utils, scenes, shaders

**Alternatives considered:**
1. Deep `core/` module hierarchy - Rejected: Over-engineered, unclear benefits
2. Keep all files flat - Rejected: renderer.rs too large, hard to find renderer implementations
3. Separate crates for each renderer - Rejected: Over-complicated for internal use

### Decision: Backward Compatibility via Re-exports

Maintain all existing public exports at crate root:

```rust
// src/lib.rs
pub mod renderer;
pub mod camera;
pub mod lighting;
pub mod renderers;

// Backward-compatible re-exports
pub use renderer::Renderer;
pub use camera::CameraConfig;
pub use lighting::{AMBIENT, DIFFUSE_STRENGTH, LIGHT_DIR, BACKGROUND_COLOR};
pub use renderers::{CpuTracer, BcfTracer, GlTracer, ComputeTracer, MeshRenderer};
```

**Rationale:**
- Zero migration effort for existing code
- Gradual migration path available
- New code can use new structure (e.g. `renderer::renderers::GlTracer`), old code keeps working
- Standard Rust practice for API evolution
- Entity and CubeObject remain in physics crate (not renderer's concern)

### Decision: MeshRenderer Implements Renderer

Implement the full Renderer trait for MeshRenderer:

```rust
impl Renderer for MeshRenderer {
    fn render(&mut self, _width: u32, _height: u32, _time: f32) {
        panic!("MeshRenderer requires explicit camera and GL context. Use render_to_framebuffer()");
    }

    fn render_with_camera(&mut self, _width: u32, _height: u32, _camera: &CameraConfig) {
        panic!("MeshRenderer requires GL context. Use render_to_framebuffer()");
    }

    fn name(&self) -> &str { "Mesh Renderer" }
    fn supports_gl(&self) -> bool { true }
    fn supports_image_output(&self) -> bool { false }

    fn init_gl(&mut self, gl: &Context) -> Result<(), String> {
        // Existing initialization logic
    }

    fn render_to_framebuffer(&mut self, gl: &Context, ...) -> Result<(), String> {
        // Wrapper around existing render_mesh_with_depth
    }
}
```

**Rationale:**
- Makes MeshRenderer usable polymorphically
- Panicking methods clearly document wrong usage
- Capability queries guide correct usage
- Preserves existing specialized API for direct use

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Breaking changes despite re-exports | Comprehensive testing of existing code paths |
| Trait method explosion | Keep trait focused on essential rendering operations |
| Confusion about which interface to use | Clear documentation and examples |
| Performance overhead from trait methods | Minimal - trait dispatch is negligible vs rendering cost |
| Proto-gl migration complexity | Gradual migration, both old and new APIs available |

## Migration Plan

### Phase 1: Internal Reorganization (Non-Breaking)
1. Create `core/` and `renderers/` modules
2. Move files to new locations
3. Update internal imports
4. Add re-exports to `lib.rs`
5. Run all tests to verify no breakage

### Phase 2: Trait Extension (Non-Breaking)
1. Add new methods to Renderer trait with default implementations
2. Implement GL lifecycle methods for GL-based renderers
3. Implement Renderer trait for MeshRenderer
4. Add capability query implementations
5. Run all tests

### Phase 3: Proto-gl Migration (Breaking for proto-gl only)
1. Update proto-gl to use new module paths (optional)
2. Update proto-gl to use unified GL lifecycle methods
3. Update proto-gl to use `render_to_framebuffer()`
4. Test proto-gl application thoroughly

### Rollback Strategy
If issues arise:
1. Git revert to before Phase 1
2. All changes are in renderer crate, no external dependencies
3. Re-exports ensure backward compatibility, easy to keep old structure

## Open Questions

1. **Should we add a `RenderOutput` enum for different output types?**
   - Would allow polymorphic handling of image buffer vs framebuffer rendering
   - Could be over-engineering for current needs
   - **Decision**: Defer until proven necessary

2. **Should capability queries be const fn?**
   - Would allow compile-time checks in some cases
   - Current Rust const fn limitations may prevent this
   - **Decision**: Start with runtime, migrate to const if valuable

3. **Should we version the Renderer trait or create Renderer v2?**
   - Backward compatibility is already handled via default methods
   - Creating v2 would fragment the ecosystem
   - **Decision**: Extend existing trait with default implementations
