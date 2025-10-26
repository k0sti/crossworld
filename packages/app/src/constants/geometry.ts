/**
 * Geometry and coordinate system helper functions
 *
 * Architecture:
 * - Macro depth: Octree subdivision depth (3 = 8^3 voxels in octree space)
 * - Micro depth: Rendering scale depth (0 = each octree unit is 2^0 = 1 world unit)
 * - Total depth: macro + micro = 3
 * - World size: 2^totalDepth * 2^microDepth = 8 * 1 = 8 units
 * - At macro depth (3), smallest voxel = 1 world unit (matches unit cube)
 *
 * Note: Actual depth values are managed by depth-config.ts
 */

/**
 * Calculate maximum voxels per side at finest detail level in octree space
 * @param depth Total depth (macro + micro)
 */
export function getMaxVoxelsPerSide(depth: number): number {
  return 1 << depth;
}

/**
 * Calculate world size in units after scaling
 * @param depth Total depth (macro + micro)
 * @param microDepth Rendering scale depth
 */
export function getWorldSize(depth: number, microDepth: number): number {
  return (1 << depth) * (1 << microDepth);
}

/**
 * Calculate half world size (used for centering coordinates)
 * @param depth Total depth (macro + micro)
 * @param microDepth Rendering scale depth
 */
export function getHalfWorld(depth: number, microDepth: number): number {
  return getWorldSize(depth, microDepth) / 2;
}

/**
 * Calculate scale factor from world to octree coordinates
 * @param microDepth Rendering scale depth
 */
export function getWorldToOctreeScale(microDepth: number): number {
  return 1 / (1 << microDepth);
}

/**
 * Calculate default cursor depth for edit mode
 * @param depth Total depth (macro + micro)
 * @param microDepth Rendering scale depth
 */
export function getDefaultCursorDepth(depth: number, microDepth: number): number {
  return depth - microDepth;
}

/**
 * Calculate voxel size in world units for a given depth level
 * @param targetDepth Target octree depth
 * @param maxDepth Maximum depth (macro + micro)
 * @param microDepth Rendering scale depth
 */
export function getVoxelSize(targetDepth: number, maxDepth: number, microDepth: number): number {
  return 1 << (maxDepth - targetDepth + microDepth);
}

/**
 * Get the valid world coordinate range
 * @param depth Total depth (macro + micro)
 * @param microDepth Rendering scale depth
 */
export function getWorldBounds(depth: number, microDepth: number): { min: number; max: number } {
  const halfWorld = getHalfWorld(depth, microDepth);
  return {
    min: -halfWorld,
    max: halfWorld
  };
}

/**
 * Get the valid octree coordinate range at max depth
 * @param depth Total depth (macro + micro)
 */
export function getOctreeBounds(depth: number): { min: number; max: number } {
  return {
    min: 0,
    max: getMaxVoxelsPerSide(depth) - 1
  };
}
