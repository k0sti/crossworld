import {
  getMaxVoxelsPerSide,
  getWorldSize,
  getHalfWorld,
  getWorldToOctreeScale,
  getVoxelSize as getVoxelSizeFromGeometry
} from '../constants/geometry';
import { getMicroDepth, getTotalDepth } from '../config/depth-config';

/**
 * Represents a cube coordinate in octree space at a specific depth level.
 *
 * Coordinates are always in octree space at max depth (0 to maxVoxelsPerSide-1).
 * The depth field indicates the target depth for operations (coarser depths use fewer bits).
 *
 * Coordinate system (origin centered at ground plane):
 * - World space: x[-halfWorld, halfWorld] (worldSize units per side)
 * - Octree space: x[0, maxVoxelsPerSide-1] (voxels per side at max depth)
 * - World (0,0,0) maps to octree center
 * - Ground plane is at y=0
 */
export interface CubeCoord {
  /** X coordinate in octree space at max depth [0, maxVoxelsPerSide) */
  x: number;
  /** Y coordinate in octree space at max depth [0, maxVoxelsPerSide) */
  y: number;
  /** Z coordinate in octree space at max depth [0, maxVoxelsPerSide) */
  z: number;
  /** Target octree depth level (totalDepth=finest, lower=coarser) */
  depth: number;
}

/**
 * Convert world coordinates to octree cube coordinates.
 * @param worldX World X coordinate [-halfWorld, halfWorld]
 * @param worldY World Y coordinate [-halfWorld, halfWorld]
 * @param worldZ World Z coordinate [-halfWorld, halfWorld]
 * @param depth Target octree depth level
 * @returns CubeCoord in octree space
 */
export function worldToCube(
  worldX: number,
  worldY: number,
  worldZ: number,
  depth: number
): CubeCoord {
  const microDepth = getMicroDepth();
  const totalDepth = getTotalDepth();

  const halfWorld = getHalfWorld(totalDepth, microDepth);
  const worldToOctreeScale = getWorldToOctreeScale(microDepth);

  // Convert world coords to octree coords at max depth
  // World: x[-halfWorld, halfWorld]
  // Octree: x[0, maxVoxelsPerSide-1]
  // Apply uniform +halfWorld offset to center the coordinate system
  const octreeX = Math.floor((worldX + halfWorld) * worldToOctreeScale);
  const octreeY = Math.floor((worldY + halfWorld) * worldToOctreeScale);
  const octreeZ = Math.floor((worldZ + halfWorld) * worldToOctreeScale);

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
  const microDepth = getMicroDepth();
  const totalDepth = getTotalDepth();

  const worldSize = getWorldSize(totalDepth, microDepth);
  const maxVoxelsPerSide = getMaxVoxelsPerSide(totalDepth);
  const halfWorld = getHalfWorld(totalDepth, microDepth);

  const scale = worldSize / maxVoxelsPerSide;

  // Remove the +halfWorld offset to convert back to centered world coords
  const worldX = coord.x * scale - halfWorld;
  const worldY = coord.y * scale - halfWorld;
  const worldZ = coord.z * scale - halfWorld;

  return [worldX, worldY, worldZ];
}

/**
 * Calculate voxel size for a given depth
 * @param depth Target octree depth
 * @returns Voxel size in world units
 */
export function getVoxelSize(depth: number): number {
  const totalDepth = getTotalDepth();
  const microDepth = getMicroDepth();
  return getVoxelSizeFromGeometry(depth, totalDepth, microDepth);
}

/**
 * Check if world coordinates are within valid bounds
 * @param x World X coordinate
 * @param z World Z coordinate
 * @param size Size of voxel in world units
 * @returns true if coordinates are valid
 */
export function isWithinWorldBounds(x: number, z: number, size: number = 1): boolean {
  const totalDepth = getTotalDepth();
  const microDepth = getMicroDepth();
  const halfWorld = getHalfWorld(totalDepth, microDepth);

  const minBound = -halfWorld;
  const maxBound = halfWorld - size;
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
  const totalDepth = getTotalDepth();
  const maxVoxelsPerSide = getMaxVoxelsPerSide(totalDepth);

  return x >= 0 && x < maxVoxelsPerSide &&
         y >= 0 && y < maxVoxelsPerSide &&
         z >= 0 && z < maxVoxelsPerSide;
}

/**
 * Clamp world coordinates to valid bounds
 * @param x World X coordinate
 * @param z World Z coordinate
 * @returns Clamped [x, z] coordinates
 */
export function clampToWorldBounds(x: number, z: number): [number, number] {
  const totalDepth = getTotalDepth();
  const microDepth = getMicroDepth();
  const halfWorld = getHalfWorld(totalDepth, microDepth);

  const minBound = -halfWorld + 1;
  const maxBound = halfWorld - 1;
  return [
    Math.max(minBound, Math.min(maxBound, x)),
    Math.max(minBound, Math.min(maxBound, z))
  ];
}

/**
 * Snap world coordinate to voxel grid
 * @param worldCoord World space coordinate
 * @param size Voxel size in world units
 * @returns Snapped center coordinate (voxel corners align with multiples of size at origin)
 */
export function snapToGrid(worldCoord: number, size: number): number {
  // Snap to voxel centers where corners align with 0, size, 2*size, etc.
  // Centers are at size/2, size*1.5, size*2.5, etc.
  return Math.floor(worldCoord / size) * size + size / 2;
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
