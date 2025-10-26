/**
 * Geometry and coordinate system constants
 *
 * These constants define the world size and coordinate system.
 * All coordinate-related calculations should use these values.
 *
 * Architecture:
 * - WORLD_DEPTH: Octree subdivision depth (5 = 32^3 voxels in octree space)
 * - WORLD_SCALE_DEPTH: Rendering scale (1 = each octree unit is 2^1=2 world units)
 * - World size: 32 * 2 = 64 units
 * - At max depth (5), each voxel is 2 world units
 * - At depth 4, each voxel is 4 world units
 */

/** Maximum octree subdivision depth (5 = 32^3 voxels in octree space) */
export const WORLD_DEPTH = 5;

/** Rendering scale depth - voxels are scaled by 2^WORLD_SCALE_DEPTH when rendered */
export const WORLD_SCALE_DEPTH = 1;

/** Maximum voxels per side at finest detail level in octree space (32 for depth 5) */
export const MAX_VOXELS_PER_SIDE = 1 << WORLD_DEPTH;

/** World size in units after scaling (32 * 2 = 64 for depth 5, scale 1) */
export const WORLD_SIZE = MAX_VOXELS_PER_SIDE * (1 << WORLD_SCALE_DEPTH);

/** Half world size, used for centering coordinates (32 for depth 5) */
export const HALF_WORLD = WORLD_SIZE / 2;

/** Scale factor from world to octree coordinates (0.5 for scale depth 1) */
export const WORLD_TO_OCTREE_SCALE = 1 / (1 << WORLD_SCALE_DEPTH);

/** Default cursor depth for edit mode (4 = 4 world unit voxels) */
export const DEFAULT_CURSOR_DEPTH = WORLD_DEPTH - WORLD_SCALE_DEPTH;

/** Backward compatibility alias */
export const SUBDIVISION_DEPTH = WORLD_DEPTH;

/**
 * Calculate voxel size in world units for a given depth level
 * @param depth Target octree depth (0-WORLD_DEPTH)
 * @returns Voxel size in world units (e.g., depth=5→2, depth=4→4, depth=0→64)
 */
export function getVoxelSize(depth: number): number {
  return 1 << (WORLD_DEPTH - depth + WORLD_SCALE_DEPTH);  // 2^(max_depth - depth + scale_depth)
}

/**
 * Get the valid world coordinate range
 * @returns Min and max bounds for world coordinates
 */
export function getWorldBounds(): { min: number; max: number } {
  return {
    min: -HALF_WORLD,
    max: HALF_WORLD
  };
}

/**
 * Get the valid octree coordinate range at max depth
 * @returns Min and max bounds for octree coordinates
 */
export function getOctreeBounds(): { min: number; max: number } {
  return {
    min: 0,
    max: MAX_VOXELS_PER_SIDE - 1
  };
}
