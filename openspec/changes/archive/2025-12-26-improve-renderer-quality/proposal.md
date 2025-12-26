# Improve Renderer Quality and Architecture

## Summary

Fix rendering quality issues in the renderer crate's mesh renderer, improve code architecture by renaming components for clarity, add mesh caching options, and update documentation.

## Motivation

The renderer crate currently has several quality and usability issues:

1. **Mesh renderer camera calculation is incorrect** - The mesh renderer doesn't properly synchronize camera configuration with other renderers (CPU, GL, GPU), resulting in different viewpoints and making comparison difficult.

2. **Mesh face rendering has visual artifacts** - Front/back face culling may be inverted or incorrectly configured, causing rendering issues.

3. **Inefficient mesh regeneration** - The mesh is potentially regenerated every frame even when the voxel data hasn't changed, wasting GPU upload bandwidth and CPU cycles.

4. **Unclear naming** - "DualRenderer" is now a misnomer as the system has expanded to five renderers (CPU, GL, BCF CPU, GPU, Mesh). The name should reflect its role as a comprehensive cube rendering comparison tool.

5. **Outdated documentation** - README and documentation still refer to "dual" renderer and don't document the mesh renderer or recent additions.

## Goals

1. Fix mesh renderer camera synchronization to match other renderers
2. Fix mesh renderer face culling/rendering issues
3. Add option to cache/regenerate mesh on demand instead of every frame
4. Rename `DualRendererApp` to `CubeRendererApp` throughout the codebase
5. Update README.md and documentation to reflect current capabilities

## Non-Goals

- Rewriting the mesh generation algorithm
- Adding new rendering backends
- Changing the GUI framework or layout
- Performance optimization beyond mesh caching

## Impact

### Users
- **Positive**: Mesh renderer will produce correct output for visual comparison
- **Positive**: Mesh caching option will improve frame rates when editing voxel data
- **Positive**: Clearer naming makes code easier to understand
- **Neutral**: Documentation updates reflect current state

### Codebase
- **Low Impact**: Mostly local changes to renderer crate
- **Renaming**: `DualRendererApp` → `CubeRendererApp` (search and replace)
- **New Feature**: Mesh cache toggle in UI settings

### Dependencies
- **None**: No external dependency changes

## Open Questions

1. **Mesh cache invalidation**: Should mesh cache be invalidated automatically when model changes, or require manual regeneration button?
   - **Recommendation**: Automatic invalidation on model change + manual regeneration button for user control

2. **Default mesh caching state**: Should mesh caching be enabled by default?
   - **Recommendation**: Enabled by default for better performance, with clear UI indication

3. **Camera sync validation**: Should we add validation tests to ensure all renderers use the same camera?
   - **Recommendation**: Yes, add integration test comparing camera matrices across renderers

## Alternatives Considered

### Alternative 1: Keep "DualRenderer" name
**Rejected**: Misleading as system now has 5 renderers. Would cause confusion for future contributors.

### Alternative 2: Remove mesh renderer instead of fixing it
**Rejected**: Mesh renderer provides unique value for validating mesh generation pipeline and comparing rasterization vs raytracing.

### Alternative 3: Always regenerate mesh
**Rejected**: Wasteful for static scenes. Caching with invalidation is standard practice.

## Related Work

- Mesh generation pipeline in `cube` crate (`generate_face_mesh`)
- Camera configuration in `renderer::CameraConfig`
- Entity system for mesh transforms
- Other renderer backends (CPU, GL, BCF CPU, GPU) for comparison reference
