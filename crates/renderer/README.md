# Renderer

Dual-implementation cube raytracer with both GPU (WebGL2/OpenGL) and CPU (pure Rust) backends.

## Features

### Common
- **Shared Renderer Trait**: Common interface for both implementations
- **Identical Algorithms**: Same raytracing, lighting, and camera code
- **Directional Lighting**: Diffuse + ambient lighting with Fresnel edge highlighting
- **Animated Camera**: Rotating camera orbiting the cube

### GlCubeTracer (GPU Implementation)
- **WebGL2/OpenGL ES 3.0**: Fragment shader-based raytracing
- **Real-time Rendering**: 60+ FPS interactive display
- **X11/Wayland Support**: Cross-platform window management

### CpuCubeTracer (Pure Rust Implementation)
- **No GPU Required**: Software raytracing on CPU
- **Image Output**: Renders to PNG files
- **Batch Rendering**: Generate animation frames
- **Portable**: Works on any platform with Rust

## Implementation Details

### Shaders

#### Vertex Shader
- Simple fullscreen triangle approach (no vertex buffers needed)
- Uses gl_VertexID to generate positions

#### Fragment Shader
- Ray-box intersection algorithm for cube rendering
- Accurate surface normal calculation
- Directional light with configurable direction
- Ambient + diffuse lighting model
- Fresnel effect for edge highlighting
- Gamma correction for proper color output

### Rendering Pipeline

1. Initialize OpenGL ES 3.0 context via glutin
2. Compile and link vertex/fragment shaders
3. Create VAO (Vertex Array Object)
4. Each frame:
   - Clear screen
   - Update uniforms (resolution, time)
   - Draw fullscreen triangle
   - Fragment shader performs raytracing per pixel

### Current Implementation

The renderer currently displays a single 3D cube with bounds (-1, 1) in all axes:
- Rotating camera orbits around the cube
- Different colors for each face (based on surface normal)
- Directional lighting from direction (0.5, 1.0, 0.3)

## Building and Running

### On NixOS or with Nix

The project includes Nix flake and shell configurations for managing dependencies. The renderer uses X11 backend by default (with Wayland fallback support):

```bash
# Option 1: Using the provided run script (recommended)
./run.sh

# Option 2: Using nix-shell directly
nix-shell shell.nix --run 'DISPLAY=:0 cargo run --release'

# Option 3: Using nix develop (from flake)
nix develop
DISPLAY=:0 cargo run -p renderer

# Option 4: Using direnv (if installed)
direnv allow
DISPLAY=:0 cargo run -p renderer
```

**Note:** The GL renderer requires an X11 display server. Make sure `DISPLAY` is set correctly (typically `:0` or `:1`).

### Running the CPU Renderer

The CPU renderer doesn't require a display server and outputs PNG files:

```bash
# Run CPU renderer (outputs 10 frames)
cargo run --release -- --cpu

# Or with nix-shell
nix-shell shell.nix --run 'cargo run --release -- --cpu'
```

Output: `output_frame_000.png` through `output_frame_009.png`

### On other Linux systems

Ensure you have the required libraries installed:

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

**Fedora:**
```bash
sudo dnf install wayland-devel libxkbcommon-devel mesa-libGL-devel \
  libX11-devel libXcursor-devel libXi-devel libXrandr-devel
cargo run -p renderer
```

## Code Structure

```
src/
├── main.rs          # Entry point, dual-mode runner
├── renderer.rs      # Common Renderer trait and shared code
├── gl_tracer.rs     # GlCubeTracer (GPU implementation)
└── cpu_tracer.rs    # CpuCubeTracer (CPU implementation)
```

### Shared Code (renderer.rs)

- `Renderer` trait - Common interface
- `intersect_box()` - Ray-box intersection algorithm
- `create_camera_ray()` - Camera ray generation
- `calculate_lighting()` - Lighting calculations

Both implementations use identical algorithms, just in different languages (GLSL vs Rust).

## Dependencies

- `glow`: OpenGL bindings
- `glutin`: OpenGL context creation
- `glutin-winit`: Glutin-winit integration
- `winit`: Window creation and event handling
- `raw-window-handle`: Window handle abstractions
- `glam`: Vector math library
- `image`: PNG encoding/decoding

## Future Extensions

This renderer is designed to be extended for octree rendering:
- Add octree data structures
- Implement octree traversal in shader
- Add level-of-detail (LOD) support
- Support for voxel colors and materials
- Optimized sparse voxel octree (SVO) rendering
