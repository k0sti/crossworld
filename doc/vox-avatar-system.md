# MagicaVoxel Avatar System

This document describes how to create, obtain, and use MagicaVoxel (.vox) 3D avatar models in Crossworld.

## Overview

Crossworld supports loading custom avatar models from MagicaVoxel .vox files. These can be:
- Loaded from URLs
- Uploaded from local files
- Specified in Nostr profile metadata

The system automatically:
- Parses .vox file format
- Generates Three.js-compatible geometry
- Applies per-user color customization based on their npub
- Positions avatars correctly (feet at ground level, centered horizontally)

## Finding .vox Character Models

### Free Model Sources

1. **OpenGameArt.org** (Downloaded - see `assets/models/vox/`)
   - License: CC-BY 4.0
   - Models: `chr_peasant_girl_orangehair.vox`, `chr_peasant_guy_blackhair.vox`
   - Dimensions: 16×16×32 voxels (close to our 30-voxel height target)
   - URL: https://opengameart.org/content/voxel-character-models-vox

2. **400 Free Voxel Models Pack**
   - By Mike Judge
   - Categories: characters, vehicles, buildings, props
   - All created with MagicaVoxel
   - Includes .VOX files for editing
   - URL: https://www.megavoxels.com/2019/08/free-voxel-models-for-creating-voxel-art-and-games.html

3. **Sketchfab**
   - Search for "MagicaVoxel" models
   - Many free and CC-licensed models
   - URL: https://sketchfab.com/tags/magicavoxel

4. **CGTrader**
   - Some free MagicaVoxel models
   - URL: https://www.cgtrader.com/3d-models/magicavoxel

### Creating Your Own Models

Use MagicaVoxel (free software) to create custom characters:

1. **Download MagicaVoxel**
   - Website: https://ephtracy.github.io/
   - Available for Windows, macOS, Linux

2. **Recommended Dimensions**
   - Grid size: 32×32×32 voxels
   - Character height: ~30 voxels (leaves 2 voxels margin)
   - Keep character centered in the grid

3. **Export Format**
   - Save as .vox file
   - The system will automatically handle the model

## Using .vox Models in Code

### TypeScript/JavaScript

```typescript
import { loadVoxFromUrl, loadVoxFromFile, loadVoxFromNostrProfile } from './utils/voxLoader'

// Load from URL
const geometryData = await loadVoxFromUrl(
  'https://example.com/avatar.vox',
  userNpub  // Optional: applies user-specific colors
)

// Load from File input
const file = fileInput.files[0]
const geometryData = await loadVoxFromFile(file, userNpub)

// Load from Nostr profile
const geometryData = await loadVoxFromNostrProfile(profileEvent, userNpub)
```

### Using GeometryData with Three.js

```typescript
const geometry = new THREE.BufferGeometry()
geometry.setAttribute('position', new THREE.Float32BufferAttribute(geometryData.vertices, 3))
geometry.setAttribute('normal', new THREE.Float32BufferAttribute(geometryData.normals, 3))
geometry.setAttribute('color', new THREE.Float32BufferAttribute(geometryData.colors, 3))
geometry.setIndex(new THREE.Uint32BufferAttribute(geometryData.indices, 1))

const material = new THREE.MeshStandardMaterial({
  vertexColors: true,
  side: THREE.DoubleSide
})

const mesh = new THREE.Mesh(geometry, material)
```

## Nostr Profile Integration

To specify a .vox avatar in your Nostr profile:

### 1. Add a `vox_avatar` Tag

In your kind 0 (profile metadata) event, include:

```json
{
  "kind": 0,
  "content": "{\"name\":\"Alice\",\"about\":\"...\"}",
  "tags": [
    ["vox_avatar", "https://example.com/myavatar.vox"]
  ]
}
```

### 2. Host Your .vox File

- Upload to any public HTTPS URL
- Ensure CORS is enabled for browser access
- Consider using:
  - GitHub raw content URLs
  - Your own web server
  - Nostr blossom servers (future support)

### 3. Alternative: NIP-94 File Metadata

For decentralized file hosting, you can use NIP-94:

```json
{
  "kind": 0,
  "tags": [
    ["vox_avatar", "<sha256-hash>"],
    ["alt", "Avatar model"]
  ]
}
```

Then publish a kind 1063 (file metadata) event with the .vox file details.

## Technical Implementation

### Rust/WASM Layer

The core .vox parsing happens in Rust:

- **File**: `crates/world/src/avatar/vox_loader.rs`
- **Library**: `dot_vox` v5.1
- **Function**: `load_vox_from_bytes(bytes: &[u8], user_npub: Option<String>) -> Result<GeometryData>`

Features:
- Parses RIFF-style .vox file format
- Extracts voxel positions and color palette
- Converts to internal VoxelModel representation
- Applies user-specific color shifts via HSL manipulation
- Generates optimized mesh with culled faces

### TypeScript Layer

The TypeScript utilities handle:
- Fetching .vox files from URLs
- Reading from File objects
- Parsing Nostr profile events
- Calling WASM functions
- Returning Three.js-compatible geometry data

**File**: `packages/app/src/utils/voxLoader.ts`

## Model Specifications

### Voxel Grid

- **Size**: Configurable (recommended 32×32×32)
- **Voxel Scale**: 0.1 units per voxel
- **World Size**: 3.2 × 3.2 × 3.2 units (for 32³ grid)

### Character Dimensions

Example for 30-voxel height character:
- **Height**: 30 voxels = 3.0 units
- **Width**: ~16 voxels = 1.6 units
- **Depth**: ~16 voxels = 1.6 units

### Positioning

The system automatically positions avatars:
- **Feet**: At y = 0 (ground level)
- **Horizontal**: Centered on avatar's position
- **Rotation**: Follows player orientation

## Color Customization

The system applies per-user color variations:

1. **Hash Generation**: Creates unique hash from user's npub
2. **Hue Shift**: Rotates palette colors in HSL color space
3. **Preservation**: Maintains relative color relationships
4. **Consistency**: Same user always gets same colors

This ensures:
- Visual distinction between users
- Consistent appearance for each user
- Preservation of original model design

## Example Models

Two example models are included in `assets/models/vox/`:

1. **chr_peasant_girl_orangehair.vox** (23.8 KB)
   - Female character model
   - ~32 voxels height
   - Multiple colors (hair, skin, clothing)

2. **chr_peasant_guy_blackhair.vox** (23.7 KB)
   - Male character model
   - ~32 voxels height
   - Multiple colors (hair, skin, clothing)

## Testing

To test with example models:

```typescript
// Test with local file
const geometryData = await loadVoxFromUrl(
  '/assets/models/vox/chr_peasant_girl_orangehair.vox'
)

// Test with user customization
const geometryData = await loadVoxFromUrl(
  '/assets/models/vox/chr_peasant_guy_blackhair.vox',
  'npub1...'  // User's npub for color customization
)
```

## Future Enhancements

Potential improvements:

1. **Animation Support**
   - Parse .vox scene graph
   - Support multiple models (animation frames)
   - Implement keyframe interpolation

2. **Blossom Integration**
   - Upload .vox files to Nostr blossom servers
   - Reference via NIP-94 file metadata
   - Truly decentralized avatar hosting

3. **In-Game Editor**
   - Simple voxel editor in-browser
   - Export to .vox format
   - Real-time preview

4. **Model Marketplace**
   - Browse community-created avatars
   - Filter by style, size, complexity
   - One-click apply to profile

5. **LOD (Level of Detail)**
   - Generate multiple resolution versions
   - Switch based on distance/performance
   - Maintain visual fidelity at distance

## Resources

- **MagicaVoxel**: https://ephtracy.github.io/
- **File Format Spec**: https://github.com/ephtracy/voxel-model/blob/master/MagicaVoxel-file-format-vox.txt
- **dot_vox Crate**: https://crates.io/crates/dot_vox
- **Free Models**: https://opengameart.org/content/voxel-character-models-vox
- **Nostr NIPs**: https://github.com/nostr-protocol/nips
