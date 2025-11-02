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
    logger.log('geometry', `GeometryGenerator created with macro=${macroDepth}, micro=${microDepth}, border=${borderDepth}`);
  }

  async initialize(): Promise<void> {
    await initializeWasm();
    logger.log('geometry', `Creating WorldCube with macro=${this.macroDepth}, micro=${this.microDepth}, border=${this.borderDepth}`);
    this.engine = new WorldCube(this.macroDepth, this.microDepth, this.borderDepth);
    logger.log('geometry', 'WorldCube created successfully');
  }

  generateFrame(): GeometryData | null {
    if (!this.engine) {
      logger.warn('geometry', 'generateFrame called but engine is null');
      return null;
    }
    logger.log('geometry', 'Generating frame...');
    const data = this.engine.generateFrame();
    logger.log('geometry', `Frame generated: ${data.vertices.length} vertices, ${data.indices.length} indices`);
    return data;
  }

  setVoxelAtDepth(x: number, y: number, z: number, depth: number, colorIndex: number): void {
    if (!this.engine) {
      return;
    }
    // @ts-ignore - WASM binding exists but TypeScript can't see it
    this.engine.setVoxelAtDepth(x, y, z, depth, colorIndex);
  }

  setVoxel(x: number, y: number, z: number, colorIndex: number): void {
    if (!this.engine) {
      return;
    }
    this.engine.setVoxel(x, y, z, colorIndex);
  }

  removeVoxelAtDepth(x: number, y: number, z: number, depth: number): void {
    if (!this.engine) {
      return;
    }
    // @ts-ignore - WASM binding exists but TypeScript can't see it
    this.engine.removeVoxelAtDepth(x, y, z, depth);
  }

  removeVoxel(x: number, y: number, z: number): void {
    if (!this.engine) {
      return;
    }
    this.engine.removeVoxel(x, y, z);
  }

  exportToCSM(): string | null {
    if (!this.engine) {
      return null;
    }
    return this.engine.exportToCSM();
  }

  // New unified interface methods
  root(): string | null {
    if (!this.engine) {
      return null;
    }
    return this.engine.root();
  }

  setRoot(csmCode: string): void {
    if (!this.engine) {
      return;
    }
    this.engine.setRoot(csmCode);
  }
}

// Export WorldCube as GeometryEngine for backward compatibility
export { WorldCube as GeometryEngine, GeometryData };
