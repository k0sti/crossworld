import * as THREE from 'three';

/**
 * Result from world raycast containing hit information
 */
export interface WorldRaycastResult {
  /** Hit point in world space */
  hitPoint: THREE.Vector3;
  /** Surface normal at hit point */
  normal: THREE.Vector3;
  /** Face center position (for avatar placement) */
  faceCenter: THREE.Vector3;
  /** Distance from ray origin */
  distance: number;
}

/**
 * Perform raycast against the world geometry mesh to find voxel face intersection
 *
 * This is specifically designed for teleportation, returning the face center
 * where the avatar should be placed.
 *
 * @param mesh - The geometry mesh to raycast against
 * @param raycaster - Three.js raycaster (already configured with ray)
 * @returns WorldRaycastResult or null if no hit
 */
export function raycastWorld(
  mesh: THREE.Mesh,
  raycaster: THREE.Raycaster
): WorldRaycastResult | null {
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

  // Transform normal from local to world space
  if (mesh.matrixWorld) {
    const normalMatrix = new THREE.Matrix3().getNormalMatrix(mesh.matrixWorld);
    hitNormal.applyMatrix3(normalMatrix).normalize();
  }

  // Calculate polygon area to determine voxel size
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
  const edge1 = new THREE.Vector3().subVectors(v1, v0);
  const edge2 = new THREE.Vector3().subVectors(v2, v0);
  const cross = new THREE.Vector3().crossVectors(edge1, edge2);
  const faceArea = cross.length() / 2;

  // Calculate voxel side length
  // For a cube face (square), two triangles make up the face
  // Each square face has area = sideLength^2
  // Each triangle has area = sideLength^2 / 2
  const sideLength = Math.sqrt(faceArea * 2);
  const halfSize = sideLength / 2;

  // Calculate voxel center by snapping hit point to voxel grid
  // Round to nearest voxel corner, then offset to center
  const voxelCorner = new THREE.Vector3(
    Math.floor(hitPoint.x / sideLength) * sideLength,
    Math.floor(hitPoint.y / sideLength) * sideLength,
    Math.floor(hitPoint.z / sideLength) * sideLength
  );

  const voxelCenter = voxelCorner.clone().addScalar(halfSize);

  // Calculate face center: voxel center offset by half size along normal
  const faceCenter = voxelCenter.clone().addScaledVector(hitNormal, halfSize);

  return {
    hitPoint,
    normal: hitNormal,
    faceCenter,
    distance: hit.distance
  };
}

/**
 * Calculate the avatar placement position on a voxel face
 *
 * The avatar should be placed on the face with proper orientation.
 * For ground faces (normal pointing up), place at face center.
 * For wall faces, place offset from the face.
 *
 * @param result - World raycast result
 * @param avatarOffset - Optional offset from face (default: 0)
 * @returns Position where avatar should be teleported
 */
export function calculateAvatarPlacement(
  result: WorldRaycastResult,
  avatarOffset: number = 0
): { x: number, z: number } {
  // For now, place avatar at the face center projected onto XZ plane
  // Apply optional offset along the normal
  const placement = result.faceCenter.clone();

  if (avatarOffset !== 0) {
    placement.addScaledVector(result.normal, avatarOffset);
  }

  return {
    x: placement.x,
    z: placement.z
  };
}
