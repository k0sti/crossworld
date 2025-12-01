# Crossworld Voxel Editor - Quick Start Guide

## Building and Running

The editor requires Linux system dependencies (ALSA, udev, Vulkan, X11/Wayland). Use the Nix development shell:

```bash
# From project root
cd crates/editor
nix develop

# This will enter a shell with all dependencies
# Then return to project root and run:
cd ../..
just editor
```

Or manually:
```bash
cargo run --bin editor
```

## What You'll See

When the editor launches, you'll see:

- **Center**: 3D viewport with procedurally generated voxel terrain
- **Right side**: Help panel with all keyboard controls
- **Bottom**: Status bar showing:
  - Current file name and save status (* = unsaved changes)
  - Cursor position (world coordinates)
  - Cursor size (1-16 voxels)
  - Current mode (Place/Remove)
  - Selected material (0-9)

## Basic Controls

### Camera Navigation
- **Right-click + drag**: Rotate camera around scene
- **Scroll wheel**: Zoom in/out
- **F**: Frame entire scene in view
- **Home**: Reset camera to default position
- **Numpad 1/3/7**: Front/Side/Top orthographic views

### Voxel Editing
- **Left-click**: Place voxel(s) with selected material
- **Shift + Left-click**: Remove voxel(s)
- **Delete**: Remove voxel(s) at cursor
- **Tab**: Toggle between Place mode (green cursor) and Remove mode (red cursor)
- **Hold left-click**: Continuous paint/removal as you move cursor
- **[ / ]**: Decrease/Increase cursor size (1 to 16)
- **Shift + Scroll wheel**: Adjust cursor size

### Material Selection
- **0-9 keys**: Select material 0-9
  - 0 = Air (transparent)
  - 1-9 = Various terrain materials

### File Operations
- **Ctrl+N**: New scene (clears current work)
- **Ctrl+O**: Open CSM voxel file
- **Ctrl+S**: Save to current file
- **Ctrl+Shift+S**: Save As (choose new filename)

## Workflow Example

1. **Start editing**: The editor loads with procedural terrain
2. **Select a material**: Press `1` for grass, `2` for stone, etc.
3. **Toggle to Place mode**: Press `Tab` until cursor is green
4. **Place voxels**: Left-click where you want to add blocks
5. **Increase cursor size**: Press `]` multiple times to place larger volumes
6. **Toggle to Remove mode**: Press `Tab` until cursor is red
7. **Remove voxels**: Left-click or Shift+Left-click to carve out shapes
8. **Save your work**: Press `Ctrl+S` to save as a CSM file
9. **Load it later**: Press `Ctrl+O` to reopen saved files

## CSM File Format

CSM (CubeScript Model) is a human-readable text format for voxel octrees:

```csm
s[
  o[s1 s2 s0 s0 s0 s0 s0 s0]  # octree node (8 children)
  s5                           # solid voxel (material 5)
]
```

Files saved from the editor can be:
- Opened in the web application (`packages/app`)
- Shared with other users
- Version-controlled with git
- Hand-edited in a text editor

## Current Implementation Status

### ✅ Completed Features
- **Phase 1-4**: Camera, voxel scene, raycasting, cursor (97/97 tasks)
- **Phase 6**: Voxel editing with placement/removal (partial, 15/20 tasks)
- **Phase 10**: File I/O with CSM save/load (partial, 8/17 tasks)
- **UI**: Status bar and help panel (partial Phase 7)

### 🚧 Not Yet Implemented
- **Undo/Redo**: No Ctrl+Z/Ctrl+Y support yet
- **Brush System**: Can't load/stamp .vox files as brushes
- **Copy/Paste**: No Ctrl+C to copy voxel regions
- **Menu Bar**: No File/Edit/View menus
- **Material Palette UI**: Must use 0-9 keys, no visual palette yet
- **Inspector Panel**: No scene statistics display
- **.vox Import/Export**: Only CSM format supported currently

## Known Limitations

1. **Large cursor sizes (8+)** may cause performance issues on complex scenes
2. **No confirmation dialogs** for discarding unsaved changes (always confirms)
3. **No visual feedback** for file operations (check console logs)
4. **Procedural terrain regenerates** on New Scene (can't clear to empty scene)
5. **Build requires Nix shell** on Linux (system dependencies)

## Next Steps for Development

Priority features to add next:

1. **Undo/Redo System** (Phase 17) - Most requested feature
2. **Material Palette UI** (Phase 8) - Visual material selection
3. **Menu Bar** (Phase 7) - File/Edit/View menus with egui
4. **Brush System** (Phase 5) - Load .vox files, copy/paste, scaling
5. **.vox Import/Export** (Phase 10) - Interop with MagicaVoxel

## Troubleshooting

### Build fails with "alsa-sys not found"
You need to enter the Nix dev shell:
```bash
cd crates/editor
nix develop
```

### Editor window is blank/black
Check console for errors. The voxel scene generates on first frame. Wait 1-2 seconds.

### Cursor not appearing
Move your mouse over the terrain. The cursor only appears when raycasting hits voxels.

### Can't save file
Check console logs for errors. Ensure you have write permissions in the directory.

## Help and Feedback

- GitHub Issues: https://github.com/crossworld-project/issues
- Documentation: See `CLAUDE.md` in project root
- Implementation tasks: See `openspec/changes/create-bevy-voxel-editor/tasks.md`
