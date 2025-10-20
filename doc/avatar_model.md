# Avatar Model Format

This document describes the avatar model format used in Crossworld, including event fields and handling logic.

## Overview

Avatars in Crossworld are configured through Nostr state events (kind 30317) and can be loaded from multiple sources with a defined priority order.

## State Events (Kind 30317)

Avatar configuration is stored in state events using the following tags:

### Required Fields

- `avatar_type` (string): Avatar format type
  - `vox` - Voxel-based 3D model (.vox files)
  - `glb` - GLB/GLTF 3D model

### Optional Fields

- `avatar_id` (string): Predefined model identifier (e.g., 'boy', 'girl', 'man')
  - Used to load existing models from local assets
  - Takes priority over other sources

- `avatar_url` (string): Direct URL to avatar model file
  - Load avatar from HTTP(S) URL
  - Supports `.vox` files for voxel avatars
  - Supports `.glb` files for GLB avatars

- `avatar_data` (string): Procedural generation data *(not yet implemented)*
  - Preferred method for reproducible avatar generation
  - Will contain serialized parameters for procedural generation

- `avatar_mod` (string): Custom modifications *(not yet implemented)*
  - Applied after avatar is loaded
  - Format to be defined

## Loading Priority

Avatars are loaded in the following priority order:

1. **avatar_id**: Load from predefined local assets
   - Fastest loading
   - Reliable, no network dependency
   - Example: `boy` → `chr_peasant_guy_blackhair.vox`

2. **avatar_url**: Load from custom URL
   - Allows user-provided models
   - Network dependent
   - Supports both `.vox` and `.glb` formats

3. **avatar_data**: Generate from data *(planned)*
   - Reproducible procedural generation
   - No assets required
   - Will enable infinite customization

4. **avatar_mod**: Apply modifications *(planned)*
   - Post-processing of loaded avatar
   - Accessories, colors, etc.

## Predefined Models

### Vox Models (avatar_type: 'vox')

| avatar_id | File                              |
|-----------|-----------------------------------|
| `boy`     | chr_peasant_guy_blackhair.vox     |
| `girl`    | chr_peasant_girl_orangehair.vox   |

### GLB Models (avatar_type: 'glb')

| avatar_id | File      |
|-----------|-----------|
| `man`     | man.glb   |

## Example State Event

```json
{
  "kind": 30317,
  "tags": [
    ["d", "crossworld"],
    ["a", "30311:pubkey:world"],
    ["expiration", "1234567890"],
    ["avatar_type", "vox"],
    ["avatar_id", "boy"],
    ["client", "Crossworld Web"],
    ["position", "{\"x\":4,\"y\":0,\"z\":4}"],
    ["status", "active"],
    ["voice", "disconnected"],
    ["mic", "disabled"]
  ],
  "content": ""
}
```

## Handling Logic

### Client Implementation

When loading an avatar, clients should:

1. Check `avatar_type` to determine format
2. Attempt to load in priority order:
   - If `avatar_id` is set and recognized, load from local assets
   - Else if `avatar_url` is set, load from URL
   - Else if `avatar_data` is set, generate from data
   - Else fallback to default/generated avatar
3. Apply `avatar_mod` if present (when implemented)

### Fallback Behavior

If loading fails at any step:
- Vox avatars: Fall back to procedurally generated simple voxel avatar
- GLB avatars: Fall back to default `man.glb` model

### Color Handling

Avatars use their original palette colors by default. Color customization features are disabled in the current version.

## Update Events (Kind 1317)

Position updates and movement are sent via update events. These events include:

### Movement Style

The `move_style` tag indicates how the avatar should move to the new position:

- `walk` - Walk animation at normal speed (default)
- `run` - Run animation at double speed (SHIFT+click)
- `teleport:fade` - Teleport with fade animation (CTRL+click)
- `teleport:scale` - Teleport with scale animation
- `teleport:spin` - Teleport with spin animation
- `teleport:slide` - Teleport with slide animation
- `teleport:burst` - Teleport with burst animation

### Movement Controls

- **Click**: Walk to target position
- **SHIFT+Click**: Run to target position
- **CTRL+Click**: Teleport to target position with selected animation

### Example Update Event

```json
{
  "kind": 1317,
  "tags": [
    ["a", "30317:pubkey:crossworld-avatar-..."],
    ["a", "30311:pubkey:crossworld-dev"],
    ["update_type", "position"],
    ["expiration", "1234567890"],
    ["position", "{\"x\":5.5,\"y\":0,\"z\":3.2}"],
    ["move_style", "run"]
  ],
  "content": ""
}
```

### Remote Avatar Animation

When clients receive position updates, they animate remote avatars based on `move_style`:

- **walk/run**: Smooth animation from last known position to new position
- **teleport:X**: Instant position change with visual effect X

## Future Enhancements

### avatar_data Format (Planned)

Will support procedural generation with parameters such as:
- Body type (slim, normal, bulky)
- Proportions (head size, limb length)
- Category (humanoid, animal, geometric, etc.)
- Seed for reproducibility

### avatar_mod Format (Planned)

Will support modifications such as:
- Accessories and equipment
- Material overrides
- Scale/proportion adjustments
- Animation overrides

## Related Files

- `packages/app/src/services/avatar-state.ts` - State event handling
- `packages/app/src/components/SelectAvatar.tsx` - UI for avatar selection
- `packages/app/src/components/WorldCanvas.tsx` - Avatar loading logic
