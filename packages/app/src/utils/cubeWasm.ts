import initWasm, { parse_csm_to_mesh, validate_csm } from '@workspace/wasm-cube';

let initialized = false;

/**
 * Initialize the cube WASM module (idempotent)
 */
export async function ensureCubeWasmInitialized(): Promise<void> {
  if (!initialized) {
    await initWasm();
    initialized = true;
    console.log('[CubeWasm] WASM module initialized');
  }
}

/**
 * Parse CSM code and generate mesh data
 */
export async function parseCsmToMesh(csmCode: string) {
  await ensureCubeWasmInitialized();
  return parse_csm_to_mesh(csmCode);
}

/**
 * Validate CSM code without generating mesh
 */
export async function validateCsm(csmCode: string) {
  await ensureCubeWasmInitialized();
  return validate_csm(csmCode);
}
