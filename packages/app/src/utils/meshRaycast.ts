import * as THREE from 'three';
import { worldToCube, type CubeCoord } from '../types/cube-coord';
import { getMacroDepth, getBorderDepth } from '../config/depth-config';
import { getWorldSize } from '../constants/geometry';

/**
 * Result from mesh raycast matching WASM raycast interface
 */
export interface MeshRaycastResult {
  /** Octree coordinates of hit voxel */
  x: number;
  y: number;
  z: number;
  depth: number;
  /** World position of hit (in normalized [0,1] space) */
  world_x: number;
  world_y: number;
  world_z: number;
  /** Surface normal at hit point */
  normal_x: number;
  normal_y: number;
  normal_z: number;
}

/**
 * Calculate depth (subdivision level) from polygon side length
 *
 * The mesh is generated with voxels at a specific depth level.
 * Each voxel face has a specific size in world units.
 * We can determine the depth by measuring the polygon size.
 *
 * At macroDepth, voxel size = 1 world unit
 * At depth d, voxel size = 2^(macroDepth - d) world units
 *
 * @param faceArea - Area of the triangle face
 * @param macroDepth - Current macro depth setting
 * @returns Estimated depth level of the voxel
 */
function calculateDepthFromPolygonSize(
  faceArea: number,
  macroDepth: number
): number {
  // For a cube face (square), two triangles make up the face
  // Each square face has area = sideLength^2
  // Each triangle has area = sideLength^2 / 2
  // So sideLength = sqrt(faceArea * 2)

  const sideLength = Math.sqrt(faceArea * 2);

  // At macroDepth, voxel size = 1
  // At depth d, voxel size = 2^(macroDepth - d)
  // So: sideLength = 2^(macroDepth - d)
  // Therefore: d = macroDepth - log2(sideLength)

  const depth = Math.round(macroDepth - Math.log2(sideLength));

  // Clamp to reasonable range (0 to macroDepth + some buffer)
  return Math.max(0, Math.min(macroDepth + 10, depth));
}

/**
 * Calculate triangle area from three vertices
 */
function calculateTriangleArea(
  v0: THREE.Vector3,
  v1: THREE.Vector3,
  v2: THREE.Vector3
): number {
  const edge1 = new THREE.Vector3().subVectors(v1, v0);
  const edge2 = new THREE.Vector3().subVectors(v2, v0);
  const cross = new THREE.Vector3().crossVectors(edge1, edge2);
  return cross.length() / 2;
}

/**
 * Perform raycast on Three.js mesh to find voxel intersection
 *
 * This function provides the same interface as the WASM raycast
 * but uses Three.js mesh raycasting instead.
 *
 * @param mesh - The geometry mesh to raycast against
 * @param raycaster - Three.js raycaster (already configured with ray)
 * @param far - If true, returns far side coordinate; if false, near side
 * @param maxDepth - Maximum octree depth (for normalization)
 * @returns RaycastResult or null if no hit
 */
export function raycastMesh(
  mesh: THREE.Mesh,
  raycaster: THREE.Raycaster,
  far: boolean,
  _maxDepth: number
): MeshRaycastResult | null {
  // Perform Three.js raycast
  const intersects = raycaster.intersectObject(mesh, false);

  if (intersects.length === 0) {
    return null;
  }

  // Get the first hit
  const hit = intersects[0];

  if (!hit.face || !hit.point) {
    return null;
  }

  // Get hit point and normal
  const hitPoint = hit.point.clone();
  const hitNormal = hit.face.normal.clone();

  // Transform normal from local to world space if needed
  if (mesh.matrixWorld) {
    const normalMatrix = new THREE.Matrix3().getNormalMatrix(mesh.matrixWorld);
    hitNormal.applyMatrix3(normalMatrix).normalize();
  }

  // Calculate polygon area to determine depth
  const geometry = mesh.geometry;
  const positionAttribute = geometry.getAttribute('position');

  if (!positionAttribute || !hit.face) {
    return null;
  }

  // Get triangle vertices
  const v0 = new THREE.Vector3();
  const v1 = new THREE.Vector3();
  const v2 = new THREE.Vector3();

  v0.fromBufferAttribute(positionAttribute, hit.face.a);
  v1.fromBufferAttribute(positionAttribute, hit.face.b);
  v2.fromBufferAttribute(positionAttribute, hit.face.c);

  // Transform vertices to world space
  v0.applyMatrix4(mesh.matrixWorld);
  v1.applyMatrix4(mesh.matrixWorld);
  v2.applyMatrix4(mesh.matrixWorld);

  // Calculate triangle area
  const faceArea = calculateTriangleArea(v0, v1, v2);

  // Calculate depth from polygon size
  const macroDepth = getMacroDepth();
  const hitDepth = calculateDepthFromPolygonSize(faceArea, macroDepth);

  // Determine the coordinate based on far parameter
  let coord: CubeCoord;

  if (far) {
    // Far side: use hit point directly
    coord = worldToCube(hitPoint.x, hitPoint.y, hitPoint.z, hitDepth);
  } else {
    // Near side: offset by one voxel in opposite direction of normal
    const voxelSize = Math.pow(2, macroDepth - hitDepth);
    const offset = hitNormal.clone().multiplyScalar(-voxelSize);
    const nearPoint = hitPoint.clone().add(offset);
    coord = worldToCube(nearPoint.x, nearPoint.y, nearPoint.z, hitDepth);
  }

  // Convert hit point to normalized [0,1] space for consistency with WASM
  const worldSize = getWorldSize(macroDepth, getBorderDepth());
  const halfWorld = worldSize / 2;

  const normalizedPoint = new THREE.Vector3(
    (hitPoint.x + halfWorld) / worldSize,
    (hitPoint.y + halfWorld) / worldSize,
    (hitPoint.z + halfWorld) / worldSize
  );

  return {
    x: coord.x,
    y: coord.y,
    z: coord.z,
    depth: coord.depth,
    world_x: normalizedPoint.x,
    world_y: normalizedPoint.y,
    world_z: normalizedPoint.z,
    normal_x: hitNormal.x,
    normal_y: hitNormal.y,
    normal_z: hitNormal.z
  };
}

/**
 * Unified raycast interface that can switch between mesh and WASM raycasting
 *
 * @param method - 'mesh' or 'wasm'
 * @param params - Parameters specific to the raycast method
 * @returns RaycastResult or null if no hit
 */
export function raycastUnified(
  method: 'mesh' | 'wasm',
  params: {
    // For mesh raycast
    mesh?: THREE.Mesh;
    raycaster?: THREE.Raycaster;
    // For WASM raycast
    wasmCube?: any;
    origin?: THREE.Vector3;
    direction?: THREE.Vector3;
    // Common parameters
    far: boolean;
    maxDepth: number;
  }
): MeshRaycastResult | null {
  if (method === 'mesh') {
    if (!params.mesh || !params.raycaster) {
      console.error('[MeshRaycast] Missing mesh or raycaster for mesh raycast');
      return null;
    }
    return raycastMesh(params.mesh, params.raycaster, params.far, params.maxDepth);
  } else {
    // WASM raycast
    if (!params.wasmCube || !params.origin || !params.direction) {
      console.error('[MeshRaycast] Missing parameters for WASM raycast');
      return null;
    }

    // Import and call WASM raycast
    // This would need the actual implementation
    // For now, return null as placeholder
    console.warn('[MeshRaycast] WASM raycast not yet integrated in unified interface');
    return null;
  }
}
