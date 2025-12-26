# Specification: Bevy Voxel Editor

## Overview

Create a standalone Bevy-based voxel editor (`crates/editor`) with production-quality tools for content creators to build voxel models. This fills the gap between the basic web editor and the low-level OpenGL test renderer.

## Source

Migrated from: `openspec/changes/create-bevy-voxel-editor/`

## Current Status

**Completion: ~49% (Phases 0-4, 6 largely complete, remaining phases pending)**

### Completed Work
- Phase 0: Build Environment Setup (7/9) - mold linker, cranelift, nix flake, justfile
- Phase 1: Project Setup and Scaffold (15/15) - editor crate, window, cube integration
- Phase 2: Camera Controls (8/8) - orbit camera, keyboard shortcuts
- Phase 3: Voxel Scene Management (10/10) - VoxelScene, mesh sync, utilities
- Phase 4: Raycasting and Cursor System (16/19) - raycast, cursor, focus mode, gizmos
- Phase 6: Voxel Editing (partial) - EditorState, placement, removal, material selection

### Pending Work
- Phase 5: Paint Modes and Brush System - brush loading, scale, copy
- Phase 7: Camera Mode System - LookAt/Free modes, frame scene
- Phase 8: Material and Brush UI - palette, brush selector
- Phase 10: File I/O - CSM and .vox import/export
- Phase 17: Undo/Redo System - command history
- Phase 12: Keyboard Shortcuts - comprehensive input handling
- Phases 13-17: Preferences, Polish, Testing, Documentation, Deployment

## Problem Statement

**Goal**: Provide a production-quality native voxel editor for artists and level designers.

**Key capabilities**:
1. 3D viewport with orbit camera and keyboard shortcuts
2. Voxel placement/removal via raycasting with multi-size cursor
3. Material palette and brush system
4. File I/O for CSM and .vox formats
5. Undo/redo system for editing history

## Architecture

### Core Components

```
crates/editor/
  src/
    main.rs           - Bevy app initialization
    camera.rs         - OrbitCamera and controls
    cursor.rs         - CubeCursor and FocusMode
    raycast.rs        - EditorRaycast and hit detection
    voxel_scene.rs    - VoxelScene with ThreadSafeWorldCube
    mesh_sync.rs      - Mesh regeneration system
    editing.rs        - Voxel placement/removal
    paint_mode.rs     - Single/Brush mode switching (pending)
    brush.rs          - VoxelBrush and BrushLibrary (pending)
    materials.rs      - MaterialPalette (pending)
    file_io.rs        - CSM/VOX import/export (pending)
    history.rs        - Undo/redo command history (pending)
    input.rs          - Keyboard shortcuts (pending)
    ui/
      mod.rs          - UiPlugin
      toolbar.rs      - Top toolbar with menus
      status.rs       - Status bar
      inspector.rs    - Scene inspector panel
      palette.rs      - Material palette UI
      brush_selector.rs - Brush selection grid
      shortcuts.rs    - Keyboard shortcuts help
  flake.nix           - NixOS development environment
  README.md           - Usage documentation
```

### Dependencies
- `bevy = "0.17.3"` - Game engine and ECS
- `bevy_egui` - Immediate-mode UI for panels
- `cube = { path = "../cube" }` - Voxel octree and mesh generation
- `crossworld-world = { path = "../world" }` - WorldCube editing API
- `serde`, `serde_json` - Configuration and file I/O
- `rfd = "0.15"` - Native file dialogs

### Key Features

| Feature | Description | Status |
|---------|-------------|--------|
| Orbit Camera | Right-click rotate, scroll zoom | Complete |
| Camera Shortcuts | F=frame, 1/3/7=views, Home=reset | Complete |
| VoxelScene | WorldCube wrapper with dirty flag | Complete |
| Mesh Sync | Auto-regenerate mesh on changes | Complete |
| Raycasting | Camera-to-world ray intersection | Complete |
| Cursor | Multi-size (1-16) with gizmo visualization | Complete |
| Focus Mode | Near (removal) / Far (placement) | Complete |
| Voxel Editing | Left-click place, Shift+click remove | Complete |
| Material Selection | 0-9 keys select materials | Complete |
| Paint Modes | Single / Brush mode switching | Pending |
| Brush System | Load .vox files as brushes | Pending |
| Material Palette | UI grid with color swatches | Pending |
| File I/O | Open/Save CSM and .vox | Pending |
| Undo/Redo | Command-based history | Pending |

## Configuration

Via Bevy window configuration in `main.rs`:
- Window title: "Crossworld Voxel Editor"
- Default size: 1280x720
- VSync enabled

WorldCube parameters (startup):
- macro_depth: 3
- micro_depth: 5
- border_depth: 1

## Affected Files

### Primary (to modify/create)
- `crates/editor/src/*.rs` - All editor source files
- `crates/editor/Cargo.toml` - Dependencies
- `crates/editor/flake.nix` - Dev environment
- `.cargo/config.toml` - Build optimizations

### Dependencies (reference only)
- `crates/cube/` - Voxel structures, used as-is
- `crates/world/` - World generation, used as-is

### Root level
- `Cargo.toml` - Workspace member
- `justfile` - `just planet` and `just planet-release` tasks

## Success Criteria

1. Application launches with Bevy window titled "Crossworld Voxel Editor"
2. WorldCube renders with vertex colors
3. Orbit camera with smooth controls
4. Cursor gizmo shows placement position
5. Left-click places voxels, Shift+click removes
6. Brush mode supports .vox file brushes
7. Material palette shows all available materials
8. CSM and .vox file import/export works
9. Undo/redo with Ctrl+Z/Ctrl+Y
10. All keyboard shortcuts work as documented

## Development Environment

```bash
# Run editor
just planet

# Or directly
cargo run --bin planet

# Build release
just planet-release

# NixOS development
nix develop crates/editor
```

## Out of Scope

- Networking/multiplayer
- Complex animation system
- Plugin architecture
- Physics preview (separate in proto crate)
