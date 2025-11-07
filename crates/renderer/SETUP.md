# Renderer Setup and Usage

## Quick Start (NixOS/Nix users)

The easiest way to run the renderer is using the provided run script:

```bash
cd crates/renderer
./run.sh
```

This will automatically:
- Load all required dependencies via Nix
- Set up X11 display
- Build and run the renderer in release mode

## What the Renderer Does

The renderer displays a 3D cube with:
- Real-time GPU raytracing in a fragment shader
- Directional lighting with ambient and diffuse components
- Animated rotating camera
- Color variation per face based on surface normals
- Fresnel edge highlighting
- Gamma-corrected output

## Architecture

### Backend Selection
The renderer uses **X11 as the primary backend** on Linux with Wayland as a fallback. This is configured in `src/main.rs:422-428`:

```rust
#[cfg(target_os = "linux")]
let event_loop = {
    let mut builder = EventLoop::builder();
    builder.with_x11();  // Force X11 backend
    builder.build()?
};
```

This approach was chosen because:
1. X11 has better compatibility across different environments
2. Wayland requires a running compositor which may not be available
3. The glow/glutin stack works reliably with X11

### Nix Integration

Three Nix configuration files are provided:

1. **`flake.nix`** - Modern Nix flake with development shell
2. **`shell.nix`** - Traditional nix-shell environment
3. **`.envrc`** - direnv integration for automatic environment loading

All three provide the same runtime dependencies:
- Wayland libraries (for potential fallback)
- X11 libraries (primary backend)
- OpenGL/Vulkan libraries
- Build tools (pkg-config, cmake)

## Troubleshooting

### "NoCompositor" Error
If you see `WaylandError(Connection(NoCompositor))`, this means:
- The app tried to use Wayland but no compositor is running
- Solution: The code now forces X11, rebuild with the latest changes

### "libX11.so.6: cannot open shared object file"
This means X11 libraries aren't in your library path:
- Solution: Run through `nix-shell` or use `./run.sh`

### "DISPLAY not set" or Display errors
The renderer needs an X11 display:
- Check: `echo $DISPLAY` (should show `:0` or similar)
- Fix: `export DISPLAY=:0`
- The run.sh script sets this automatically

## Files Created

- `src/main.rs` - Main renderer implementation
- `Cargo.toml` - Rust dependencies
- `flake.nix` - Nix flake configuration
- `shell.nix` - Nix shell configuration
- `.envrc` - direnv configuration
- `run.sh` - Convenient run script
- `README.md` - Project documentation
- `SETUP.md` - This file

## Next Steps

To extend this renderer for octree rendering:

1. **Add octree data structure** - Define octree nodes in Rust
2. **Pass octree to GPU** - Use texture buffers or SSBOs
3. **Implement octree traversal** - Ray-octree intersection in shader
4. **Add level-of-detail** - Stop traversal at appropriate depth
5. **Optimize rendering** - Sparse voxel octree techniques
