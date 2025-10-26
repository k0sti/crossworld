import init, { GeometryEngine, GeometryData } from '@workspace/wasm';

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
  private worldDepth: number;
  private scaleDepth: number;

  constructor(worldDepth: number = 5, scaleDepth: number = 1) {
    this.worldDepth = worldDepth;
    this.scaleDepth = scaleDepth;
  }

  async initialize(): Promise<void> {
    await initializeWasm();
    this.engine = new GeometryEngine(this.worldDepth, this.scaleDepth);
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
}

export { GeometryEngine, GeometryData };
