# Change: Refactor Renderer Interface for Cross-Crate Usability

## Why

The renderer crate currently has renderer implementations scattered across multiple top-level modules (cpu_tracer.rs, gl_tracer.rs, bcf_cpu_tracer.rs, gpu_tracer.rs, mesh_renderer.rs). While there is a `Renderer` trait, it's not consistently implemented by all renderers (e.g., `MeshRenderer` doesn't implement it), and the interface is limited to basic `render()` methods without support for common GL operations like initialization and cleanup.

Other crates like `proto-gl` need to use these renderers but currently must work with inconsistent APIs. Some renderers require GL context initialization (`init_gl`), some have image buffers, some render directly to framebuffers. This makes it difficult to write generic rendering code.

## What Changes

- **Create unified Renderer interface** that supports:
  - Common rendering operations (`render`, `render_with_camera`)
  - GL-specific lifecycle (`init_gl`, `destroy_gl`, `render_to_framebuffer`)
  - Capability queries (`supports_gl`, `supports_image_output`, `name`)
  - File output for all renderers (`save_to_file`) - software renderers save from image buffer, GL renderers read back framebuffer

- **Reorganize module structure**:
  - Move all renderer implementations to `src/renderers/` module
  - Split `renderer.rs` into focused files: `renderer.rs` (trait), `camera.rs`, `lighting.rs`
  - Update exports in `lib.rs` for backward compatibility

- **Make MeshRenderer implement Renderer trait** with proper interface

- **Add renderer lifecycle management** for GL-based renderers

- **Update proto-gl** to use the unified interface

## Architecture Clarification

- **cube crate**: Octree data structure (Cube<T>, BCF serialization, mesh generation)
- **physics crate**: 3D entities (Entity trait, CubeObject with position/rotation)
- **renderer crate**: Rendering only (Renderer trait, CameraConfig, renderers, shaders)
- **proto-gl**: Application that combines all three crates

## Impact

- **Affected specs**: `renderer-interface` (new capability)
- **Affected code**:
  - `crates/renderer/src/lib.rs` - Module reorganization and re-exports
  - `crates/renderer/src/renderer.rs` - Split into renderer.rs, camera.rs, lighting.rs
  - `crates/renderer/src/renderers/` - New module with all renderer implementations
  - `crates/renderer/src/*_tracer.rs` + `mesh_renderer.rs` - Move to renderers module
  - `crates/proto-gl/src/app.rs` - Use unified interface

- **Breaking changes**: None (backward compatible re-exports)
- **Migration path**: Existing code continues to work via re-exports; new code should use `renderer::renderers::` module
