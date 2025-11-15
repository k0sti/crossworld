# Voxel System Architecture

## Overview

The Crossworld voxel system is based on an octree structure that efficiently stores and renders voxel data. The system uses the Cube Script Model (CSM) format for serialization and supports hierarchical subdivision for detail levels.

**Key Components**:
- **Octree Engine** (`crates/cube/`) - Core voxel data structure
- **CSM Format** - Text-based serialization format
- **Mesh Generation** - Optimized geometry creation from voxels
- **Color Mapping** - HSV-based color system with palette support

## Octree Structure

### Octant Indexing

Octants are labeled a-h representing the 8 children of a cube:

```
a=000 (x-,y-,z-)  e=100 (x+,y-,z-)
b=001 (x-,y-,z+)  f=101 (x+,y-,z+)
c=010 (x-,y+,z-)  g=110 (x+,y+,z-)
d=011 (x-,y+,z+)  h=111 (x+,y+,z+)
```

Each cube can be:
- **Empty** - No voxels
- **Solid** - Single color value
- **Subdivided** - 8 child cubes (one octree level deeper)

### Depth and Resolution

The octree depth determines the maximum resolution:
- Depth 0: 1×1×1 (single cube)
- Depth 1: 2×2×2 (8 cubes)
- Depth 2: 4×4×4 (64 cubes)
- Depth 3: 8×8×8 (512 cubes)
- Depth N: 2^N × 2^N × 2^N cubes

## Cube Script Model (CSM) Format

### Grammar

```
Model = Epoch+
Epoch = Statement+ ('|' Epoch)?
Statement = '>' Octant+ Cube
Cube = Value
     | '[' Cube{8} ']'  // Exactly 8 children
     | '<' Path?         // Reference from previous epoch (root if no path)
     | Transform Cube
Value = Integer
Path = Octant+
Octant = [a-h]
Transform = '/' Axis+   // Mirror along axis(es), order doesn't matter
Axis = 'x' | 'y' | 'z'
```

### Basic Syntax

```csm
# Comments start with #

# Solid voxel at path 'a'
>a 100

# Array of 8 child voxels (octants a-h)
>b [1 2 3 4 5 6 7 8]

# Nested structure (path 'cd' = c -> d)
>cd [10 11 12 13 14 15 16 17]
```

### Color Values

- `0` = Empty (no voxel)
- `1-360` = Maps to HSV hue (0=red, 120=green, 240=blue, etc.)
- Negative values = Red color
- Custom palettes supported via `set_model_palette()`

## CSM Examples

### Simple Examples

**Single Voxel**:
```csm
>a 100
```

**Array of 8 Voxels**:
```csm
>a [1 2 3 4 5 6 7 8]
```

**Nested Structure**:
```csm
>a [1 2 3 4 5 6 7 8]
>aa [10 11 12 13 14 15 16 17]
>aaa 100
```

### Epochs and References

**Copy from Previous Epoch**:
```csm
>a [1 2 3 4 5 6 7 8]
| >b <a
```
First epoch creates structure at 'a', second epoch copies it to 'b'.

**Mirror Transform**:
```csm
>a [1 2 3 4 5 6 7 8]
| >b /x <a
```
Mirror the structure along the X axis.

**Complex Transformations**:
```csm
# First epoch - build base structure
>a [1 2 3 4 5 6 7 8]
>aa [10 11 12 13 14 15 16 17]
>ab [20 21 22 23 24 25 26 27]

# Second epoch - create variations
| >b <a           # Copy entire structure
  >c /x <a        # X-axis mirror
  >d /y <a        # Y-axis mirror
  >e /z <a        # Z-axis mirror
  >f /xy <a       # XY mirror
```

### Humanoid Character Example

```csm
# Head (top octant 'd')
>d [100 100 100 100 100 100 100 100]
>dd [150 150 150 150 150 150 150 150]

# Body/Torso (octant 'c')
>c [80 80 80 80 80 80 80 80]
>cd [90 90 90 90 90 90 90 90]

# Arms (octants 'cf' and 'cg')
>cf [70 70 70 70 0 0 0 0]
>cg [70 70 70 70 0 0 0 0]

# Legs (octants 'a' and 'b')
>a [60 60 0 0 0 0 0 0]
>b [60 60 0 0 0 0 0 0]
```

## Save and Load

### TypeScript API

**Saving a Model**:
```typescript
import { saveWorldToCSM, autoSaveWorld } from '@/utils/csmSaver';

// Manual save (downloads file in browser)
saveWorldToCSM('world', 'my-world.csm');

// Auto-save with debouncing (waits 1 second after last modification)
autoSaveWorld('world', 1000);
```

**Loading a Model**:
```typescript
import { loadWorldFromCSM } from '@/utils/csmSaver';

const csmText = `
>a [1 2 3 4 5 6 7 8]
>b [10 11 12 13 14 15 16 17]
`;

loadWorldFromCSM(csmText, 'world', 4); // depth=4 for 16x16x16
```

**Getting CSM Text**:
```typescript
import { getModelCSM } from '@/utils/csmSaver';

const csmText = getModelCSM('world');
console.log(csmText);
```

### WASM Bindings

The CSM functionality is implemented in Rust and exposed via WASM:

```rust
// In crates/cube/src/wasm.rs

#[wasm_bindgen]
pub fn serialize_model_to_csm(model_id: &str) -> JsValue;

#[wasm_bindgen]
pub fn load_model_from_csm(model_id: &str, csm_text: &str, max_depth: usize) -> JsValue;
```

### File Locations

By convention, world files are saved to:
- `assets/worlds/default.csm` - Default world file
- `assets/worlds/*.csm` - Custom world files

## Implementation Details

### Cube Crate Structure

The `crates/cube/` crate contains:
- `src/octree.rs` - Core octree data structures
- `src/parser.rs` - CSM format parsing
- `src/serializer.rs` - CSM format serialization
- `src/face_builder.rs` - Mesh generation with face culling
- `src/raycast/` - Ray-octree intersection
- `src/mesh.rs` - Color mapping traits
- `src/vox_loader.rs` - MagicaVoxel format support

### Mesh Generation

The mesh generation system:
- Uses greedy meshing for efficiency
- Culls faces between adjacent solid voxels
- Generates vertex positions, normals, and colors
- Exports `GeometryData` for Three.js rendering

### Color Mapping

Two color mapping strategies:
- **HsvColorMapper**: Maps integers 1-360 to HSV hue values
- **PaletteMapper**: Uses custom color palettes

## Related Documentation

- [raycast.md](raycast.md) - Ray-octree intersection system
- [rendering.md](rendering.md) - Mesh rendering pipeline
- `crates/cube/README.md` - Cube crate implementation details
