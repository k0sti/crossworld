# Raycast to CubeCoord Conversion - Implementation Guide

## Problem Statement

The current implementation incorrectly converts raycast hits to cube coordinates. It converts the hit point directly to a CubeCoord at the cursor depth, which doesn't respect the actual voxel that was hit.

### Current Broken Approach
```typescript
// ❌ WRONG: Converts world point directly to cursor depth
const hit = raycastGeometry(); // Returns { point, normal }
const coord = worldToCube(hit.point.x, hit.point.y, hit.point.z, this.cursorDepth);
```

**Why this is wrong:**
- The hit point is on the surface of a voxel at some depth D
- That voxel might be larger or smaller than the cursor size
- Converting the hit point directly to cursor depth ignores the actual voxel structure
- Results in incorrect positioning, especially when cursor depth ≠ hit voxel depth

### Correct Approach

```typescript
// ✅ CORRECT: Use the hit voxel's coordinate and scale it
const hit = raycastGeometry(); // Returns { point, normal, cubeCoord }
const cursorCoord = scaleCubeCoord(hit.cubeCoord, this.cursorDepth);
// Adjust for near/far placement if needed
if (placeFar) {
  cursorCoord.x += Math.round(hit.normal.x);
  cursorCoord.y += Math.round(hit.normal.y);
  cursorCoord.z += Math.round(hit.normal.z);
}
```

**Why this is correct:**
1. Raycast returns the actual voxel coordinate `{x, y, z, depth}` that was hit
2. We scale that coordinate to the cursor depth, preserving spatial position
3. The scaled coordinate finds the closest voxel at cursor depth that overlaps the hit voxel
4. Near/far offset is applied in octree space, ensuring correct placement

## Implementation Steps

### 1. Understanding the Coordinate Spaces

**Octree Space at Depth D:**
- Coordinates range from `[0, 2^D - 1]` per axis
- Each voxel has integer coordinates
- Voxel size in world units: `2^(macroDepth - D)` for D ≤ macroDepth

**Example with macroDepth=3:**
```
depth=0: voxelSize=8 units,  coords [0,0]    (1×1×1 voxels)
depth=1: voxelSize=4 units,  coords [0,1]    (2×2×2 voxels)
depth=2: voxelSize=2 units,  coords [0,3]    (4×4×4 voxels)
depth=3: voxelSize=1 unit,   coords [0,7]    (8×8×8 voxels)  ← macro depth
depth=4: voxelSize=0.5 units, coords [0,15]  (16×16×16 voxels)
depth=5: voxelSize=0.25 units, coords [0,31] (32×32×32 voxels)
```

### 2. Update All Raycast Usage Sites

**Files to Update:**
- `packages/app/src/renderer/scene.ts` - Main file with many raycast usages

**Search Pattern:**
```bash
grep -n "raycastGeometry()" packages/app/src/renderer/scene.ts
```

**For each usage, replace:**

```typescript
// OLD CODE:
const hit = this.raycastGeometry();
if (hit) {
  const coord = worldToCube(hit.point.x, hit.point.y, hit.point.z, this.cursorDepth);
  const [voxelX, voxelY, voxelZ] = cubeToWorld(coord);
  // ... use voxelX, voxelY, voxelZ
}

// NEW CODE:
const hit = this.raycastGeometry();
if (hit) {
  // Scale the hit voxel coordinate to cursor depth
  const cursorCoord = scaleCubeCoord(hit.cubeCoord, this.cursorDepth);

  // Adjust for near/far placement
  const placeFar = this.depthSelectMode === 1;
  if (placeFar) {
    cursorCoord.x += Math.round(hit.normal.x);
    cursorCoord.y += Math.round(hit.normal.y);
    cursorCoord.z += Math.round(hit.normal.z);
  }

  // Convert to world space if needed
  const [voxelX, voxelY, voxelZ] = cubeToWorld(cursorCoord);
  // ... use voxelX, voxelY, voxelZ
}
```

### 3. Uncomment and Use hitToCursorCoord Helper

**Location:** `packages/app/src/renderer/scene.ts:347-384`

**Uncomment the function:**
```typescript
private hitToCursorCoord(
  hitCubeCoord: CubeCoord,
  hitNormal: THREE.Vector3,
  placeFar: boolean
): CubeCoord {
  // Scale the hit coordinate to cursor depth
  let cursorCoord = scaleCubeCoord(hitCubeCoord, this.cursorDepth);

  // If placing on far side (depth select mode 1), offset in normal direction
  if (placeFar) {
    const normalOffset = {
      x: Math.round(hitNormal.x),
      y: Math.round(hitNormal.y),
      z: Math.round(hitNormal.z)
    };

    cursorCoord = {
      x: cursorCoord.x + normalOffset.x,
      y: cursorCoord.y + normalOffset.y,
      z: cursorCoord.z + normalOffset.z,
      depth: this.cursorDepth
    };
  }

  return cursorCoord;
}
```

**Uncomment the import:**
```typescript
import { scaleCubeCoord } from '../types/raycast-utils';
```

**Then simplify usage:**
```typescript
const hit = this.raycastGeometry();
if (hit) {
  const placeFar = this.depthSelectMode === 1;
  const cursorCoord = this.hitToCursorCoord(hit.cubeCoord, hit.normal, placeFar);
  const [voxelX, voxelY, voxelZ] = cubeToWorld(cursorCoord);
  // ... use voxelX, voxelY, voxelZ
}
```

### 4. Specific Locations to Update

**Location 1: Mouse move with geometry hit (Line ~1270)**
```typescript
// Around line 1267-1303
} else if (hasGeometryHit && geometryHitPoint && geometryHitNormal) {
  // OLD: const coord = worldToCube(geometryHitPoint.x, geometryHitPoint.y, geometryHitPoint.z, this.cursorDepth);

  // NEW: Get hit from earlier raycast call
  const hit = this.raycastGeometry();
  if (hit) {
    const placeFar = this.depthSelectMode === 1;
    const cursorCoord = this.hitToCursorCoord(hit.cubeCoord, hit.normal, placeFar);
    // Store and use cursorCoord
    this.currentCursorCoord = cursorCoord;
  }
}
```

**Location 2: Paint voxel (Line ~788)**
```typescript
// Around line 783-793
private paintVoxelWithSize(x: number, y: number, z: number, size: number): void {
  // The x, y, z here should already be from a properly scaled cursorCoord
  // Ensure they come from this.currentCursorCoord which was set using hitToCursorCoord

  if (!this.currentCursorCoord) {
    logger.warn('renderer', 'Cannot paint: no current cursor coord');
    return;
  }

  const colorValue = this.selectedColorIndex + 32;
  logger.log('renderer', '[Paint -> CubeCoord]', {
    coord: this.currentCursorCoord,
    colorValue,
    hasCallback: !!this.onVoxelEdit
  });

  this.onVoxelEdit?.(this.currentCursorCoord, colorValue);
}
```

**Location 3: Erase voxel (Line ~800)**
```typescript
// Around line 796-806
private eraseVoxelWithSize(x: number, y: number, z: number, size: number): void {
  if (!this.currentCursorCoord) {
    logger.warn('renderer', 'Cannot erase: no current cursor coord');
    return;
  }

  logger.log('renderer', '[Erase -> CubeCoord]', {
    coord: this.currentCursorCoord,
    hasCallback: !!this.onVoxelEdit
  });

  this.onVoxelEdit?.(this.currentCursorCoord, 0);
}
```

**Location 4: Click to set edit plane (Line ~658)**
```typescript
// Around line 650-670
const hit = this.raycastGeometry();
if (hit) {
  this.setActiveEditPlane(hit.point, hit.normal);

  // OLD: Calculate voxel position using SPACE mode (same as cursor)
  // const coord = worldToCube(hit.point.x, hit.point.y, hit.point.z, this.cursorDepth);

  // NEW: Use proper coordinate scaling
  const placeFar = this.depthSelectMode === 1;
  const cursorCoord = this.hitToCursorCoord(hit.cubeCoord, hit.normal, placeFar);

  // Use cursorCoord for positioning
  const [voxelX, voxelY, voxelZ] = cubeToWorld(cursorCoord);
  // ... rest of the logic
}
```

### 5. Testing the Fix

**Test Cases:**

1. **Same Depth Test:**
   - Set cursor to depth 3 (1 unit voxels)
   - Draw some voxels
   - Try to paint on them
   - ✅ Cursor should align perfectly with voxel faces

2. **Larger Cursor Test:**
   - Set cursor to depth 2 (2 unit voxels)
   - Hover over depth 3 voxels (1 unit)
   - ✅ Cursor should snap to nearest depth-2 voxel that overlaps the hit voxel

3. **Smaller Cursor Test:**
   - Set cursor to depth 4 (0.5 unit voxels)
   - Hover over depth 3 voxels (1 unit)
   - ✅ Cursor should snap to one of the 8 depth-4 voxels inside the hit voxel
   - ✅ Should choose the voxel closest to the hit point

4. **Near/Far Placement Test:**
   - Toggle depth select mode (Q key)
   - Hover over a voxel face
   - ✅ Mode 0 (near): Cursor should be on camera side of face
   - ✅ Mode 1 (far): Cursor should be on opposite side of face
   - Both should correctly offset by 1 voxel in normal direction

5. **Edge Cases:**
   - Hover over terrain (depth 3 voxels typically)
   - Change cursor depth from 0 to 5
   - ✅ All depths should work correctly
   - ✅ No coordinates should be out of bounds

### 6. Validation Checklist

- [ ] Uncomment `hitToCursorCoord()` in scene.ts
- [ ] Uncomment `scaleCubeCoord` import
- [ ] Update all `raycastGeometry()` call sites to use `hit.cubeCoord`
- [ ] Replace all `worldToCube(hit.point, cursorDepth)` with `hitToCursorCoord()`
- [ ] Update `paintVoxelWithSize()` to use `currentCursorCoord` directly
- [ ] Update `eraseVoxelWithSize()` to use `currentCursorCoord` directly
- [ ] Test same-depth painting
- [ ] Test cross-depth painting (cursor depth ≠ voxel depth)
- [ ] Test near/far placement modes
- [ ] Test all cursor depths (0-5)
- [ ] Verify no out-of-bounds errors

## Key Functions Reference

### scaleCubeCoord (raycast-utils.ts)
```typescript
/**
 * Scale a CubeCoord from one depth to another
 * Finds the closest matching voxel position at the target depth
 */
export function scaleCubeCoord(coord: CubeCoord, targetDepth: number): CubeCoord {
  if (coord.depth === targetDepth) {
    return coord;
  }

  // Convert to world space (corner)
  const [worldX, worldY, worldZ] = cubeToWorld(coord);

  // Get voxel size at source depth to find center
  const sourceVoxelSize = getVoxelSize(coord.depth);
  const centerX = worldX + sourceVoxelSize / 2;
  const centerY = worldY + sourceVoxelSize / 2;
  const centerZ = worldZ + sourceVoxelSize / 2;

  // Convert center back to cube coord at target depth
  return worldToCube(centerX, centerY, centerZ, targetDepth);
}
```

### raycastGeometry (scene.ts)
```typescript
/**
 * Now returns the hit voxel's CubeCoord along with point and normal
 */
private raycastGeometry(): {
  point: THREE.Vector3;
  normal: THREE.Vector3;
  cubeCoord: CubeCoord
} | null {
  // ... raycasting logic ...

  if (result) {
    const hitCubeCoord: CubeCoord = {
      x: result.x,      // Octree coordinate from raycast
      y: result.y,
      z: result.z,
      depth: result.depth
    };

    return { point: hitPoint, normal: hitNormal, cubeCoord: hitCubeCoord };
  }
}
```

## Common Pitfalls

### ❌ Don't use world coordinates for octree operations
```typescript
// WRONG: Converting world point to octree at wrong depth
const coord = worldToCube(worldX, worldY, worldZ, cursorDepth);
```

### ✅ Use the raycast result's octree coordinate
```typescript
// CORRECT: Use hit voxel's coordinate and scale it
const coord = scaleCubeCoord(hit.cubeCoord, cursorDepth);
```

### ❌ Don't offset in world space
```typescript
// WRONG: Offsetting in world space loses voxel grid alignment
const offsetPoint = hit.point.clone().addScaledVector(hit.normal, voxelSize);
const coord = worldToCube(offsetPoint.x, offsetPoint.y, offsetPoint.z, depth);
```

### ✅ Offset in octree space
```typescript
// CORRECT: Offset by integer voxel units in octree space
const coord = scaleCubeCoord(hit.cubeCoord, cursorDepth);
coord.x += Math.round(hit.normal.x);
coord.y += Math.round(hit.normal.y);
coord.z += Math.round(hit.normal.z);
```

## Further Reading

- `packages/app/src/types/cube-coord.ts` - Coordinate conversion functions
- `packages/app/src/types/raycast-utils.ts` - Raycast coordinate utilities
- `packages/app/src/utils/meshRaycast.ts` - Mesh raycasting implementation
- `crates/cube/src/octree.rs` - Octree coordinate system (Rust)
