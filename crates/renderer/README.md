# Renderer

A WebGL2-based renderer for cube octrees with raytracing capabilities using the glow crate.

## Features

- **WebGL2/OpenGL ES 3.0 Support**: Uses the glow crate for cross-platform OpenGL rendering
- **GPU Raytracing**: Fragment shader-based raytracing for rendering 3D cubes
- **Directional Lighting**: Real-time diffuse and ambient lighting with Fresnel edge highlighting
- **Animated Camera**: Rotating camera that orbits around the scene
- **Real-time Rendering**: Continuous rendering loop with time-based animations

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

**Note:** The renderer requires an X11 display server. Make sure `DISPLAY` is set correctly (typically `:0` or `:1`).

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

## Dependencies

- `glow`: OpenGL bindings
- `glutin`: OpenGL context creation
- `glutin-winit`: Glutin-winit integration
- `winit`: Window creation and event handling
- `raw-window-handle`: Window handle abstractions

## Future Extensions

This renderer is designed to be extended for octree rendering:
- Add octree data structures
- Implement octree traversal in shader
- Add level-of-detail (LOD) support
- Support for voxel colors and materials
- Optimized sparse voxel octree (SVO) rendering
