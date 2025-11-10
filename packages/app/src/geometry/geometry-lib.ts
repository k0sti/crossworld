import * as logger from '../utils/logger';
import init, { WorldCube, GeometryData } from 'crossworld-world';
import { getMacroDepth, getMicroDepth, getBorderDepth } from '../config/depth-config';
import { CubeManager } from './cube-manager';

let wasmInitialized = false;
let initPromise: Promise<void> | null = null;

export async function initializeWasm(): Promise<void> {
  if (wasmInitialized) return;

  if (initPromise) {
    await initPromise;
    return;
  }

  initPromise = init().then(() => {
    wasmInitialized = true;
    logger.log('geometry', 'WASM module initialized');
  });

  await initPromise;
}

export class GeometryGenerator {
  private manager: CubeManager;

  constructor(macroDepth: number = getMacroDepth(), microDepth: number = getMicroDepth(), borderDepth: number = getBorderDepth()) {
    this.manager = new CubeManager(macroDepth, microDepth, borderDepth);
    logger.log('geometry', `GeometryGenerator created: macro=${macroDepth}, micro=${microDepth}, border=${borderDepth}`);
  }

  async initialize(): Promise<void> {
    await initializeWasm();
    await this.manager.initialize();
    logger.log('geometry', 'GeometryGenerator initialized');
  }

  generateFrame(): GeometryData | null {
    return this.manager.generateFrame();
  }

  setVoxelAtDepth(x: number, y: number, z: number, depth: number, colorIndex: number): void {
    this.manager.setVoxelAtDepth(x, y, z, depth, colorIndex);
  }

  removeVoxelAtDepth(x: number, y: number, z: number, depth: number): void {
    this.manager.removeVoxelAtDepth(x, y, z, depth);
  }

  exportToCSM(): string | null {
    return this.manager.exportCSM();
  }

  // New unified interface methods
  root(): string | null {
    return this.exportToCSM();
  }

  setRoot(csmCode: string): void {
    this.manager.setRoot(csmCode);
  }

  // Check if there are pending operations
  hasPendingOperations(): boolean {
    return this.manager.hasPendingOperations();
  }
}

// Export WorldCube as GeometryEngine for backward compatibility
export { WorldCube as GeometryEngine, GeometryData };
