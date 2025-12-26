# Specification: Implement GPU Raytracer with Cube Raytracing

## Overview

Implement GPU-based raytracer with shader-based octree traversal for high-performance voxel rendering. The renderer currently only supports CPU-based raytracing through `cpu_tracer.rs`, which integrates the cube crate's raycast implementation. To enable high-performance voxel rendering in the browser, we need a GPU-based raytracer that can leverage WebGL/WebGPU compute capabilities.

## Source

Migrated from: `openspec/changes/implement-gpu-raytracer/`

## Current Status

**Completion: 84% (36/43 tasks complete)**

### Completed Work
- Phase 1: WebGL 2.0 Fragment Shader Implementation (9/9)
- Phase 2: WebGL 2.0 Tracer Integration (7/7)
- Phase 3: Architecture Refactoring - 3-Tracer System (8/8)
- Phase 4: Testing and Validation (5/8) - basic validation done
- Phase 5: Code Quality (5/5)
- Phase 6: GPU Tracer Phase 1 - Basic Ray-Cube Intersection (12/14)
- Phase 7: CPU-Side Raycast Testing (6/6)

### Pending Work
- Performance benchmarks beyond basic timing
- Test various octree depths (0, 1, 2, 3+)
- Test edge cases (empty octrees, single voxels, etc.)
- GPU tracer integration into egui app
- Verify ray-cube hit detection in GPU tracer

## Problem Statement

**Current situation:**
- `cpu_tracer.rs` - Uses `cube::Cube::raycast()` for CPU-based raytracing
- `gpu_tracer.rs` - Contains only stub implementation
- No GPU shader implementation for octree traversal

**Impact:** Without GPU raytracing, rendering performance is limited by JavaScript/WASM CPU execution speed, making real-time voxel rendering at higher resolutions impractical.

**Scope:** Implement GPU raytracer with shader-based octree traversal, matching the functionality of the CPU raytracer but optimized for parallel execution on the GPU.

## Solution: 3-Tracer Architecture

### CPU Tracer
- Uses `cube::Cube::raycast()` for CPU-based raytracing
- Reference implementation for correctness validation
- Located in `crates/renderer/src/cpu_tracer.rs`

### GL Tracer (WebGL 2.0 Fragment Shader)
- Fragment shader-based raytracing using WebGL 2.0
- Implemented octree traversal in GLSL
- Located in `crates/renderer/src/gl_tracer.rs`
- Uses 3D texture for octree data

### GPU Tracer (Compute Shader)
- Compute shader-based raytracing
- Basic ray-cube intersection with slab method
- Located in `crates/renderer/src/gpu_tracer.rs`
- Uses compute shader dispatch with 8x8 work groups

## Architecture

### Files Created/Modified
- `crates/renderer/src/shaders/` - Shader directory structure
- `crates/renderer/src/shaders/basic_raycast.comp` - Compute shader for GPU tracer
- `crates/renderer/src/shader_utils.rs` - Shared shader compilation utilities
- `crates/renderer/src/gl_tracer.rs` - WebGL 2.0 fragment shader tracer
- `crates/renderer/src/gpu_tracer.rs` - Compute shader tracer
- `crates/renderer/src/cpu_tracer.rs` - CPU-based raytracer using cube.raycast()

### UI Layout
- 2x2 grid layout in egui app
- CPU | GL | GPU | Diff comparison views
- Dropdown ComboBoxes for diff source selection
- Camera controls for all views
- Performance metrics display for each renderer

## Key Implementations

### Shader Features
- Octree data structure in shader
- DDA octree traversal algorithm ported to GLSL
- Voxel value lookup in shader
- Empty space detection (voxel value == 0)
- Surface normals from entry face
- Coordinate space transformations
- Depth limiting
- Blinn-Phong lighting calculation

### GPU Tracer Features
- Ray-box intersection using slab method
- Time-based orbit camera support
- Explicit camera with quaternion rotation
- Blit shader for displaying compute shader output
- Proper resource management (init, render, destroy)

## Dependencies

- Requires completed `reimplement-raycast` change (for algorithm reference)
- Requires completed `integrate-cube-raycast` change (for CPU reference implementation)
- Uses WebGL/WebGPU APIs available in browser

## Affected Files

### Primary
- `crates/renderer/src/gpu_tracer.rs` - GPU tracer implementation
- `crates/renderer/src/gl_tracer.rs` - GL tracer implementation
- `crates/renderer/src/cpu_tracer.rs` - CPU tracer implementation
- `crates/renderer/src/shader_utils.rs` - Shared utilities
- `crates/renderer/src/shaders/*.comp` - Compute shaders
- `crates/renderer/src/egui_app.rs` - 2x2 grid UI

### Reference
- `crates/cube/src/raycast.rs` - CPU raycast algorithm

## Performance Target

- **Goal**: GPU tracer produces identical output to CPU tracer (verified by diff)
- **Target**: 10x or more performance improvement for typical scenes
- **Metric**: Frame time comparison between CPU and GPU tracers

## Success Criteria

1. GPU tracer produces identical output to CPU tracer (verified by diff)
2. Performance improvement of 10x or more for typical scenes
3. Handles all octree depths correctly
4. Correctly renders empty spaces (voxel value == 0)
5. All shader code passes validation
6. Code passes clippy with no warnings

## Development Environment

```bash
# Run renderer tests
cargo test -p crossworld-renderer

# Check renderer builds
cargo check -p crossworld-renderer

# Run egui app for visual comparison
cargo run -p renderer --example egui_app
```

## Key Commits

- `6c2f590` - Refactor renderer into 3-tracer architecture with 2x2 grid UI
- `d392da8` - Wire up all three tracers (CPU, GL, GPU) to raycast test report
