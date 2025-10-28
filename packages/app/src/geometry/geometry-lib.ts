import init, { GeometryEngine, GeometryData } from '@workspace/wasm';
import { getMacroDepth, getMicroDepth } from '../config/depth-config';

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
    console.log('WASM module initialized');
  });

  await initPromise;
}

export class GeometryGenerator {
  private engine: GeometryEngine | null = null;
  private macroDepth: number;
  private microDepth: number;

  constructor(macroDepth: number = getMacroDepth(), microDepth: number = getMicroDepth()) {
    this.macroDepth = macroDepth;
    this.microDepth = microDepth;
  }

  async initialize(): Promise<void> {
    await initializeWasm();
    this.engine = new GeometryEngine(this.macroDepth, this.microDepth);
  }

  generateFrame(): GeometryData | null {
    if (!this.engine) {
      return null;
    }
    return this.engine.generate_frame();
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

  setFaceMeshMode(enabled: boolean): void {
    if (!this.engine) {
      return;
    }
    // @ts-ignore - WASM binding exists but TypeScript can't see it
    this.engine.setFaceMeshMode(enabled);
  }

  setGroundRenderMode(useCube: boolean): void {
    if (!this.engine) {
      return;
    }
    // @ts-ignore - WASM binding exists but TypeScript can't see it
    this.engine.setGroundRenderMode(useCube);
  }

  exportToCSM(): string | null {
    if (!this.engine) {
      return null;
    }
    // @ts-ignore - WASM binding exists but TypeScript can't see it
    return this.engine.exportToCSM();
  }
}

export { GeometryEngine, GeometryData };
