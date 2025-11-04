import * as THREE from 'three';
import { worldToCube, cubeToWorld, type CubeCoord, getVoxelSize } from './cube-coord';
import { getMacroDepth, getMicroDepth } from '../config/depth-config';

/**
 * Determine the depth level of a voxel face based on its size
 *
 * @param faceSize Estimated size of the hit face in world units
 * @returns Depth level that matches this face size
 */
export function depthFromFaceSize(faceSize: number): number {
  const macroDepth = getMacroDepth();
  const microDepth = getMicroDepth();
  const totalDepth = macroDepth + microDepth;

  // Find the depth where voxel size matches the face size
  for (let depth = 0; depth <= totalDepth; depth++) {
    const voxelSize = getVoxelSize(depth);
    // Allow some tolerance for floating point comparison
    if (Math.abs(voxelSize - faceSize) < 0.01) {
      return depth;
    }
    // If we've gone smaller, return the previous depth
    if (voxelSize < faceSize) {
      return Math.max(0, depth - 1);
    }
  }

  return totalDepth;
}

/**
 * Convert a raycast hit to a CubeCoord, properly handling depth and side
 *
 * @param hitPoint World space hit point on the face
 * @param hitNormal Face normal vector
 * @param faceSize Size of the hit face (to determine depth)
 * @param placementSide 'near' for placing on near side, 'far' for far side
 * @param targetDepth Target depth for the cursor (optional, defaults to face depth)
 * @returns CubeCoord at target depth
 */
export function raycastToCubeCoord(
  hitPoint: THREE.Vector3,
  hitNormal: THREE.Vector3,
  faceSize: number,
  placementSide: 'near' | 'far' = 'near',
  targetDepth?: number
): CubeCoord {
  // Step 1: Determine depth of the hit face
  const hitDepth = depthFromFaceSize(faceSize);

  // Step 2: Convert hit point to CubeCoord at hit depth
  // Offset slightly along normal to ensure we're inside the voxel
  const epsilon = 0.001;
  const offsetDir = placementSide === 'far' ? -1 : 1;
  const adjustedPoint = hitPoint.clone().addScaledVector(hitNormal, epsilon * offsetDir);

  let hitCoord = worldToCube(
    adjustedPoint.x,
    adjustedPoint.y,
    adjustedPoint.z,
    hitDepth
  );

  // Step 3: If placing on far side, offset by one voxel in normal direction
  if (placementSide === 'far') {
    // Convert normal to octree space offset
    const normalOffset = {
      x: Math.round(hitNormal.x),
      y: Math.round(hitNormal.y),
      z: Math.round(hitNormal.z)
    };

    hitCoord = {
      x: hitCoord.x + normalOffset.x,
      y: hitCoord.y + normalOffset.y,
      z: hitCoord.z + normalOffset.z,
      depth: hitDepth
    };
  }

  // Step 4: Scale to target depth if different from hit depth
  if (targetDepth !== undefined && targetDepth !== hitDepth) {
    return scaleCubeCoord(hitCoord, targetDepth);
  }

  return hitCoord;
}

/**
 * Scale a CubeCoord from one depth to another based on the voxel's minimum corner
 * This ensures proper alignment when changing cursor sizes
 *
 * The key insight: when you hit a voxel and want to show a cursor at a different depth,
 * the cursor should be positioned at the target-depth voxel whose corner aligns with or
 * contains the hit voxel's corner position.
 *
 * @param coord Source CubeCoord (the hit voxel)
 * @param targetDepth Target depth level (cursor depth)
 * @returns CubeCoord at target depth that properly contains/aligns with source voxel
 */
export function scaleCubeCoord(coord: CubeCoord, targetDepth: number): CubeCoord {
  if (coord.depth === targetDepth) {
    return coord;
  }

  // Convert source voxel to world space - this gives us the minimum corner
  const [worldX, worldY, worldZ] = cubeToWorld(coord);

  // Convert this corner position to the target depth
  // This finds the target-depth voxel that contains this corner point
  return worldToCube(worldX, worldY, worldZ, targetDepth);
}

/**
 * Estimate face size from raycast intersection
 * This uses neighboring triangle vertices to estimate the quad size
 *
 * @param intersection THREE.Intersection from raycaster
 * @returns Estimated face size in world units
 */
export function estimateFaceSizeFromIntersection(intersection: THREE.Intersection): number {
  // If we have face information, try to get actual size from geometry
  if (intersection.face && intersection.object.type === 'Mesh') {
    const mesh = intersection.object as THREE.Mesh;
    const geometry = mesh.geometry;

    if (geometry.type === 'BufferGeometry') {
      const positions = geometry.attributes.position;
      const face = intersection.face;

      // Get the three vertices of the triangle
      const v1 = new THREE.Vector3().fromBufferAttribute(positions, face.a);
      const v2 = new THREE.Vector3().fromBufferAttribute(positions, face.b);
      const v3 = new THREE.Vector3().fromBufferAttribute(positions, face.c);

      // Calculate edge lengths
      const edge1 = v1.distanceTo(v2);
      const edge2 = v2.distanceTo(v3);
      const edge3 = v3.distanceTo(v1);

      // For a cube face (two triangles forming a quad), the longest edges
      // should be the diagonals. The shorter edges are the quad edges.
      const edges = [edge1, edge2, edge3].sort((a, b) => a - b);

      // Return the shortest edge as the face size (should be the quad edge)
      return edges[0];
    }
  }

  // Fallback: assume depth 3 (1 unit voxel)
  return 1.0;
}
