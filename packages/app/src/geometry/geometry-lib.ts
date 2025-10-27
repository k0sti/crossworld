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
      console.error('GeometryEngine not initialized');
      return null;
    }
    return this.engine.generate_frame();
  }

  setVoxelAtDepth(x: number, y: number, z: number, depth: number, colorIndex: number): void {
    console.log('[GeometryLib] setVoxelAtDepth', { x, y, z, depth, colorIndex, hasEngine: !!this.engine });
    if (!this.engine) {
      console.error('GeometryEngine not initialized');
      return;
    }
    // @ts-ignore - WASM binding exists but TypeScript can't see it
    this.engine.setVoxelAtDepth(x, y, z, depth, colorIndex);
    console.log('[GeometryLib] setVoxelAtDepth completed');
  }

  setVoxel(x: number, y: number, z: number, colorIndex: number): void {
    console.log('[GeometryLib] setVoxel', { x, y, z, colorIndex, hasEngine: !!this.engine });
    if (!this.engine) {
      console.error('GeometryEngine not initialized');
      return;
    }
    this.engine.setVoxel(x, y, z, colorIndex);
    console.log('[GeometryLib] setVoxel completed');
  }

  removeVoxelAtDepth(x: number, y: number, z: number, depth: number): void {
    console.log('[GeometryLib] removeVoxelAtDepth', { x, y, z, depth, hasEngine: !!this.engine });
    if (!this.engine) {
      console.error('GeometryEngine not initialized');
      return;
    }
    // @ts-ignore - WASM binding exists but TypeScript can't see it
    this.engine.removeVoxelAtDepth(x, y, z, depth);
    console.log('[GeometryLib] removeVoxelAtDepth completed');
  }

  removeVoxel(x: number, y: number, z: number): void {
    console.log('[GeometryLib] removeVoxel', { x, y, z, hasEngine: !!this.engine });
    if (!this.engine) {
      console.error('GeometryEngine not initialized');
      return;
    }
    this.engine.removeVoxel(x, y, z);
    console.log('[GeometryLib] removeVoxel completed');
  }

  setFaceMeshMode(enabled: boolean): void {
    console.log('[GeometryLib] setFaceMeshMode', { enabled, hasEngine: !!this.engine });
    if (!this.engine) {
      console.error('GeometryEngine not initialized');
      return;
    }
    // @ts-ignore - WASM binding exists but TypeScript can't see it
    this.engine.setFaceMeshMode(enabled);
    console.log('[GeometryLib] setFaceMeshMode completed');
  }

  setGroundRenderMode(useCube: boolean): void {
    console.log('[GeometryLib] setGroundRenderMode', { useCube, hasEngine: !!this.engine });
    if (!this.engine) {
      console.error('GeometryEngine not initialized');
      return;
    }
    // @ts-ignore - WASM binding exists but TypeScript can't see it
    this.engine.setGroundRenderMode(useCube);
    console.log('[GeometryLib] setGroundRenderMode completed');
  }
}

export { GeometryEngine, GeometryData };
