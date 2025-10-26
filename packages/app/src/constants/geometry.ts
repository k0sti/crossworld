/**
 * Geometry and coordinate system constants
 *
 * Architecture:
 * - MACRO_DEPTH: Octree subdivision depth (3 = 8^3 voxels in octree space)
 * - MICRO_DEPTH: Rendering scale depth (1 = each octree unit is 2^1 = 2 world units)
 * - DEPTH: Total depth (macro + micro = 4)
 * - World size: 2^DEPTH * 2^MICRO_DEPTH = 16 * 2 = 32 units
 */

/** Default macro depth - octree subdivision levels */
export const DEFAULT_MACRO_DEPTH = 3;

/** Default micro depth - rendering scale depth */
export const DEFAULT_MICRO_DEPTH = 1;

/** Default total depth (macro + micro) */
export const DEFAULT_DEPTH = DEFAULT_MACRO_DEPTH + DEFAULT_MICRO_DEPTH;

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

// Legacy exports for backward compatibility (using default values)
/** @deprecated Use getMaxVoxelsPerSide(depth) instead */
export const MAX_VOXELS_PER_SIDE = getMaxVoxelsPerSide(DEFAULT_DEPTH);

/** @deprecated Use getWorldSize(depth, microDepth) instead */
export const WORLD_SIZE = getWorldSize(DEFAULT_DEPTH, DEFAULT_MICRO_DEPTH);

/** @deprecated Use getHalfWorld(depth, microDepth) instead */
export const HALF_WORLD = getHalfWorld(DEFAULT_DEPTH, DEFAULT_MICRO_DEPTH);

/** @deprecated Use getWorldToOctreeScale(microDepth) instead */
export const WORLD_TO_OCTREE_SCALE = getWorldToOctreeScale(DEFAULT_MICRO_DEPTH);

/** @deprecated Use getDefaultCursorDepth(depth, microDepth) instead */
export const DEFAULT_CURSOR_DEPTH = getDefaultCursorDepth(DEFAULT_DEPTH, DEFAULT_MICRO_DEPTH);

/** @deprecated Use DEFAULT_DEPTH instead */
export const WORLD_DEPTH = DEFAULT_DEPTH;

/** @deprecated Use DEFAULT_MICRO_DEPTH instead */
export const WORLD_SCALE_DEPTH = DEFAULT_MICRO_DEPTH;
