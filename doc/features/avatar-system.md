# Avatar System

## Overview

Crossworld supports voxel-based 3D avatars with physics-based movement and user customization. Avatars use MagicaVoxel (.vox) format files and are rendered using procedural mesh generation in Rust compiled to WebAssembly.

**Key Features**:
- **Voxel Models**: Load from .vox files or use procedurally generated models
- **User Customization**: Color palette customization based on user npub hash
- **Physics Integration**: Character controller with collision detection and gravity
- **Nostr Integration**: Avatar state stored in Nostr events (kind 30317)
- **Multiple Sources**: Load from local assets, URLs, or Nostr profiles

## Architecture

The avatar system uses a **hybrid Rust/TypeScript architecture**:

### Rust (WASM) Responsibilities
- Load and parse voxel model data (.vox format)
- Generate optimized polygon mesh from voxels (greedy meshing)
- Apply per-user color customization based on npub hash
- Export geometry via `GeometryData` struct (vertices, indices, normals, colors)
- Physics simulation via character controller

### TypeScript Responsibilities
- Render avatars using Three.js
- Handle user input (mouse, keyboard, gamepad)
- Manage avatar movement and animations
- Synchronize state via Nostr events

### Data Flow

```
1. User selects avatar (local ID or URL)
   ↓
2. TypeScript: AvatarEngine.generate_avatar(userNpub)
   ↓
3. Rust: Load voxel data and customize palette
   ↓
4. Rust: Generate mesh with face culling
   ↓
5. Rust: Return GeometryData to TypeScript
   ↓
6. TypeScript: Create Three.js mesh and add to scene
   ↓
7. TypeScript: Initialize physics character controller
   ↓
8. User sees unique voxel character in world
```

## Avatar Configuration

### Nostr State Events (Kind 30317)

Avatar configuration is stored in Nostr state events:

**Required Tags**:
- `avatar_type` - Format type: `vox` (voxel) or `glb` (GLB/GLTF)

**Optional Tags**:
- `avatar_id` - Model identifier from `models.json` (e.g., `chr_army1`)
- `avatar_url` - Direct URL to avatar model file
- `avatar_data` - Procedural generation data (future)
- `avatar_mod` - Custom modifications (future)

**Loading Priority**:
1. **avatar_id**: Load from local assets (fastest, no network)
2. **avatar_url**: Load from custom URL
3. **avatar_data**: Generate from parameters (planned)
4. **avatar_mod**: Apply modifications (planned)

**Example Event**:
```json
{
  "kind": 30317,
  "tags": [
    ["d", "crossworld"],
    ["a", "30311:pubkey:world"],
    ["avatar_type", "vox"],
    ["avatar_id", "chr_army1"],
    ["position", "{\"x\":4,\"y\":0,\"z\":4}"],
    ["status", "active"]
  ],
  "content": ""
}
```

### Available Models

Models are defined in `public/assets/models.json`:

```json
{
  "vox": [
    ["Army Character 1", "chr_army1.vox"],
    ["Lady Character 1", "chr_lady1.vox"]
  ],
  "glb": [
    ["Default Avatar", "default.glb"]
  ]
}
```

To use a model, set `avatar_id` to the filename without extension.

## Model Format

### MagicaVoxel (.vox) Files

**Recommended Specifications**:
- Grid size: 32×32×32 voxels
- Character height: ~30 voxels (leaves 2 voxel margin)
- Keep character centered in the grid
- Use distinct colors for better visibility

**Finding Models**:
1. **OpenGameArt.org** - CC-BY 4.0 licensed models
2. **400 Free Voxel Models Pack** by Mike Judge
3. **Sketchfab** - Search for "MagicaVoxel"
4. **CGTrader** - Some free models available

**Creating Models**:
- Use MagicaVoxel (free software): https://ephtracy.github.io/
- Export as .vox file
- Place in `public/assets/models/vox/`
- Add entry to `models.json`

### Loading from Code

**TypeScript API**:
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

**Using GeometryData with Three.js**:
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

## Physics Integration

### Character Controller

Avatars use a kinematic character controller with physics-based collision detection:

**Rust Implementation** (`crates/physics/src/character_controller.rs`):
```rust
pub struct CharacterController {
    // Kinematic rigid body
    // Capsule collider (height, radius)
    // Ground detection via raycasting
    // Gravity and jump mechanics
}

pub struct CharacterControllerConfig {
    pub height: f32,              // Default: 1.8m
    pub radius: f32,              // Default: 0.3m
    pub step_height: f32,         // Default: 0.5m (climbable)
    pub max_slope_angle: f32,     // Default: 45°
    pub gravity: f32,             // Default: 9.8 m/s²
    pub jump_impulse: f32,        // Default: 5.0
    pub ground_check_distance: f32, // Default: 0.1m
}
```

**WASM JavaScript API**:
```javascript
// Character Management
createCharacter(x, y, z, height, radius): number
removeCharacter(characterId: number): void

// Movement
moveCharacter(characterId, velX, velZ, dt): void
jumpCharacter(characterId): void

// State Queries
getCharacterPosition(characterId): {x, y, z}
getCharacterVelocity(characterId): {x, y, z}
isCharacterGrounded(characterId): boolean
```

### Movement System

**Input Methods**:
- **Mouse**: Click-to-move, CTRL+click teleport, SHIFT+click run
- **Keyboard**: WASD movement, Space jump, Shift sprint
- **Gamepad**: Left stick movement, RT sprint, A jump

**Movement Styles** (Nostr event tag `move_style`):
- `walk` - Normal walking speed
- `run` - Double speed (SHIFT modifier)
- `teleport:fade` - Instant with fade effect
- `teleport:scale` - Instant with scale effect
- `teleport:spin` - Instant with spin effect

**Physics-Based Velocity**:
```typescript
// BaseAvatar movement logic
setTargetPosition(x: number, z: number)
  → Calculate direction to target
  → Apply acceleration (40 u/s²)
  → Apply damping (0.9)
  → Update position via physics
  → Smooth rotation (15 rad/s)
```

### Collision Detection

**Voxel Collision**:
- Octree-based collision mesh generation
- Only exposed voxel faces create colliders
- Compound colliders for efficiency
- See `VoxelColliderBuilder` in `crates/physics/src/voxel_collider.rs`

**Raycasting**:
```rust
PhysicsWorld::cast_ray(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    solid_only: bool,
) -> Option<(ColliderHandle, f32, Vec3, Vec3)>
```

## User Customization

### Color Palette System

Each user gets a unique color palette based on their npub:

**Process**:
1. User's npub is hashed to a number
2. Number determines hue shift (0-360°)
3. Base palette colors shifted in HSL color space
4. Results in deterministic but visually unique colors

**Implementation** (`crates/world/src/avatar/voxel_model.rs`):
```rust
pub struct VoxelPalette {
    colors: Vec<(u8, u8, u8)>, // RGB colors
}

impl VoxelPalette {
    // Shift all palette colors by hue offset
    pub fn shift_hue(&self, hue_shift: f32) -> Self;
}
```

## Rendering

### Mesh Generation

**Features**:
- Face culling (only render visible faces)
- Greedy meshing for efficiency
- Vertex colors from palette
- Normal generation for lighting
- Shadow support

**TypeScript Avatar Classes**:
- **BaseAvatar**: Shared movement and rotation logic
- **Avatar**: GLB model loader with skeletal animations
- **VoxelAvatar**: Voxel mesh renderer with flat shading

### Visual Style

Voxel avatars use:
- `MeshPhongMaterial` with vertex colors
- Flat shading for blocky voxel aesthetic
- Double-sided rendering
- Cast and receive shadows

## Position Updates (Kind 1317)

Movement is synchronized via Nostr update events:

```json
{
  "kind": 1317,
  "tags": [
    ["a", "30317:pubkey:crossworld-avatar-..."],
    ["update_type", "position"],
    ["position", "{\"x\":5.5,\"y\":0,\"z\":3.2}"],
    ["move_style", "run"]
  ],
  "content": ""
}
```

## Performance

**Metrics**:
- First generation: ~1-5ms (mesh + caching)
- Cached retrieval: <1ms
- Memory: ~30KB per cached avatar
- Rendering: 1000-2000 triangles (flat shaded)

**Optimization**:
- Avatar meshes cached by user npub
- Greedy meshing reduces triangle count 50-80%
- Face culling eliminates interior faces
- LOD system planned for distant avatars

## Future Enhancements

### Planned Features

1. **Skeletal Animation**
   - Load skeleton from glTF
   - Bind voxel mesh to bones
   - Idle, walk, run, jump animations

2. **Real VOX Loading**
   - Parse MagicaVoxel .vox files at runtime
   - Support custom user-uploaded models
   - Use `dot_vox` crate

3. **Enhanced Customization**
   - Full palette editing (not just hue shift)
   - Accessories and equipment
   - Material overrides

4. **Advanced Physics**
   - Ragdoll physics
   - Vehicle support
   - Swimming/flying

5. **Procedural Generation**
   - Generate avatars from parameters
   - Body type, proportions, style
   - Deterministic from seed

## Implementation Files

### Rust (crates/)
- `world/src/avatar/` - Avatar management and mesh generation
  - `mod.rs` - Module exports
  - `voxel_model.rs` - Voxel data and palette
  - `mesher.rs` - Mesh generation
  - `manager.rs` - Avatar caching
- `physics/src/character_controller.rs` - Physics controller
- `physics/src/wasm.rs` - WASM bindings for physics

### TypeScript (packages/app/src/)
- `renderer/voxel-avatar.ts` - VoxelAvatar class
- `renderer/avatar.ts` - GLB Avatar class
- `renderer/base-avatar.ts` - Shared BaseAvatar logic
- `renderer/scene.ts` - Scene management and avatar integration
- `services/avatar-state.ts` - Nostr state event handling
- `components/SelectAvatar.tsx` - UI for avatar selection
- `components/WorldCanvas.tsx` - Avatar initialization
- `utils/voxLoader.ts` - VOX file loading utilities

## Related Documentation

- [physics.md](../architecture/physics.md) - Physics system overview
- [voxel-system.md](../architecture/voxel-system.md) - Voxel engine details
- [nostr-integration.md](nostr-integration.md) - Nostr protocol integration
