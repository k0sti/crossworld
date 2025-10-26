import {
  WORLD_SIZE,
  WORLD_DEPTH,
  DEFAULT_CURSOR_DEPTH,
  MAX_VOXELS_PER_SIDE,
  WORLD_TO_OCTREE_SCALE,
  HALF_WORLD
} from '../constants/geometry';

/**
 * Represents a cube coordinate in octree space at a specific depth level.
 *
 * Coordinates are always in octree space at max depth (0 to MAX_VOXELS_PER_SIDE-1).
 * The depth field indicates the target depth for operations (coarser depths use fewer bits).
 *
 * Coordinate system (origin centered at ground plane):
 * - World space: x[-HALF_WORLD, HALF_WORLD] (WORLD_SIZE units per side = 64 units)
 * - Octree space: x[0, MAX_VOXELS_PER_SIDE-1] (32 voxels per side at max depth)
 * - World (0,0,0) maps to octree (16, 16, 16) - center of the cube
 * - Ground plane is at y=0
 * - At max depth (WORLD_DEPTH=5), 1 octree unit = 2 world units (due to WORLD_SCALE_DEPTH=1)
 */
export interface CubeCoord {
  /** X coordinate in octree space at max depth [0, MAX_VOXELS_PER_SIDE) */
  x: number;
  /** Y coordinate in octree space at max depth [0, MAX_VOXELS_PER_SIDE) */
  y: number;
  /** Z coordinate in octree space at max depth [0, MAX_VOXELS_PER_SIDE) */
  z: number;
  /** Target octree depth level (WORLD_DEPTH=finest, lower=coarser) */
  depth: number;
}

// Re-export constants for backward compatibility
export { WORLD_DEPTH as MAX_DEPTH, DEFAULT_CURSOR_DEPTH, WORLD_SIZE };

/**
 * Convert world coordinates to octree cube coordinates.
 * @param worldX World X coordinate [-HALF_WORLD, HALF_WORLD]
 * @param worldY World Y coordinate [-HALF_WORLD, HALF_WORLD]
 * @param worldZ World Z coordinate [-HALF_WORLD, HALF_WORLD]
 * @param depth Target octree depth level
 * @returns CubeCoord in octree space
 */
export function worldToCube(
  worldX: number,
  worldY: number,
  worldZ: number,
  depth: number
): CubeCoord {
  // Convert world coords to octree coords at max depth
  // World: x[-HALF_WORLD, HALF_WORLD]
  // Octree: x[0, MAX_VOXELS_PER_SIDE-1]
  // Apply uniform +HALF_WORLD offset to center the coordinate system
  const octreeX = Math.floor((worldX + HALF_WORLD) * WORLD_TO_OCTREE_SCALE);
  const octreeY = Math.floor((worldY + HALF_WORLD) * WORLD_TO_OCTREE_SCALE);
  const octreeZ = Math.floor((worldZ + HALF_WORLD) * WORLD_TO_OCTREE_SCALE);

  // Disabled: Too verbose (called every frame)
  // console.log('[worldToCube]', {
  //   input: { worldX, worldY, worldZ, depth },
  //   scale,
  //   halfWorld,
  //   output: { octreeX, octreeY, octreeZ }
  // });

  return {
    x: octreeX,
    y: octreeY,
    z: octreeZ,
    depth
  };
}

/**
 * Convert cube coordinates back to world coordinates (returns the min corner).
 * @param coord CubeCoord in octree space
 * @returns World coordinates [x, y, z]
 */
export function cubeToWorld(
  coord: CubeCoord
): [number, number, number] {
  const scale = WORLD_SIZE / MAX_VOXELS_PER_SIDE;

  // Remove the +HALF_WORLD offset to convert back to centered world coords
  const worldX = coord.x * scale - HALF_WORLD;
  const worldY = coord.y * scale - HALF_WORLD;
  const worldZ = coord.z * scale - HALF_WORLD;

  // Disabled: Too verbose (called every frame)
  // console.log('[cubeToWorld]', {
  //   input: { x: coord.x, y: coord.y, z: coord.z, depth: coord.depth },
  //   scale,
  //   halfWorld,
  //   output: { worldX, worldY, worldZ }
  // });

  return [worldX, worldY, worldZ];
}

/**
 * Calculate voxel size for a given depth (re-exported from constants for convenience)
 * @param depth Target octree depth
 * @returns Voxel size in world units
 */
export function getVoxelSize(depth: number): number {
  return 1 << (WORLD_DEPTH - depth + 1);  // +1 is WORLD_SCALE_DEPTH
}

/**
 * Check if world coordinates are within valid bounds
 * @param x World X coordinate
 * @param z World Z coordinate
 * @param size Size of voxel in world units
 * @returns true if coordinates are valid
 */
export function isWithinWorldBounds(x: number, z: number, size: number = 1): boolean {
  const minBound = -HALF_WORLD;
  const maxBound = HALF_WORLD - size;
  return x >= minBound && x <= maxBound && z >= minBound && z <= maxBound;
}

/**
 * Check if octree coordinates are within valid bounds
 * @param x Octree X coordinate
 * @param y Octree Y coordinate
 * @param z Octree Z coordinate
 * @returns true if coordinates are valid
 */
export function isWithinOctreeBounds(x: number, y: number, z: number): boolean {
  return x >= 0 && x < MAX_VOXELS_PER_SIDE &&
         y >= 0 && y < MAX_VOXELS_PER_SIDE &&
         z >= 0 && z < MAX_VOXELS_PER_SIDE;
}

/**
 * Clamp world coordinates to valid bounds
 * @param x World X coordinate
 * @param z World Z coordinate
 * @returns Clamped [x, z] coordinates
 */
export function clampToWorldBounds(x: number, z: number): [number, number] {
  const minBound = -HALF_WORLD + 1;
  const maxBound = HALF_WORLD - 1;
  return [
    Math.max(minBound, Math.min(maxBound, x)),
    Math.max(minBound, Math.min(maxBound, z))
  ];
}

/**
 * Snap world coordinate to voxel grid
 * @param worldCoord World space coordinate
 * @param size Voxel size in world units
 * @returns Snapped center coordinate
 */
export function snapToGrid(worldCoord: number, size: number): number {
  return Math.floor(worldCoord / size + 0.5) * size;
}

/**
 * Convert CubeCoord to a string representation
 * @param coord CubeCoord to print
 * @returns String representation of the coordinate
 */
export function printCubeCoord(coord: CubeCoord | undefined | null): string {
  if (!coord) return 'N/A';
  return `${coord.x}, ${coord.y}, ${coord.z} @d${coord.depth}`;
}
