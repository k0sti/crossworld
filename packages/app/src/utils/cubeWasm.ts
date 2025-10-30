import * as logger from './logger';
import initWasm from '@workspace/wasm-cube';
import * as cube from '@workspace/wasm-cube';

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
  return cube.parse_csm_to_mesh(csmCode);
}

/**
 * Validate CSM code without generating mesh
 */
export async function validateCsm(csmCode: string) {
  await ensureCubeWasmInitialized();
  return cube.validate_csm(csmCode);
}

/**
 * Load a CSM model into Rust MODEL_STORAGE for raycasting
 */
export async function loadModelFromCsm(modelId: string, csmText: string, maxDepth: number) {
  await ensureCubeWasmInitialized();
  // @ts-expect-error - WASM exports not fully typed in generated .d.ts
  return cube.load_model_from_csm(modelId, csmText, maxDepth);
}

/**
 * Raycast through octree using aether implementation (async, checks init)
 */
export async function raycastAether(
  modelId: string,
  posX: number,
  posY: number,
  posZ: number,
  dirX: number,
  dirY: number,
  dirZ: number
) {
  await ensureCubeWasmInitialized();
  // @ts-expect-error - WASM exports not fully typed in generated .d.ts
  return cube.raycast_aether(modelId, posX, posY, posZ, dirX, dirY, dirZ);
}

/**
 * Raycast through octree using aether implementation (synchronous, assumes WASM is initialized)
 */
export function raycastAetherSync(
  modelId: string,
  posX: number,
  posY: number,
  posZ: number,
  dirX: number,
  dirY: number,
  dirZ: number
) {
  if (!initialized) {
    logger.error('common', '[CubeWasm] Attempted to call raycastAetherSync before WASM initialization');
    return null;
  }
  // @ts-expect-error - WASM exports not fully typed in generated .d.ts
  return cube.raycast_aether(modelId, posX, posY, posZ, dirX, dirY, dirZ);
}
