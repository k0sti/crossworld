# Design: Improve Renderer Quality

## Overview

This document outlines the technical approach for fixing mesh renderer issues, implementing mesh caching, and improving code organization in the renderer crate.

## Camera Synchronization

### Problem

The mesh renderer uses different camera calculations than the raytracing renderers, resulting in mismatched viewpoints that prevent visual comparison.

### Root Cause Analysis

The mesh renderer likely has one or more of these issues:

1. **View matrix calculation**: Mesh renderer may not be using `CameraConfig::view_matrix()` correctly
2. **Projection matrix**: Aspect ratio or field of view may differ
3. **Coordinate system**: OpenGL mesh rendering uses different conventions than raytracing
4. **Entity transform**: Additional transforms may be applied to mesh entities

### Solution

**Unified camera approach:**

```rust
impl MeshRenderer {
    pub unsafe fn render_to_gl_with_camera(
        &mut self,
        gl: &Context,
        width: i32,
        height: i32,
        camera: &CameraConfig,
        entities: &[Entity],
    ) {
        // Use SAME camera matrices as other renderers
        let view = camera.view_matrix();
        let aspect = width as f32 / height as f32;
        let proj = camera.projection_matrix(aspect);

        // Pass to shader as MVP for each entity
        for entity in entities {
            let model = entity.transform;
            let mvp = proj * view * model;
            // ... set uniforms and draw
        }
    }
}
```

**Key principle**: All renderers must use identical `CameraConfig` methods for view and projection matrices.

## Face Culling

### Problem

Mesh faces may render incorrectly due to front/back face culling configuration or incorrect winding order.

### Investigation Steps

1. Check current culling state:
   ```rust
   // In MeshRenderer::init_gl or render call
   gl.enable(GL_CULL_FACE);  // or disable?
   gl.cull_face(GL_BACK);    // or GL_FRONT?
   gl.front_face(GL_CCW);    // or GL_CW?
   ```

2. Verify triangle winding from `generate_face_mesh`:
   - OpenGL convention: Counter-clockwise (CCW) = front face
   - Check if generated indices match this convention

3. Test with culling disabled:
   ```rust
   gl.disable(GL_CULL_FACE);  // Should render all faces
   ```

### Solutions

**Option A: Fix winding order** (if indices are backwards)
```rust
// In upload_mesh, reverse index order
for i in (0..builder.indices.len()).step_by(3) {
    indices.push(builder.indices[i + 2]);
    indices.push(builder.indices[i + 1]);
    indices.push(builder.indices[i]);
}
```

**Option B: Fix OpenGL state** (if culling is incorrect)
```rust
gl.enable(GL_CULL_FACE);
gl.cull_face(GL_BACK);
gl.front_face(GL_CCW);  // Standard OpenGL convention
```

**Option C: Fix normals** (if normals point inward)
```rust
// Negate normals during upload
for i in 0..builder.normals.len() {
    normals[i] = -builder.normals[i];
}
```

**Decision criteria**: Test each option systematically, check against known-good output from raytracer.

## Mesh Caching

### Requirements

1. **Cache validation**: Mesh should regenerate only when voxel data changes
2. **User control**: UI toggle to enable/disable caching
3. **Manual regeneration**: Button to force mesh upload
4. **Visual feedback**: Indicate cache status to user

### Architecture

**State management:**

```rust
pub struct DualRendererApp {  // Will become CubeRendererApp
    // Existing fields...

    // New fields for mesh caching
    mesh_cache_enabled: bool,        // User setting
    mesh_needs_regeneration: bool,   // Invalidation flag
    mesh_vertex_count: usize,        // Stats for UI
    mesh_face_count: usize,
    last_mesh_upload_ms: f32,        // Timing
}
```

**Cache invalidation logic:**

```rust
impl CubeRendererApp {
    fn invalidate_mesh_cache(&mut self) {
        self.mesh_needs_regeneration = true;
    }

    fn on_model_change(&mut self) {
        self.invalidate_mesh_cache();
        // ... existing model change logic
    }

    fn on_material_change(&mut self) {
        if self.current_model == TestModel::SingleRedVoxel {
            self.invalidate_mesh_cache();
        }
        // ... existing material change logic
    }
}
```

**Rendering logic:**

```rust
fn render_mesh_frame(&mut self, gl: &Context) {
    let should_upload = !self.mesh_cache_enabled || self.mesh_needs_regeneration;

    if should_upload {
        let start = Instant::now();

        // Clear old meshes
        self.mesh_renderer.clear_meshes(gl);

        // Upload new mesh
        let idx = unsafe {
            self.mesh_renderer.upload_mesh(gl, &self.current_cube, depth)?
        };
        self.mesh_indices = vec![idx];

        self.last_mesh_upload_ms = start.elapsed().as_secs_f32() * 1000.0;
        self.mesh_needs_regeneration = false;

        // Update stats
        // (extract from mesh_renderer if exposed)
    }

    // Render using cached mesh
    unsafe {
        self.mesh_renderer.render_to_gl_with_camera(
            gl, width, height, &self.camera, &entities
        );
    }
}
```

**UI additions:**

```rust
ui.horizontal(|ui| {
    ui.checkbox(&mut self.mesh_cache_enabled, "Cache Mesh");

    if ui.button("Regenerate").clicked() {
        self.invalidate_mesh_cache();
    }

    let status = if self.mesh_needs_regeneration {
        "⚠ Needs Regen"
    } else if self.mesh_cache_enabled {
        "✓ Cached"
    } else {
        "○ Uncached"
    };
    ui.label(status);
});

ui.label(format!("Vertices: {}, Faces: {}",
    self.mesh_vertex_count, self.mesh_face_count));
if self.last_mesh_upload_ms > 0.0 {
    ui.label(format!("Last upload: {:.2}ms", self.last_mesh_upload_ms));
}
```

### Performance Impact

**Expected improvements:**

- **Static scenes**: 0% CPU/GPU cost for mesh generation when cached
- **Frame rate**: 5-30% improvement (depends on cube complexity)
- **Upload bandwidth**: Zero GPU transfers when cache valid

**Measurement approach:**

1. Measure frame time with caching disabled (baseline)
2. Enable caching, render 100 frames
3. Calculate average frame time improvement
4. Log mesh uploads to verify caching works

## Renaming Strategy

### Scope

**Rename targets:**

- `DualRendererApp` → `CubeRendererApp` (primary struct)
- `run_dual_renderer*` → `run_cube_renderer*` (functions)
- `dual_renderer` → `cube_renderer` (variables)
- Comments and docs mentioning "dual"

**Files affected:**

- `src/egui_app.rs` (struct definition, impl blocks)
- `src/main.rs` (function names, variables, help text)
- `src/lib.rs` (re-exports if any)

### Implementation

**Step-by-step:**

1. **Struct rename** (egui_app.rs):
   ```bash
   sed -i 's/DualRendererApp/CubeRendererApp/g' src/egui_app.rs
   ```

2. **Function renames** (main.rs):
   ```bash
   sed -i 's/run_dual_renderer/run_cube_renderer/g' src/main.rs
   sed -i 's/dual_renderer/cube_renderer/g' src/main.rs
   ```

3. **Manual review**:
   - Check that "Dual" → "Cube" makes sense in context
   - Update comments explaining rationale
   - Verify no missed occurrences: `rg -i "dual.*renderer"`

4. **Test compilation**:
   ```bash
   cargo check --package renderer
   cargo clippy --package renderer
   ```

### Backward Compatibility

**Not applicable**: This is an internal crate, no public API. Renaming is safe.

## Documentation Updates

### README.md Changes

**Before:**
```markdown
# Renderer

Dual-implementation cube raytracer with both GPU (WebGL2/OpenGL)
and CPU (pure Rust) backends.
```

**After:**
```markdown
# Renderer

Multi-implementation cube raytracer with five rendering backends
for comparison and validation:

- **CPU Raytracer**: Pure Rust software rendering
- **GL Raytracer**: WebGL 2.0 fragment shader raytracing
- **BCF CPU Raytracer**: CPU raytracer using Binary Cube Format
- **GPU Compute Raytracer**: OpenGL compute shader raytracing
- **Mesh Rasterizer**: Traditional triangle mesh rendering
```

**New sections to add:**

1. **Mesh Renderer** subsection:
   - How mesh is generated from octree
   - Mesh caching feature
   - Limitations vs raytracing

2. **Performance Tips**:
   - Enable mesh caching for static scenes
   - Use sync mode for accurate timing comparison

3. **Comparison Features**:
   - Visual diff tool
   - Performance metrics
   - Camera synchronization

### Code Documentation

**Module docstrings:**

```rust
//! # Cube Renderer Application
//!
//! egui-based GUI application for comparing five different cube rendering
//! implementations side-by-side. Supports synchronized rendering, visual
//! diff comparison, and performance profiling.
//!
//! ## Renderers
//!
//! 1. **CPU Tracer**: Pure Rust raytracer (slow but accurate)
//! 2. **GL Tracer**: Fragment shader raytracer (fast, GPU-based)
//! 3. **BCF CPU Tracer**: CPU raytracer with Binary Cube Format
//! 4. **GPU Tracer**: Compute shader raytracer (experimental)
//! 5. **Mesh Renderer**: Triangle rasterizer (traditional approach)
//!
//! ## Features
//!
//! - Synchronized camera across all renderers
//! - Real-time performance metrics
//! - Visual diff comparison between any two renderers
//! - Mesh caching for improved performance
//! - Model selection and customization
```

## Testing Strategy

### Camera Sync Test

```rust
#[test]
fn test_camera_synchronization() {
    // Create camera config
    let camera = CameraConfig::look_at(
        Vec3::new(3.0, 2.0, 3.0),
        Vec3::ZERO,
        Vec3::Y
    );

    let aspect = 800.0 / 600.0;

    // All renderers must produce same matrices
    let view = camera.view_matrix();
    let proj = camera.projection_matrix(aspect);

    // Verify matrices are consistent
    assert!(view.is_finite());
    assert!(proj.is_finite());

    // Additional checks:
    // - View matrix determinant != 0
    // - Projection matrix has expected perspective properties
}
```

### Mesh Cache Test

```rust
#[test]
fn test_mesh_cache_invalidation() {
    let mut app = CubeRendererApp::new_for_test();

    // Initially needs regeneration
    assert!(app.mesh_needs_regeneration);

    // After upload
    app.render_mesh_frame();
    assert!(!app.mesh_needs_regeneration);

    // Model change invalidates
    app.on_model_change();
    assert!(app.mesh_needs_regeneration);
}
```

## Migration Path

**No migration needed**: Changes are internal to renderer crate.

**Deployment:**

1. Merge all changes in single PR
2. Run full test suite: `cargo test --package renderer`
3. Manual smoke test with GUI
4. Update project documentation references if any

**Rollback plan:**

If issues arise, revert PR. No persistent state affected.
