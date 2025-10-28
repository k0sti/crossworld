## CSM Save/Load Feature

This document describes how to save and load voxel models using the CSM (Cube Script Model) format.

### Overview

The CSM format is a text-based format for describing octree-based voxel models. It supports:
- Hierarchical octree structures
- Compact array notation for uniform regions
- Human-readable/editable format

### Usage

#### Saving a Model

```typescript
import { saveWorldToCSM, autoSaveWorld } from '@/utils/csmSaver';

// Manual save (downloads file in browser)
saveWorldToCSM('world', 'my-world.csm');

// Auto-save with debouncing (waits 1 second after last modification)
// Call this after each draw/modification operation
autoSaveWorld('world', 1000);
```

#### Loading a Model

```typescript
import { loadWorldFromCSM } from '@/utils/csmSaver';

// Load CSM text into a model
const csmText = `
>a [1 2 3 4 5 6 7 8]
>b [10 11 12 13 14 15 16 17]
`;

loadWorldFromCSM(csmText, 'world', 4); // depth=4 for 16x16x16
```

#### Getting CSM Text

```typescript
import { getModelCSM } from '@/utils/csmSaver';

// Get CSM text without downloading
const csmText = getModelCSM('world');
console.log(csmText);
```

### File Location

By convention, world files are saved to:
- `assets/worlds/default.csm` - Default world file
- `assets/worlds/*.csm` - Custom world files

### CSM Format Syntax

#### Basic Syntax

```csm
# Comments start with #

# Solid voxel at path 'a'
>a 100

# Array of 8 child voxels (octants a-h)
>b [1 2 3 4 5 6 7 8]

# Nested structure
>cd [10 11 12 13 14 15 16 17]
```

#### Octant Layout

Octants are labeled a-h representing the 8 children of a cube:

```
a=000 (x-,y-,z-)  e=100 (x+,y-,z-)
b=001 (x-,y-,z+)  f=101 (x+,y-,z+)
c=010 (x-,y+,z-)  g=110 (x+,y+,z-)
d=011 (x-,y+,z+)  h=111 (x+,y+,z+)
```

#### Example: Simple Humanoid

```csm
# Head (4x4x4 region at top)
>d [100 100 100 100 100 100 100 100]

# Body
>c [80 80 80 80 80 80 80 80]

# Arms
>cf [70 70 70 70 0 0 0 0]
>cg [70 70 70 70 0 0 0 0]

# Legs
>a [60 60 0 0 0 0 0 0]
>b [60 60 0 0 0 0 0 0]
```

### Integration with WASM

The CSM save/load functionality uses WASM bindings from the `crossworld-cube` crate:

```rust
// In crates/cube/src/wasm.rs

#[wasm_bindgen]
pub fn serialize_model_to_csm(model_id: &str) -> JsValue;

#[wasm_bindgen]
pub fn load_model_from_csm(model_id: &str, csm_text: &str, max_depth: usize) -> JsValue;
```

### Color Values

- `0` = Empty (no voxel)
- `1-360` = Maps to HSV hue (when using HsvColorMapper)
- Custom palette can be set with `set_model_palette()`

### See Also

- `doc/cube-script-model.md` - Complete CSM specification
- `doc/csm-examples.md` - More examples
- `packages/app/src/utils/csmSaver.ts` - TypeScript utilities
