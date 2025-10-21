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
}

export { GeometryEngine, GeometryData };
