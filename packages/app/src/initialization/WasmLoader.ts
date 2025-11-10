/**
 * WASM Module Loader
 *
 * Coordinates loading of all WASM modules required by the app
 */

import initPhysicsWasm from '../../../wasm-physics/crossworld_physics.js';
import { ensureCubeWasmInitialized } from '../utils/cubeWasm';
import * as logger from '../utils/logger';

export interface WasmModules {
  cubeLoaded: boolean;
  physicsLoaded: boolean;
}

let cubeWasmInitialized = false;
let physicsWasmInitialized = false;

/**
 * Load all WASM modules in parallel
 *
 * @returns Promise that resolves when all modules are loaded
 */
export async function loadAllWasmModules(): Promise<WasmModules> {
  logger.log('common', '[WasmLoader] Starting WASM module loading...');

  const startTime = performance.now();

  // Load both WASM modules in parallel
  const [cubeResult, physicsResult] = await Promise.allSettled([
    loadCubeWasm(),
    loadPhysicsWasm(),
  ]);

  const cubeLoaded = cubeResult.status === 'fulfilled';
  const physicsLoaded = physicsResult.status === 'fulfilled';

  if (!cubeLoaded) {
    logger.error('common', '[WasmLoader] Cube WASM failed to load:', cubeResult.reason);
    throw new Error('Failed to load cube WASM module: ' + cubeResult.reason);
  }

  if (!physicsLoaded) {
    logger.error('common', '[WasmLoader] Physics WASM failed to load:', physicsResult.reason);
    throw new Error('Failed to load physics WASM module: ' + physicsResult.reason);
  }

  const elapsed = performance.now() - startTime;
  logger.log('common', `[WasmLoader] All WASM modules loaded in ${elapsed.toFixed(0)}ms`);

  return { cubeLoaded, physicsLoaded };
}

/**
 * Load cube geometry WASM module
 */
async function loadCubeWasm(): Promise<void> {
  if (cubeWasmInitialized) {
    logger.log('common', '[WasmLoader] Cube WASM already initialized');
    return;
  }

  const startTime = performance.now();
  await ensureCubeWasmInitialized();
  cubeWasmInitialized = true;

  const elapsed = performance.now() - startTime;
  logger.log('common', `[WasmLoader] Cube WASM loaded in ${elapsed.toFixed(0)}ms`);
}

/**
 * Load physics WASM module
 */
async function loadPhysicsWasm(): Promise<void> {
  if (physicsWasmInitialized) {
    logger.log('common', '[WasmLoader] Physics WASM already initialized');
    return;
  }

  const startTime = performance.now();
  await initPhysicsWasm();
  physicsWasmInitialized = true;

  const elapsed = performance.now() - startTime;
  logger.log('common', `[WasmLoader] Physics WASM loaded in ${elapsed.toFixed(0)}ms`);
}

/**
 * Check if cube WASM is initialized
 */
export function isCubeWasmInitialized(): boolean {
  return cubeWasmInitialized;
}

/**
 * Check if physics WASM is initialized
 */
export function isPhysicsWasmInitialized(): boolean {
  return physicsWasmInitialized;
}

/**
 * Check if all WASM modules are initialized
 */
export function areAllWasmModulesInitialized(): boolean {
  return cubeWasmInitialized && physicsWasmInitialized;
}
