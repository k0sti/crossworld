# Refactor GL Tracer to Use Hierarchical Octree Traversal

> **⚠️ BLOCKED**: This change is **blocked** by `reimplement-raycast` (89/97 tasks).
> CPU raycast must be working correctly before GL tracer can be validated.
> **Priority**: Complete `reimplement-raycast` first, then return to this change.

## Problem Statement

The current GL tracer (`crates/renderer/src/gl_tracer.rs`) uses 3D texture sampling to render voxel data, which is architecturally incorrect for hierarchical octree structures. The implementation:

1. **Samples the octree into a flat 8x8x8 3D texture** (lines 211-282 in `gl_tracer.rs`)
2. **Uses `sampler3D` in shaders** for texture lookups (line 19, 124 in `octree_raycast.frag`)
3. **Loses hierarchical structure**, treating octrees as dense voxel grids
4. **Limits resolution** to fixed grid sizes (currently 8³ = 512 voxels)
5. **Wastes GPU memory** by storing empty space as explicit voxels

This approach contradicts the fundamental design principle: **all data is hierarchical octree voxels with material in leaves**.

## Proposed Solution

Refactor the GL tracer to use proper hierarchical octree traversal on the GPU:

1. **Serialize octree to linearized buffer** using BCF-inspired format suitable for GPU
2. **Replace 3D texture with texture buffer** or SSBO (Shader Storage Buffer Object)
3. **Implement DDA-based hierarchical traversal** in fragment shader
4. **Maintain compatibility** with existing renderer interface and test framework

## Benefits

- **Architectural Correctness**: Aligns with octree-first design philosophy
- **Memory Efficiency**: Only stores non-empty octree nodes
- **Scalability**: Supports arbitrary octree depths without grid size limits
- **Performance**: DDA traversal efficiently skips empty space
- **Consistency**: GL tracer behavior matches CPU tracer behavior

## Scope

### In Scope
- Octree serialization to GPU-friendly linearized buffer format
- Fragment shader rewrite with hierarchical DDA traversal
- Rust GL code updates (buffer upload, uniform binding)
- Validation against CPU tracer output for correctness

### Out of Scope
- GPU tracer (compute shader) - separate crate, different approach
- BCF format parser/writer (already specified in `doc/architecture/bcf-format.md`)
- Performance optimization beyond basic implementation
- Support for `Planes` and `Slices` variants (focus on `Solid` and `Cubes`)

## Dependencies

### Blocking Dependencies (Must Complete First)
- **`reimplement-raycast` (89/97 tasks)** - CRITICAL BLOCKER
  - CPU raycast must be working correctly before GL tracer validation
  - GL tracer correctness is validated by comparing against CPU tracer output
  - Cannot proceed with this change until CPU raycast is fully functional
  - **Priority**: Fix CPU raycast first, then unblock this change

### Required Dependencies
- Existing BCF format specification (`doc/architecture/bcf-format.md`)
- GL tracer test infrastructure (`crates/renderer/src/main.rs`)

### Optional Dependencies
- `add-binary-cube-format` (70/118 tasks) - Provides BCF format foundation (informational only)

## Risks and Mitigations

### Risk: WebGL 2.0 Buffer Size Limits
- **Mitigation**: Use texture buffers (up to 128MB on most hardware) instead of uniforms
- **Fallback**: Implement depth limiting if buffer exceeds device limits

### Risk: Shader Complexity
- **Mitigation**: Port proven CPU DDA algorithm incrementally
- **Validation**: Pixel-perfect diff against CPU tracer output

### Risk: Performance Regression
- **Mitigation**: Profile both approaches, accept initial slowdown for correctness
- **Future**: Optimization pass in separate change after validation

## Success Criteria

1. **Correctness**: GL tracer output matches CPU tracer (pixel-perfect or <1% diff)
2. **No 3D Textures**: All `TEXTURE_3D`, `sampler3D` references removed
3. **Hierarchical Traversal**: Shader implements proper octree descent with DDA
4. **Tests Pass**: All existing renderer tests pass with new implementation
5. **Documentation**: Code comments explain buffer format and traversal algorithm

## Related Changes

- `add-binary-cube-format` (70/118 tasks) - Provides BCF format foundation
- `implement-gpu-raytracer` (46/51 tasks) - Separate GPU compute implementation
