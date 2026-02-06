# renderer

OpenGL rendering implementations for voxel-based graphics.

## Features

### Renderers

The renderer includes five different implementations:

1. **CpuTracer** - Pure Rust software raytracer
   - No GPU required
   - Image file output (PNG)
   - Batch rendering support
   - Portable across all platforms

2. **GlTracer** - WebGL 2.0 fragment shader raytracer
   - Real-time rendering (60+ FPS)
   - OpenGL ES 3.0 compatible
   - X11/Wayland support

3. **BcfTracer** - Binary Cube Format traversal raytracer
   - BCF (Binary Cube Format) data structure
   - Optimized octree traversal
   - CPU-based rendering

4. **ComputeTracer** - GPU compute shader raytracer
   - OpenGL compute shader pipeline
   - Parallel pixel processing
   - High performance on modern GPUs

5. **MeshRenderer** - Triangle mesh rasterizer
   - Traditional vertex/index buffer rendering
   - Phong shading model
   - Mesh caching for performance

### Common Features

- **Shared Renderer Trait**: Common interface for all implementations
- **Identical Algorithms**: Same raytracing, lighting, and camera code
- **Material System**: R2G3B2 color encoding with palette support
- **Directional Lighting**: Diffuse + ambient lighting model
- **Animated Camera**: Rotating camera orbiting the cube
- **Debug Mode**: Optional lighting toggle for pure material colors
- **Diff Comparison**: Side-by-side output comparison with amplified differences

## GUI Application

The `CubeRendererApp` provides an egui-based GUI for comparing all five renderers:

```bash
# Default: Run GUI with all renderers
cargo run -p renderer

# Synchronized mode: All tracers use same timestamp/camera
cargo run -p renderer -- --sync

# Single frame mode: Render once and exit
cargo run -p renderer -- --single

# Headless diff comparison
cargo run -p renderer -- --single --headless --diff cpu gl
```

### UI Features

- **3x2 Grid Layout**: All five renderers displayed simultaneously
- **Diff Comparison Panel**: Select any two renderers to compare
- **Manual Camera Control**: Drag to orbit, scroll to zoom
- **Model Selector**: Switch between test models (octa cube, VOX models, etc.)
- **Mesh Caching**: Enable/disable mesh caching with regeneration button
- **Lighting Toggle**: Disable lighting for pure material color output

## Material System

The renderer uses a standardized material palette system:

### R2G3B2 Color Encoding

For material indices 128-255, colors are encoded as R2G3B2:
- Bits 7-6: Red (2 bits, 0-3)
- Bits 5-3: Green (3 bits, 0-7)
- Bits 2-0: Blue (2 bits, 0-3)

Common colors:
| Value | Color | R2G3B2 |
|-------|-------|--------|
| 224 | Red | (3,0,0) |
| 252 | Yellow | (3,7,0) |
| 156 | Green | (0,7,0) |
| 131 | Blue | (0,0,3) |
| 255 | White | (3,7,3) |

### Palette Colors (0-127)

Indices 0-127 use a predefined palette where:
- Index 0: Empty/transparent
- Indices 1-7: Reserved for error indicators
- Indices 8+: Application-defined colors

## Lighting Model

```rust
finalColor = materialColor * (AMBIENT + diffuse * DIFFUSE_STRENGTH)

where:
  diffuse = max(dot(normal, LIGHT_DIR), 0.0)
  AMBIENT = 0.3
  DIFFUSE_STRENGTH = 0.7
  LIGHT_DIR = normalize(0.5, 1.0, 0.3)
```

Gamma correction (γ = 2.2) is applied for proper sRGB display.

## Test Models

Available test models:

| Model | Depth | Description |
|-------|-------|-------------|
| SingleRedVoxel | 0 | Single solid voxel |
| OctaCube | 1 | 2x2x2 octree with 6 colored voxels |
| ExtendedOctaCube | 2 | Depth 2 with sparse/packed subdivisions |
| Depth3Cube | 3 | Complex structure with random subdivisions |
| VoxRobot | 5 | MagicaVoxel robot character |
| VoxAlienBot | 5 | MagicaVoxel alien bot |
| VoxEskimo | 5 | MagicaVoxel eskimo character |

## Mesh Caching

The mesh renderer supports caching to avoid regenerating the mesh every frame:

- **Cache Enabled**: Mesh is uploaded once and reused
- **Cache Disabled**: Mesh regenerated every frame (for debugging)
- **Manual Regeneration**: "Regen Mesh" button forces cache invalidation
- **Automatic Invalidation**: Cache cleared when model changes

## Code Structure

```
src/
├── main.rs              # Entry point, CLI argument parsing
├── lib.rs               # Library exports
├── renderer.rs          # Common Renderer trait and utilities
├── cpu_tracer.rs        # CpuTracer implementation
├── gl_tracer.rs         # GlTracer implementation
├── bcf_cpu_tracer.rs    # BcfTracer implementation
├── gpu_tracer.rs        # ComputeTracer implementation
├── mesh_renderer.rs     # MeshRenderer implementation
├── egui_app.rs          # CubeRendererApp GUI
├── shader_utils.rs      # OpenGL shader compilation
└── scenes/
    └── default_models.rs # Test model definitions
```

## Building and Running

### With Nix

```bash
# Using nix develop
nix develop
cargo run -p renderer

# Using nix-shell
nix-shell shell.nix --run 'DISPLAY=:0 cargo run -p renderer'
```

### On Linux

Ensure required libraries are installed:

**Debian/Ubuntu:**
```bash
sudo apt-get install libwayland-dev libxkbcommon-dev libgl1-mesa-dev \
  libx11-dev libxcursor-dev libxi-dev libxrandr-dev
cargo run -p renderer
```

**Arch Linux:**
```bash
sudo pacman -S wayland libxkbcommon mesa libx11 libxcursor libxi libxrandr
cargo run -p renderer
```

## CLI Options

```
Usage: renderer [OPTIONS]

Options:
  --console          Run in console mode (no GUI, batch CPU rendering)
  --sync             GUI with synchronized rendering
  --single           Render once and exit
  --headless         Force headless mode (with --single)
  --diff <L> <R>     Diff comparison between renderers (cpu, gl, bcf, compute, mesh)
  --diff all         Compare all renderer pairs

Examples:
  renderer --single                    # GUI: render one frame and exit
  renderer --single --headless         # Headless: render and save diff
  renderer --diff cpu compute          # Compare CPU vs Compute
  renderer --sync                      # Synchronized continuous rendering
```

## Dependencies

- `glow`: OpenGL bindings
- `glutin`: OpenGL context creation
- `glutin-winit`: Glutin-winit integration
- `winit`: Window creation and event handling
- `egui`: Immediate mode GUI
- `egui-glow`: egui OpenGL backend
- `glam`: Vector math library
- `image`: PNG encoding/decoding
- `cube`: Voxel octree data structure

## Testing

```bash
# Run all tests
cargo test --package renderer

# Color verification tests
cargo test --package renderer --test color_verification

# Render validation tests
cargo test --package renderer --test render_validation
```
