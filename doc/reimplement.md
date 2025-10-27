# Reimplement Plan: World Size & Depth Refactor

## Problem Statement

Ground voxel drawing and rendering had issues. The system needed refactoring to:
- Define world size/depth constants in a single source of truth
- Treat the world as a single octree with configurable depth
- Fix coordinate conversion so voxels at world depth are size 1
- Add debug panel showing constants and cursor position/depth
- Make voxel drawing generic without depth-specific validation

## High-Level Architecture Goals

### 1. Constants System
**Location**: TypeScript side (`packages/common/src/constants.ts`)

**Key Constants**:
```typescript
WORLD_DEPTH = 3          // World is 8×8×8 voxels (2^3)
SUBVOXEL_DEPTH = 3       // Each voxel can be subdivided 3 levels
TOTAL_DEPTH = 6          // Maximum octree depth (2^6 = 64 grid)
WORLD_SIZE_VOXELS = 8    // World size in voxels
MAX_GRID_SIZE = 64       // Maximum grid resolution
WORLD_SIZE_UNITS = 8     // World size in 3D units
```

**Rationale**:
- Single source of truth for all depth/size calculations
- TypeScript-first: easier to iterate, no WASM rebuild needed
- Passed to WASM on initialization
- Shared across app and editor packages

### 2. Generic Octree Operations
**Goal**: Make voxel operations depth-agnostic

**Before**: World-specific validation (checking bounds at depth 4, hardcoded offsets)
**After**: Generic octree operations that work at any depth

**Key Changes**:
- Accept coordinates at max depth (0..2^TOTAL_DEPTH)
- Scale coordinates internally: `pos = coord / 2^(max_depth - target_depth)`
- No world-specific boundary checks in cube layer
- World semantics handled at TypeScript layer

### 3. Coordinate Systems
**Three coordinate spaces**:

1. **World Space** (3D scene):
   - Used by Three.js camera, rendering
   - Range: [0, WORLD_SIZE_UNITS] (0-8)

2. **Voxel Space** (logical):
   - Used by edit cursor, user operations
   - Range: [0, WORLD_SIZE_VOXELS) at world depth (0-7)

3. **Max Grid Space** (internal octree):
   - Used by octree implementation
   - Range: [0, MAX_GRID_SIZE) at max depth (0-63)

**Conversions**:
- Voxel → Max Grid: `coord * (MAX_GRID_SIZE / WORLD_SIZE_VOXELS)` = `coord * 8`
- Max Grid → Voxel: `coord / (MAX_GRID_SIZE / WORLD_SIZE_VOXELS)` = `coord / 8`
- Normalized [0,1] → World: `coord * WORLD_SIZE_UNITS`

### 4. Mesh Generation
**Octree returns normalized [0,1] coordinates**:
- Mesh vertices in range [0, 1]³
- Scaled to world space at render time: `vertex * WORLD_SIZE_UNITS`
- Allows octree to be size-agnostic

### 5. WASM Initialization
**Problem**: Multiple `init()` calls caused out-of-memory errors

**Solution**: Singleton initialization pattern
- Create `worldWasm.ts` with `ensureWorldWasmInitialized()`
- All modules use shared init function
- Ensures only ONE WASM instance created

**Files using WASM**:
- `WorldCanvas.tsx` (AvatarEngine)
- `geometry-lib.ts` (GeometryEngine)
- `voxLoader.ts` (load_vox_from_bytes)

### 6. Worker vs Main Thread
**Problem**: Web Workers can't share WASM memory, each needs own instance

**Initial Approach**: Geometry worker for async generation
**Issue**: Out of memory with multiple WASM instances

**Solution**: Run geometry on main thread
- Small world (8×8) is fast enough
- Avoids WASM memory duplication
- Update at ~30 FPS with setInterval
- Can revisit worker approach with SharedArrayBuffer later

## Implementation Details

### Files to Create

1. **`packages/common/src/constants.ts`**
   - Export `WORLD_CONSTANTS` object
   - Helper functions: `getVoxelSizeAtDepth()`, coordinate conversions

2. **`packages/editor/src/constants.ts`**
   - Re-export from common package

3. **`packages/app/src/utils/worldWasm.ts`**
   - Singleton WASM initialization
   - `ensureWorldWasmInitialized()` function

4. **`packages/editor/src/components/DebugPanel.tsx`**
   - Show constants: WORLD_DEPTH, TOTAL_DEPTH, etc.
   - Show cursor: position, depth, voxel size
   - Toggle with edit mode

### Files to Modify

#### Rust (crates/world/)

**`src/lib.rs`**:
- Add `total_depth` parameter to `GeometryEngine::new()`
- Pass to `CubeGround::new(total_depth)`

**`src/geometry/mod.rs`**:
- `GeometryEngine::new_with_depth(total_depth: u32)`
- Pass depth to cube_ground constructor

**`src/geometry/cube_ground/mod.rs`**:
- Remove world-specific validation
- Generic `set_voxel_at_depth()` with coordinate scaling
- Accept coordinates at max depth, scale internally

#### TypeScript (packages/app/)

**`src/geometry/geometry-lib.ts`**:
- Import `ensureWorldWasmInitialized` from `worldWasm.ts`
- Pass `WORLD_CONSTANTS.totalDepth` to `GeometryEngine`

**`src/geometry/geometry-controller.ts`**:
- Run on main thread with `setInterval`
- Direct `GeometryGenerator` usage (no worker)
- Generate geometry at ~30 FPS

**`src/components/WorldCanvas.tsx`**:
- Use `ensureWorldWasmInitialized()` instead of `init()`

**`src/utils/voxLoader.ts`**:
- Use `ensureWorldWasmInitialized()` instead of `init()`

**`src/renderer/scene.ts`**:
- Import `WORLD_CONSTANTS`
- Scale mesh vertices: `vertex * WORLD_SIZE_UNITS`
- Update camera position for new world size
- Update grid helper size
- Update cursor depth default to TOTAL_DEPTH

**`vite.config.ts`**:
- Update alias: `@workspace/wasm-world` (not `@workspace/wasm`)

**`tsconfig.json`**:
- Update path mapping for `@workspace/wasm-world`

**`src/services/avatar-state.ts`**:
- Add `static DEBUG_LOGGING = false` flag
- Add private `log()` method that checks flag
- Replace all `console.log` calls with `this.log()`

### Files to Delete

- `packages/app/src/global.d.ts` (old WASM types)
- `packages/app/src/types/wasm.d.ts` (old WASM types)
- `packages/app/src/wasm.d.ts` (old WASM types)
- These are replaced by types from `@workspace/wasm-world` package

### Build Process

1. Build WASM: `wasm-pack build --target web --out-dir ../packages/wasm-world crates/world`
2. Clear browser cache (important!)
3. Restart dev server

## Known Issues & Solutions

### Issue 1: Out of Memory on WASM Init
**Symptom**: `RangeError: WebAssembly.instantiate(): Out of memory`

**Causes**:
1. Multiple `init()` calls creating duplicate instances
2. Old large WASM cached in browser (depth 7 = 128³)

**Solutions**:
1. Use singleton `ensureWorldWasmInitialized()`
2. Clear browser cache when changing octree depth
3. Hard refresh: Ctrl+Shift+R

### Issue 2: Worker @react-refresh Error
**Symptom**: `Uncaught ReferenceError: window is not defined at @react-refresh`

**Cause**: Vite's React Refresh plugin injected into worker context

**Solution**: Configure Vite to exclude plugins from workers:
```typescript
worker: {
  format: 'es',
  plugins: () => []
}
```

### Issue 3: Ground Mesh Not Rendering
**Symptom**: Avatar renders but ground doesn't appear

**Debug Steps**:
1. Check GeometryController initialization
2. Check for worker errors in console
3. Verify WASM loaded successfully
4. Check mesh has vertices/indices
5. Verify mesh scaling applied correctly

## Testing Checklist

- [ ] App loads without WASM errors
- [ ] Ground mesh renders (8×8 cube)
- [ ] Avatar renders on ground
- [ ] Edit mode shows debug panel
- [ ] Voxel drawing works at all depths (0-6)
- [ ] Cursor depth slider works (0-6)
- [ ] Camera positioned correctly for 8-unit world
- [ ] Grid helper size matches world (8 units)
- [ ] No console errors on page load
- [ ] Hard refresh after cache clear works

## Rollback Plan

If issues occur:
1. `git stash` - save work in progress
2. `git reset --hard <last-good-commit>` - revert to known state
3. `wasm-pack build` - rebuild WASM at old commit
4. Clear browser cache
5. Restart dev server

Last known good commits:
- `eaeb6d6` - Fix scale_factor (depth 4, 16³ octree)
- `740c7ab` - Fix mesh generation

## Future Improvements

1. **SharedArrayBuffer for Workers**
   - Allow geometry worker to share WASM memory
   - Better for larger worlds

2. **Dynamic World Sizes**
   - Allow runtime world size changes
   - Useful for different game modes

3. **LOD System**
   - Render distant voxels at lower depth
   - Improve performance for large worlds

4. **Streaming Octree**
   - Load/unload octree chunks
   - Support very large worlds

## References

- Original issue discussion in session context
- World constants: `packages/common/src/constants.ts`
- Octree implementation: `crates/cube/src/octree.rs`
- Mesh generation: `crates/cube/src/mesh.rs`
