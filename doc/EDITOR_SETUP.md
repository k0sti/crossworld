# Block Editor Setup

This document describes the block editor implementation for Crossworld.

## Project Structure

### Packages

- **`packages/common`** - Shared functionality between apps
  - TopBar with Nostr login
  - ResponsivePanel component
  - ProfileButton with guest/extension/Amber login
  - Login settings service
  - Config constants

- **`packages/editor`** - Block editor package
  - CubeEditorView - 3D voxel editor with Three.js
  - PaletteSelector - Color palette selector panel
  - HSV palette generator
  - DawnBringer palettes (16 and 32 colors)

- **`packages/app`** - Main Crossworld application
  - World view (existing functionality)
  - Now uses common package for shared UI

## Routes

- **`/`** - Original Crossworld app (world view)
- **`/editor`** - Block editor view

## Key Features

### Block Editor

1. **3D Cube Editor View**
   - Three.js-based voxel editor
   - Click to place/remove voxels
   - Mouse drag to rotate camera
   - 16x16x16 grid with 0.1 unit voxel size

2. **Palette Selector**
   - ResponsivePanel-based UI
   - Two palette sources:
     - HSV: Generated palette with configurable sizes (8, 16, 32, 64)
     - DawnBringer: Classic pixel art palettes (16, 32 colors)
   - Color grid with visual selection
   - Selected color display with hex code

3. **Editor Controls**
   - Toggle palette
   - Clear all voxels
   - Save (placeholder)
   - Export (placeholder)

### Shared Components (Common Package)

- **TopBar**: Unified top bar with:
  - Nostr profile button with login modal
  - Network settings button
  - Info button
  - Supports guest accounts, browser extensions, and Amber (Android)

- **ResponsivePanel**: Flexible panel component
  - Auto-switches to fullscreen on overflow
  - Configurable positioning
  - Click-outside and ESC key to close
  - Title and action buttons support

## Usage

### Development

```bash
# Install dependencies
bun install

# Run development server
cd packages/app
bun run dev
```

### Navigation

- Visit `/` for the world view
- Visit `/editor` for the block editor

### Editor Controls

1. **Place Voxels**: Click on existing voxels to place adjacent voxels, or click empty space to place at origin
2. **Remove Voxels**: Click existing voxels to remove them
3. **Rotate View**: Click and drag to rotate the camera
4. **Select Color**: Click the color button in the left sidebar to open palette selector

### Palette Selection

1. Choose source: HSV or DawnBringer
2. Select palette size from dropdown
3. Click a color in the grid to select it
4. Selected color appears in the bottom display

## WASM Interface

The editor is designed to integrate with the existing WASM cube module:

- **`packages/wasm-cube`**: CSM (Crossworld Scene Model) parser
  - `parse_csm_to_mesh`: Parse CSM code to mesh data
  - `validate_csm`: Validate CSM code
  - Returns vertices, indices, normals, and colors

Future work will connect the editor to generate CSM code from voxel data.

## Architecture

### Separation of Concerns

- **Common**: Shared UI components and services
- **Editor**: Editor-specific functionality
- **App**: Application composition and routing

This separation allows:
- Clean code organization
- Reusable components
- Independent development
- Easy maintenance

### Nostr Integration

Both apps share Nostr functionality:
- Login with browser extensions (nos2x, Alby, etc.)
- Guest accounts (persistent in localStorage)
- Amber support (Android)
- Relay configuration
- Profile management

## Future Work

1. **CSM Export**: Generate CSM code from voxel data
2. **File I/O**: Save/load editor projects
3. **Undo/Redo**: Command pattern for edit history
4. **Selection Tools**: Multi-voxel selection and editing
5. **Layer System**: Organize voxels in layers
6. **Preview Mode**: Real-time preview with different renderers
7. **Nostr Publishing**: Publish creations to Nostr
