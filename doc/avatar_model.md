# Avatar Model Format

This document describes the avatar model format used in Crossworld, including event fields and handling logic.

## Overview

Avatars in Crossworld are configured through Nostr state events (kind 30317) and can be loaded from multiple sources with a defined priority order.

## Event Fields

Avatar configuration is stored in state events using the following tags:

### Required Fields

- `avatar_type` (string): Avatar format type
  - `voxel` - Voxel-based 3D model
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

### Voxel Models (avatar_type: 'voxel')

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
    ["avatar_type", "voxel"],
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
- Voxel avatars: Fall back to procedurally generated simple voxel avatar
- GLB avatars: Fall back to default `man.glb` model

### Color Handling

Avatars use their original palette colors by default. Color customization features are disabled in the current version.

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
