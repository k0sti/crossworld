# Renderer

Dual-implementation cube raytracer with both GPU (WebGL2/OpenGL) and CPU (pure Rust) backends.

## Features

### Common
- **Shared Renderer Trait**: Common interface for both implementations
- **Identical Algorithms**: Same raytracing, lighting, and camera code
- **Material System**: 128-color palette with standardized voxel materials
- **Directional Lighting**: Simplified diffuse + ambient lighting model
- **Animated Camera**: Rotating camera orbiting the cube
- **Debug Mode**: Optional lighting toggle for pure material color output

### GlCubeTracer (GPU Implementation)
- **WebGL2/OpenGL ES 3.0**: Fragment shader-based raytracing
- **Real-time Rendering**: 60+ FPS interactive display
- **X11/Wayland Support**: Cross-platform window management

### CpuCubeTracer (Pure Rust Implementation)
- **No GPU Required**: Software raytracing on CPU
- **Image Output**: Renders to PNG files
- **Batch Rendering**: Generate animation frames
- **Portable**: Works on any platform with Rust

## Material System

The renderer uses a standardized material palette system for voxel colors:

### Material Palette

- **Total materials**: 128 (indices 0-127)
- **Index 0**: Reserved for empty/transparent voxels
- **Indices 1-127**: Solid voxel materials with distinct colors

### Test Palette

For renderer testing, a minimal 7-color palette is defined in `src/materials.rs`:

| Index | Name | Color (RGB) | Hex | Usage |
|-------|------|-------------|-----|-------|
| 0 | Empty | (0.0, 0.0, 0.0) | #000000 | Transparent/Air |
| 1 | Red | (1.0, 0.0, 0.0) | #FF0000 | Solid voxel |
| 2 | Green | (0.0, 1.0, 0.0) | #00FF00 | Solid voxel |
| 3 | Blue | (0.0, 0.0, 1.0) | #0000FF | Solid voxel |
| 4 | Yellow | (1.0, 1.0, 0.0) | #FFFF00 | Solid voxel |
| 5 | White | (1.0, 1.0, 1.0) | #FFFFFF | Solid voxel |
| 6 | Black | (0.0, 0.0, 0.0) | #000000 | Solid voxel |

### Usage in Code

```rust
use renderer::materials::get_material_color;

// Get material color for a voxel value
let color = get_material_color(cube_hit.value);
// Returns Vec3 with RGB values in range [0.0, 1.0]
```

### Lighting Toggle

Both CPU and GL tracers support disabling lighting for debugging:

```rust
let mut tracer = CpuCubeTracer::new();

// Render with lighting (default)
tracer.set_disable_lighting(false);
tracer.render_with_camera(width, height, &camera);

// Render pure material colors (no lighting)
tracer.set_disable_lighting(true);
tracer.render_with_camera(width, height, &camera);
```

This is useful for:
- Verifying exact material colors in tests
- Debugging material palette issues
- Creating color-accurate texture atlases

## Lighting Model

The renderer uses a simplified physically-based lighting model:

### Constants

```rust
// Light direction (normalized)
pub const LIGHT_DIR: Vec3 = Vec3::new(0.431934, 0.863868, 0.259161);

// Lighting coefficients
pub const AMBIENT: f32 = 0.3;           // Base ambient light
pub const DIFFUSE_STRENGTH: f32 = 0.7;  // Diffuse multiplier

// Background color
pub const BACKGROUND_COLOR: Vec3 = Vec3::new(0.4, 0.5, 0.6); // Bluish gray
```

### Formula

```
finalColor = materialColor * (AMBIENT + diffuse * DIFFUSE_STRENGTH)

where:
  diffuse = max(dot(normal, LIGHT_DIR), 0.0)  // Lambert's cosine law
  materialColor = palette lookup from voxel value
```

### Gamma Correction

All rendered output applies gamma correction (γ = 2.2) for proper sRGB display:

```rust
color = color.powf(1.0 / 2.2);  // Linear to sRGB
```

This means:
- **Background color**: RGB(0.4, 0.5, 0.6) in linear space → RGB(170, 186, 201) after gamma
- **Material colors**: Scaled by lighting, then gamma corrected

## Implementation Details

### Shaders

#### Vertex Shader
- Simple fullscreen triangle approach (no vertex buffers needed)
- Uses gl_VertexID to generate positions

#### Fragment Shader
- Ray-box intersection algorithm for cube rendering
- Octree traversal with DDA algorithm for voxel raycasting
- Material palette lookup from voxel values
- Accurate surface normal calculation from octree hit
- Directional light with fixed direction
- Simplified ambient + diffuse lighting model (no fresnel)
- Gamma correction for proper sRGB color output
- Optional lighting disable via uniform flag

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

The renderer displays an octree-based voxel cube (2×2×2 "octa cube" test scene):
- **6 solid voxels**: Red, Green, Blue, Yellow, White (×2)
- **2 empty spaces**: Transparent octants for depth testing
- Rotating camera orbits around the cube
- Material-based colors (no normal-based variation)
- Directional lighting from direction (0.431934, 0.863868, 0.259161)

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

## Testing

The renderer includes comprehensive test coverage:

### Color Verification Tests (`tests/color_verification.rs`)
- **Material color tests**: Verify CPU tracer renders distinct palette colors
- **Background color test**: Verify bluish-gray background (gamma-corrected)
- **Lighting toggle test**: Verify `disable_lighting` produces pure colors
- **Material palette test**: Verify all 7 test colors are accessible
- **Lighting constants test**: Verify light direction normalization and ranges

Run tests:
```bash
cargo test --package renderer --test color_verification
```

### Integration Tests
- **Octa cube rendering** (`tests/octa_cube_rendering.rs`): Multi-angle rendering validation
- **Render validation** (`tests/render_validation.rs`): Visible output verification
- **Combined tracer test** (`tests/combined_render_test.rs`): CPU/GL/GPU parity testing

## Future Extensions

Potential improvements and extensions:
- Expand material palette to full 128 entries with texture properties
- Add level-of-detail (LOD) support for large octrees
- Implement deeper octree traversal (currently depth 1)
- Add texture mapping and UV coordinates
- Implement shadows and reflections
- Optimized sparse voxel octree (SVO) rendering for massive scenes
