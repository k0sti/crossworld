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

  async initialize(): Promise<void> {
    await initializeWasm();
    this.engine = new GeometryEngine();
  }

  generateFrame(): GeometryData | null {
    if (!this.engine) {
      console.error('GeometryEngine not initialized');
      return null;
    }
    return this.engine.generate_frame();
  }

  setGroundRenderMode(useCube: boolean): void {
    if (!this.engine) {
      console.error('GeometryEngine not initialized');
      return;
    }
    this.engine.setGroundRenderMode(useCube);
  }

  getGroundRenderMode(): boolean {
    if (!this.engine) {
      console.error('GeometryEngine not initialized');
      return false;
    }
    return this.engine.getGroundRenderMode();
  }

  setVoxelAtDepth(x: number, y: number, z: number, depth: number, colorIndex: number): void {
    if (!this.engine) {
      console.error('GeometryEngine not initialized');
      return;
    }
    this.engine.setVoxelAtDepth(x, y, z, depth, colorIndex);
  }

  setVoxel(x: number, y: number, z: number, colorIndex: number): void {
    if (!this.engine) {
      console.error('GeometryEngine not initialized');
      return;
    }
    this.engine.setVoxel(x, y, z, colorIndex);
  }

  removeVoxelAtDepth(x: number, y: number, z: number, depth: number): void {
    if (!this.engine) {
      console.error('GeometryEngine not initialized');
      return;
    }
    this.engine.removeVoxelAtDepth(x, y, z, depth);
  }

  removeVoxel(x: number, y: number, z: number): void {
    if (!this.engine) {
      console.error('GeometryEngine not initialized');
      return;
    }
    this.engine.removeVoxel(x, y, z);
  }
}

export { GeometryEngine, GeometryData };
