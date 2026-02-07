# GPU Raytracer Shaders

This directory contains GLSL ES 3.0 shaders for GPU-based octree raytracing.

## Files

- `octree_raycast.vert` - Fullscreen triangle vertex shader
- `octree_raycast.frag` - Octree raytracing fragment shader with DDA traversal

## Algorithm

The fragment shader implements the same DDA octree traversal algorithm as the CPU raytracer in `crates/cube/src/raycast/mod.rs`, but optimized for parallel GPU execution.

### Key Features

1. **Octree Traversal**: Recursive DDA stepping through octree hierarchy
2. **Empty Space Skipping**: Efficiently jumps over empty voxels
3. **Normal Calculation**: Computes surface normals from entry face
4. **Lighting**: Directional lighting with ambient and diffuse components
5. **3D Texture Lookup**: Voxel data stored as 3D texture for fast access

### Coordinate Systems

- **World Space**: Camera and cube positions (-1 to 1 range)
- **Normalized Space**: Octree traversal uses [0,1]³ coordinates
- **Texture Space**: 3D texture coordinates for voxel lookup

## Performance

The GPU implementation provides significant performance improvements over CPU raytracing:

- Parallel execution across all pixels
- Hardware-accelerated texture sampling
- Optimized for high-resolution rendering

## Future Enhancements

- [ ] Hierarchical octree texture representation
- [ ] Support for arbitrary octree depths
- [ ] Material system for varied voxel colors
- [ ] Shadow casting
- [ ] Ambient occlusion
