import * as logger from './logger';
import initWasm from '@workspace/wasm-cube';

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
 * Parse CSM code and generate mesh data
 */
export async function parseCsmToMesh(csmCode: string) {
  await ensureCubeWasmInitialized();
  const wasmModule = await import('@workspace/wasm-cube');
  return wasmModule.parse_csm_to_mesh(csmCode);
}

/**
 * Validate CSM code without generating mesh
 */
export async function validateCsm(csmCode: string) {
  await ensureCubeWasmInitialized();
  const wasmModule = await import('@workspace/wasm-cube');
  return wasmModule.validate_csm(csmCode);
}

/**
 * Load a CSM model as a WasmCube for raycasting
 */
export async function loadCubeFromCsm(csmText: string): Promise<any | null> {
  await ensureCubeWasmInitialized();
  try {
    // Dynamic import to access WASM functions
    // @ts-ignore - WASM module exports not fully typed
    const wasmModule = await import('@workspace/wasm-cube');
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
