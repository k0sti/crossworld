import * as logger from '../utils/logger';
import init, { WorldCube, GeometryData } from '@workspace/wasm';
import { getMacroDepth, getMicroDepth, getBorderDepth } from '../config/depth-config';

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
  private engine: WorldCube | null = null;
  private macroDepth: number;
  private microDepth: number;
  private borderDepth: number;

  constructor(macroDepth: number = getMacroDepth(), microDepth: number = getMicroDepth(), borderDepth: number = getBorderDepth()) {
    this.macroDepth = macroDepth;
    this.microDepth = microDepth;
    this.borderDepth = borderDepth;
    logger.log('geometry', `GeometryGenerator created: macro=${macroDepth}, micro=${microDepth}, border=${borderDepth}`);
  }

  async initialize(): Promise<void> {
    await initializeWasm();
    logger.log('geometry', `Initializing WorldCube: macro=${this.macroDepth}, micro=${this.microDepth}, border=${this.borderDepth}`);
    this.engine = new WorldCube(this.macroDepth, this.microDepth, this.borderDepth);
    logger.log('geometry', 'WorldCube initialized');
  }

  generateFrame(): GeometryData | null {
    if (!this.engine) {
      logger.warn('geometry', 'Cannot generate frame: engine not initialized');
      return null;
    }
    const data = this.engine.generateFrame();
    logger.log('geometry', `Generated mesh: ${data.vertices.length / 3} vertices, ${data.indices.length / 3} triangles`);
    return data;
  }

  setVoxelAtDepth(x: number, y: number, z: number, depth: number, colorIndex: number): void {
    if (!this.engine) {
      logger.warn('geometry', 'Cannot set voxel: engine not initialized');
      return;
    }
    // @ts-ignore - WASM binding exists but TypeScript can't see it
    this.engine.setVoxelAtDepth(x, y, z, depth, colorIndex);
    logger.log('geometry', `Set voxel at (${x}, ${y}, ${z}) depth=${depth} color=${colorIndex}`);
  }

  setVoxel(x: number, y: number, z: number, colorIndex: number): void {
    if (!this.engine) {
      logger.warn('geometry', 'Cannot set voxel: engine not initialized');
      return;
    }
    this.engine.setVoxel(x, y, z, colorIndex);
    logger.log('geometry', `Set voxel at (${x}, ${y}, ${z}) color=${colorIndex}`);
  }

  removeVoxelAtDepth(x: number, y: number, z: number, depth: number): void {
    if (!this.engine) {
      logger.warn('geometry', 'Cannot remove voxel: engine not initialized');
      return;
    }
    // @ts-ignore - WASM binding exists but TypeScript can't see it
    this.engine.removeVoxelAtDepth(x, y, z, depth);
    logger.log('geometry', `Removed voxel at (${x}, ${y}, ${z}) depth=${depth}`);
  }

  removeVoxel(x: number, y: number, z: number): void {
    if (!this.engine) {
      logger.warn('geometry', 'Cannot remove voxel: engine not initialized');
      return;
    }
    this.engine.removeVoxel(x, y, z);
    logger.log('geometry', `Removed voxel at (${x}, ${y}, ${z})`);
  }

  exportToCSM(): string | null {
    if (!this.engine) {
      logger.warn('geometry', 'Cannot export CSM: engine not initialized');
      return null;
    }
    const csm = this.engine.exportToCSM();
    logger.log('geometry', `Exported world to CSM (${csm.length} chars)`);
    return csm;
  }

  // New unified interface methods
  root(): string | null {
    if (!this.engine) {
      logger.warn('geometry', 'Cannot get root: engine not initialized');
      return null;
    }
    return this.engine.root();
  }

  setRoot(csmCode: string): void {
    if (!this.engine) {
      logger.warn('geometry', 'Cannot set root: engine not initialized');
      return;
    }
    this.engine.setRoot(csmCode);
    logger.log('geometry', `Loaded world from CSM (${csmCode.length} chars)`);
  }
}

// Export WorldCube as GeometryEngine for backward compatibility
export { WorldCube as GeometryEngine, GeometryData };
