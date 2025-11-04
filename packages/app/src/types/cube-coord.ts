import {
  getHalfWorld,
  getVoxelSize as getVoxelSizeFromGeometry
} from '../constants/geometry';
import { getMacroDepth, getMicroDepth } from '../config/depth-config';

/**
 * Represents a cube coordinate in octree space at a specific depth level.
 *
 * Coordinates are in octree space at the target depth (0 to 2^depth-1).
 * The depth field indicates the depth level for these coordinates.
 *
 * Coordinate system (origin centered at ground plane):
 * - World space: x[-halfWorld, halfWorld] (worldSize units per side)
 * - Octree space at depth d: x[0, 2^d-1] (voxels per side at depth d)
 * - World (0,0,0) maps to octree center
 * - Ground plane is at y=0
 */
export interface CubeCoord {
  /** X coordinate in octree space at target depth [0, 2^depth) */
  x: number;
  /** Y coordinate in octree space at target depth [0, 2^depth) */
  y: number;
  /** Z coordinate in octree space at target depth [0, 2^depth) */
  z: number;
  /** Target octree depth level (higher=finer subdivision) */
  depth: number;
}

/**
 * Convert world coordinates to octree cube coordinates.
 *
 * # Coordinate Spaces
 * - World space: [-halfWorld, halfWorld] in world units (continuous)
 * - Octree space at depth d: [0, 2^d - 1] (discrete voxel indices)
 *
 * # Voxel Size by Depth
 * At macroDepth=3 (8×8×8 world units):
 * - depth=0: voxelSize=8 units, coords [0,0] (entire world)
 * - depth=1: voxelSize=4 units, coords [0,1] (2×2×2 voxels)
 * - depth=2: voxelSize=2 units, coords [0,3] (4×4×4 voxels)
 * - depth=3: voxelSize=1 unit,  coords [0,7] (8×8×8 voxels) ← macro depth
 * - depth=4: voxelSize=0.5 units, coords [0,15] (16×16×16 voxels)
 *
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
  const macroDepth = getMacroDepth();
  const halfWorld = getHalfWorld(macroDepth);

  // Convert world coords to octree coords at target depth
  // World: x[-halfWorld, halfWorld] (in world units)
  // Octree: x[0, 2^depth-1] (in octree voxels at target depth)
  let octreeX: number, octreeY: number, octreeZ: number;

  if (depth >= macroDepth) {
    // Fine voxels (depth >= macro): multiple octree voxels per world unit
    // voxelsPerWorldUnit = 2^(depth-macro)
    // Example: depth=4, macro=3 → scale=2, so 2 voxels per world unit
    const scale = 1 << (depth - macroDepth);
    octreeX = Math.floor((worldX + halfWorld) * scale);
    octreeY = Math.floor((worldY + halfWorld) * scale);
    octreeZ = Math.floor((worldZ + halfWorld) * scale);
  } else {
    // Coarse voxels (depth < macro): each voxel covers multiple world units
    // worldUnitsPerVoxel = 2^(macro-depth)
    // Example: depth=1, macro=3 → scale=4, so each voxel is 4×4×4 world units
    const scale = 1 << (macroDepth - depth);
    octreeX = Math.floor((worldX + halfWorld) / scale);
    octreeY = Math.floor((worldY + halfWorld) / scale);
    octreeZ = Math.floor((worldZ + halfWorld) / scale);
  }

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
  const macroDepth = getMacroDepth();
  const halfWorld = getHalfWorld(macroDepth);

  let worldX: number, worldY: number, worldZ: number;

  if (coord.depth >= macroDepth) {
    // Fine voxels: multiple octree voxels per world unit
    const scale = 1 << (coord.depth - macroDepth);
    worldX = coord.x / scale - halfWorld;
    worldY = coord.y / scale - halfWorld;
    worldZ = coord.z / scale - halfWorld;
  } else {
    // Coarse voxels: each voxel covers multiple world units
    const scale = 1 << (macroDepth - coord.depth);
    worldX = coord.x * scale - halfWorld;
    worldY = coord.y * scale - halfWorld;
    worldZ = coord.z * scale - halfWorld;
  }

  return [worldX, worldY, worldZ];
}

/**
 * Calculate voxel size for a given depth
 * @param depth Target octree depth
 * @returns Voxel size in world units
 */
export function getVoxelSize(depth: number): number {
  const macroDepth = getMacroDepth();
  const microDepth = getMicroDepth();
  return getVoxelSizeFromGeometry(depth, macroDepth, microDepth);
}

/**
 * Check if world coordinates are within valid bounds
 * @param x World X coordinate
 * @param z World Z coordinate
 * @param size Size of voxel in world units
 * @returns true if coordinates are valid
 */
export function isWithinWorldBounds(x: number, z: number, size: number = 1): boolean {
  const macroDepth = getMacroDepth();
  const halfWorld = getHalfWorld(macroDepth);

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
export function isWithinOctreeBounds(x: number, y: number, z: number, depth: number): boolean {
  const maxVoxelsPerSide = 1 << depth; // 2^depth

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
  const macroDepth = getMacroDepth();
  const halfWorld = getHalfWorld(macroDepth);

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
