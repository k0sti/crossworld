# Change: Create Bevy-based Voxel Editor

## Why

The project currently has:
1. A web-based TypeScript editor (`packages/editor`) focused on integration with the Chakra UI React app
2. A native OpenGL renderer (`crates/renderer`) with low-level glow/glutin/egui that serves as a test harness

We need a dedicated native voxel editor application with:
- Better ergonomics and UI/UX for content creators building voxel models
- Faster iteration cycles (no web browser overhead)
- Native performance for complex voxel scenes
- Modern Rust game engine architecture (Bevy ECS)
- Reuse of existing `cube` and `world` crate infrastructure

This fills the gap between the basic web editor and the low-level OpenGL test renderer, providing a production-quality tool for artists and level designers.

## What Changes

- **NEW**: Create `crates/editor` - Standalone Bevy application for voxel editing
- Reuse existing `cube` crate for octree operations, mesh generation, CSM parsing
- Reuse existing `world` crate for multi-depth world generation and editing
- Implement Bevy systems for:
  - 3D viewport with camera controls (orbit, pan, zoom)
  - Voxel placement/removal via raycasting
  - CSM import/export UI
  - Material palette selection
  - Scene hierarchy and layer management
  - Undo/redo system
- Desktop-native UI using `bevy_egui` for immediate-mode panels
- File operations: Load/save `.vox` files, CSM text format
- Export functionality for use in web application

## Impact

**New capabilities:**
- `bevy-voxel-editor` - Full specification for the native editor application

**Affected code:**
- **NEW**: `crates/editor/` - New Bevy application crate (renamed binary: `planet`)
- **NEW**: `.cargo/config.toml` - Linux build optimizations (mold linker, cranelift)
- **NEW**: `flake.nix` - Nix flake for Bevy dependencies on Linux
- **NO CHANGES**: `crates/cube/` - Used as-is (already has all needed APIs)
- **NO CHANGES**: `crates/world/` - Used as-is (already has WorldCube API)
- `Cargo.toml` - Add new workspace member, rename binary to `planet`
- `justfile` - Add new task `just planet` for running the editor

**Dependencies:**
- Primary: `cube`, `world` (existing crates)
- New: `bevy = "0.17.3"` (game engine), `bevy_egui` (UI, compatible with 0.17), `bevy_rapier3d` (optional physics preview)
- Build tooling: `mold` linker, `cranelift` codegen for fast iteration

**Non-breaking:**
- Existing web editor (`packages/editor`) continues to work unchanged
- Existing renderer (`crates/renderer`) can coexist (different use case)
- No API changes to `cube` or `world` crates
