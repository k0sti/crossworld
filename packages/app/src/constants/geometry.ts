/**
 * Geometry and coordinate system helper functions
 *
 * Architecture:
 * - Macro depth: Determines world size (e.g., 3 → 8×8×8 world units)
 * - Micro depth: Sub-unit voxel subdivisions (e.g., 2 → 4×4×4 subdivisions per unit)
 * - Total depth: macro + micro (e.g., 3 + 2 = 5)
 * - World size: 2^macro units (independent of micro depth)
 * - Octree size: 2^(macro+micro) voxels per side
 * - Each world unit contains 2^micro octree voxels per dimension
 *
 * Example: macro=3, micro=2, total=5
 * - World: 8×8×8 units (2^3)
 * - Octree: 32×32×32 voxels (2^5)
 * - Each world unit = 4×4×4 octree voxels (2^2)
 *
 * Note: Actual depth values are managed by depth-config.ts
 */

/**
 * Calculate maximum voxels per side at finest detail level in octree space
 * @param totalDepth Total depth (macro + micro)
 */
export function getMaxVoxelsPerSide(totalDepth: number): number {
  return 1 << totalDepth;
}

/**
 * Calculate world size in units (only depends on macro depth)
 * @param macroDepth Macro depth
 */
export function getWorldSize(macroDepth: number): number {
  return 1 << macroDepth;
}

/**
 * Calculate half world size (used for centering coordinates)
 * @param macroDepth Macro depth
 */
export function getHalfWorld(macroDepth: number): number {
  return getWorldSize(macroDepth) / 2;
}

/**
 * Calculate scale factor from world to octree coordinates
 * @param microDepth Micro depth (subdivisions per world unit)
 */
export function getWorldToOctreeScale(microDepth: number): number {
  return 1 << microDepth; // Each world unit = 2^micro octree voxels
}

/**
 * Calculate default cursor depth for edit mode (macro depth = unit voxels)
 * @param macroDepth Macro depth
 */
export function getDefaultCursorDepth(macroDepth: number): number {
  return macroDepth; // Default to macro depth (unit voxels)
}

/**
 * Calculate voxel size in world units for a given depth level
 * @param targetDepth Target octree depth level
 * @param macroDepth Macro depth
 * @param _microDepth Micro depth (unused, reserved for future use)
 */
export function getVoxelSize(targetDepth: number, macroDepth: number, _microDepth: number): number {
  // At depth 0: voxel size = 2^macro world units (entire world)
  // At macro depth: voxel size = 1 world unit
  // At macro+micro depth: voxel size = 1/(2^micro) world units
  if (targetDepth <= macroDepth) {
    // Coarse levels: voxel size >= 1 world unit
    return 1 << (macroDepth - targetDepth);
  } else {
    // Fine levels: voxel size < 1 world unit
    return 1.0 / (1 << (targetDepth - macroDepth));
  }
}

/**
 * Get the valid world coordinate range
 * @param macroDepth Macro depth
 */
export function getWorldBounds(macroDepth: number): { min: number; max: number } {
  const halfWorld = getHalfWorld(macroDepth);
  return {
    min: -halfWorld,
    max: halfWorld
  };
}

/**
 * Get the valid octree coordinate range at max depth
 * @param totalDepth Total depth (macro + micro)
 */
export function getOctreeBounds(totalDepth: number): { min: number; max: number } {
  return {
    min: 0,
    max: getMaxVoxelsPerSide(totalDepth) - 1
  };
}
