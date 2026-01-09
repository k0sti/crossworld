# Planet - Crossworld Native Voxel Editor

A native voxel editor for Crossworld built with Bevy 0.17.3. This editor provides a desktop-native experience for creating and editing voxel models and worlds.

## Features (In Development)

- **Native Performance**: Built with Bevy game engine for fast, responsive editing
- **Voxel Editing**: Place, remove, and paint voxels with intuitive controls
- **Camera Controls**: Orbit, pan, and zoom with mouse interactions
- **Material System**: Palette-based colors matching Crossworld's material specification
- **File I/O**: Import/export CSM (CubeScript Model) and MagicaVoxel (.vox) files
- **Undo/Redo**: Full editing history with keyboard shortcuts

## Building and Running

### Prerequisites

#### Linux (Recommended)

For the best development experience on Linux, use the provided Nix flake:

```bash
# From crates/editor directory
nix develop

# This provides:
# - Rust toolchain with rust-analyzer
# - All Bevy system dependencies (udev, alsa, Vulkan, X11, Wayland)
# - mold linker for fast incremental builds
# - Development tools (just, gdb, etc.)
```

Without Nix, install Bevy dependencies manually:
```bash
# Ubuntu/Debian
sudo apt install pkg-config libudev-dev libasound2-dev libxkbcommon-dev \
  libwayland-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev \
  vulkan-tools libvulkan-dev

# Fedora
sudo dnf install pkgconf-pkg-config libudev-devel alsa-lib-devel libxkbcommon-devel \
  wayland-devel libX11-devel libXcursor-devel libXi-devel libXrandr-devel \
  vulkan-tools vulkan-loader-devel
```

For more details, see: https://github.com/bevyengine/bevy/blob/latest/docs/linux_dependencies.md

#### Windows

No additional dependencies required. Ensure you have:
- Rust toolchain (https://rustup.rs)
- Visual Studio Build Tools

#### macOS

```bash
# No additional dependencies required
# Ensure Xcode Command Line Tools are installed:
xcode-select --install
```

### Build Commands

From the project root (`planet-cw/`):

```bash
# Development build (fast compilation)
cargo run --bin planet
# or
just planet

# Release build (optimized)
cargo run --release --bin planet
# or
just planet-release

# Check for errors without running
cargo check --bin planet

# Run tests
cargo test --package editor
```

### Build Optimizations

The project is configured with several optimizations for faster development:

**Linux users get:**
- **mold linker**: 5-10x faster incremental linking (configured in `.cargo/config.toml`)
- **cranelift codegen**: 2-3x faster debug builds with `cargo +nightly build`

**All platforms get:**
- Custom build profiles (`fast-dev`, `release`, `dist`)
- Incremental compilation enabled
- Parallel codegen for faster builds

## Architecture

### Project Structure

```
crates/editor/
├── Cargo.toml          # Dependencies and binary configuration
├── flake.nix           # Nix development environment (Linux)
├── src/
│   ├── main.rs         # Bevy app entry point
│   ├── camera.rs       # Orbit camera controller (TODO)
│   ├── voxel_scene.rs  # Voxel scene management (TODO)
│   ├── raycast.rs      # Raycasting for voxel picking (TODO)
│   ├── editing.rs      # Voxel editing operations (TODO)
│   ├── ui/             # UI panels and widgets (TODO)
│   └── ...
└── README.md          # This file
```

### Dependencies

- **bevy = "0.17.3"**: Game engine and renderer
- **bevy_egui = "0.38.0"**: Immediate-mode UI
- **cube**: Octree data structure and mesh generation
- **crossworld-world**: Multi-depth world generation
- **serde/serde_json**: Serialization for save files
- **rfd**: Native file dialogs
- **glam**: Math library (vectors, matrices)

### Integration with Existing Crates

The editor reuses existing Crossworld infrastructure:

- **`cube` crate**: Octree operations, CSM parsing, mesh generation
- **`crossworld-world` crate**: Multi-depth world editing, procedural terrain

This ensures compatibility between native editor and web application.

## Development Status

**Phase 0: Build Environment** ✅ Complete
- `.cargo/config.toml` with mold linker configuration
- `flake.nix` for Nix users
- `just planet` command in justfile

**Phase 1: Basic Scaffold** 🔨 In Progress
- [x] Bevy app initialization
- [x] Window and camera setup
- [x] Basic lighting and ground plane
- [ ] Voxel rendering integration
- [ ] Verify cube crate mesh generation

**Phase 2-18: Core Features** 📋 Planned
- Camera controls (orbit, pan, zoom, modes)
- Raycasting and cursor system
- Voxel editing (place, remove, paint)
- Material and brush UI
- File I/O (CSM, .vox)
- Undo/redo system
- Keyboard shortcuts
- Documentation

## Keyboard Shortcuts (Planned)

### File Operations
- `Ctrl+N`: New scene
- `Ctrl+O`: Open file
- `Ctrl+S`: Save file
- `Ctrl+Shift+S`: Save as

### Editing
- `Left Click`: Place voxel/brush
- `Shift+Left Click`: Remove voxel/brush
- `Ctrl+Z`: Undo
- `Ctrl+Y` or `Ctrl+Shift+Z`: Redo
- `Ctrl+C`: Copy selection to brush
- `Delete`: Remove voxel at cursor

### Camera
- `Right Click + Drag`: Rotate camera
- `Middle Click + Drag`: Pan camera
- `Scroll Wheel`: Zoom in/out
- `F`: Frame all content
- `C`: Toggle camera mode (LookAt ↔ Free)

### Tools
- `Tab`: Toggle focus mode (Near ↔ Far)
- `M`: Toggle paint mode (Single ↔ Brush)
- `[` / `]`: Decrease / Increase cursor size
- `1-9`: Quick select materials 1-9

## Contributing

When adding features:

1. Add Bevy systems to `EditorPlugin`
2. Keep resources and components organized by feature
3. Test integration with `cube` and `crossworld-world` crates
4. Update this README with new features

## Troubleshooting

### Build fails with linker errors on Linux

Ensure mold is installed:
```bash
# Ubuntu/Debian
sudo apt install mold

# Fedora
sudo dnf install mold

# Or use Nix flake which includes it
nix develop
```

### Window doesn't open / Graphics errors

**Linux**: Ensure Vulkan drivers are installed and working:
```bash
vulkaninfo  # Should show GPU info
```

**All platforms**: Try running with debug logging:
```bash
RUST_LOG=info cargo run --bin planet
```

### Slow compilation

Use the fast-dev profile:
```bash
cargo build --profile fast-dev --bin planet
```

Or use cranelift on nightly (Linux):
```bash
cargo +nightly build -Z codegen-backend=cranelift --bin planet
```

## Resources

- [Bevy Documentation](https://bevyengine.org/learn/)
- [Bevy Linux Dependencies](https://github.com/bevyengine/bevy/blob/latest/docs/linux_dependencies.md)
- [bevy_egui Crate](https://docs.rs/bevy_egui/)
- [Crossworld Project](../../README.md)

## License

MIT OR Apache-2.0
