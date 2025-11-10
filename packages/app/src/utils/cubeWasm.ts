import * as logger from './logger';
import initWasm from 'cube';

let initialized = false;

/**
 * Initialize the cube WASM module (idempotent)
 */
export async function ensureCubeWasmInitialized(): Promise<void> {
  if (!initialized) {
    await initWasm();
    initialized = true;
    logger.log('common', '[CubeWasm] WASM module initialized');
  }
}

/**
 * Parse CSM code and generate mesh data for avatars
 *
 * Avatars use a simple cube representation without border layers.
 * max_depth = 4 means 2^4 = 16 unit cube, suitable for typical 16x16x16 voxel avatars.
 *
 * @param csmCode - Cubescript code to parse
 * @param maxDepth - Maximum octree depth (default 4 for 16x16x16 avatars)
 */
export async function parseCsmToMesh(csmCode: string, maxDepth: number = 3) {
  await ensureCubeWasmInitialized();
  const wasmModule = await import('cube');

  // Load CSM into a WasmCube
  // @ts-ignore - WASM module exports loadCsm
  const cube = wasmModule.loadCsm(csmCode);

  // Generate mesh from the cube
  // - null palette = use HSV color mapping
  // - maxDepth = total octree depth (no separate border layers for avatars)
  return cube.generateMesh(null, maxDepth);
}

/**
 * Validate CSM code without generating mesh
 */
export async function validateCsm(csmCode: string) {
  await ensureCubeWasmInitialized();
  const wasmModule = await import('cube');
  // @ts-ignore - WASM module exports validateCsm
  return wasmModule.validateCsm(csmCode);
}

/**
 * Load a CSM model as a WasmCube for raycasting
 */
export async function loadCubeFromCsm(csmText: string): Promise<any | null> {
  await ensureCubeWasmInitialized();
  try {
    // Dynamic import to access WASM functions
    // @ts-ignore - WASM module exports not fully typed
    const wasmModule = await import('cube');
    // @ts-ignore - WASM loadCsm export
    return wasmModule.loadCsm(csmText);
  } catch (error) {
    logger.error('common', '[CubeWasm] Failed to load CSM:', error);
    return null;
  }
}

/**
 * Raycast through a WasmCube
 *
 * @param wasmCube - The WasmCube instance to raycast through
 * @param posX, posY, posZ - Ray origin in normalized [0,1] space
 * @param dirX, dirY, dirZ - Ray direction (should be normalized)
 * @param far - If true, returns position on far side of contact plane
 * @param maxDepth - Maximum octree depth to traverse
 * @returns RaycastResult or null if no hit
 */
export function raycastWasm(
  wasmCube: any,
  posX: number,
  posY: number,
  posZ: number,
  dirX: number,
  dirY: number,
  dirZ: number,
  far: boolean,
  maxDepth: number
): any {
  if (!initialized) {
    logger.error('common', '[CubeWasm] Attempted to call raycastWasm before WASM initialization');
    return null;
  }

  try {
    return wasmCube.raycast(posX, posY, posZ, dirX, dirY, dirZ, far, maxDepth);
  } catch (error) {
    logger.error('common', '[CubeWasm] Raycast failed:', error);
    return null;
  }
}
