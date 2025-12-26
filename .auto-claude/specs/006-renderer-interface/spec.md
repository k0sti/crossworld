# Specification: Refactor Renderer Interface for Cross-Crate Usability

## Overview

Refactor the renderer crate to provide a unified interface for cross-crate usability. This includes creating a consistent Renderer trait, reorganizing module structure, and adding GL lifecycle management for all renderer types.

## Source

Migrated from: `openspec/changes/refactor-renderer-interface/`

## Current Status

**Completion: 82% (42/51 tasks complete)**

### Completed Work
- Phase 1: Core Interface Design (4/4)
- Phase 2: Module Reorganization (8/8)
- Phase 3: Implement Extended Renderer Trait (5/5)
- Phase 4: GL Lifecycle Implementation (6/6)
- Phase 5: File Output Implementation (7/8)
- Phase 6: Library Exports and Backward Compatibility (4/4)
- Phase 7: Update proto-gl Integration (4/5 - 1 N/A)

### Pending Work
- Add tests for file output from all renderer types
- Documentation and Examples (Phase 8 - all 6 tasks pending)
- Integration test using Renderer trait polymorphically
- Test file output from all renderer types

## Problem Statement

**Current situation:**
- Renderer implementations scattered across multiple top-level modules (cpu_tracer.rs, gl_tracer.rs, bcf_cpu_tracer.rs, gpu_tracer.rs, mesh_renderer.rs)
- `Renderer` trait exists but not consistently implemented by all renderers (e.g., `MeshRenderer` doesn't implement it)
- Interface is limited to basic `render()` methods without support for common GL operations like initialization and cleanup
- Other crates like `proto-gl` must work with inconsistent APIs

**Impact:** Difficult to write generic rendering code when some renderers require GL context initialization, some have image buffers, and some render directly to framebuffers.

**Scope:** Unified renderer interface with GL lifecycle support and module reorganization.

## Solution

### Unified Renderer Interface

Create unified Renderer interface that supports:
- Common rendering operations (`render`, `render_with_camera`)
- GL-specific lifecycle (`init_gl`, `destroy_gl`, `render_to_framebuffer`)
- Capability queries (`supports_gl`, `supports_image_output`, `name`)
- File output for all renderers (`save_to_file`) - software renderers save from image buffer, GL renderers read back framebuffer

### Module Reorganization

- Move all renderer implementations to `src/renderers/` module
- Split `renderer.rs` into focused files: `renderer.rs` (trait), `camera.rs`, `lighting.rs`
- Update exports in `lib.rs` for backward compatibility

### Additional Changes

- Make MeshRenderer implement Renderer trait with proper interface
- Add renderer lifecycle management for GL-based renderers
- Update proto-gl to use the unified interface

## Architecture Clarification

- **cube crate**: Octree data structure (Cube<T>, BCF serialization, mesh generation)
- **physics crate**: 3D entities (Entity trait, CubeObject with position/rotation)
- **renderer crate**: Rendering only (Renderer trait, CameraConfig, renderers, shaders)
- **proto-gl**: Application that combines all three crates

## Affected Files

### Modified
- `crates/renderer/src/lib.rs` - Module reorganization and re-exports
- `crates/renderer/src/renderer.rs` - Split into renderer.rs, camera.rs, lighting.rs
- `crates/renderer/src/*_tracer.rs` + `mesh_renderer.rs` - Move to renderers module
- `crates/proto-gl/src/app.rs` - Use unified interface

### Created
- `crates/renderer/src/renderers/` - New module with all renderer implementations
- `crates/renderer/src/camera.rs` - Camera configuration
- `crates/renderer/src/lighting.rs` - Lighting constants and helpers

## Breaking Changes

- None (backward compatible re-exports)

## Migration Path

Existing code continues to work via re-exports; new code should use `renderer::renderers::` module.

## Success Criteria

1. All renderers implement unified Renderer trait
2. GL lifecycle (init, render, destroy) works consistently
3. File output works for all renderer types
4. Backward compatibility maintained via re-exports
5. proto-gl uses unified interface
6. Code passes clippy with no warnings

## Development Environment

```bash
# Check renderer builds
cargo check -p crossworld-renderer

# Run renderer tests
cargo test -p crossworld-renderer

# Run proto-gl
cargo run -p proto-gl
```
