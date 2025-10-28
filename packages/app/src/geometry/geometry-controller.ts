import { GeometryResult } from '../workers/geometry-worker';
import { getMacroDepth, getMicroDepth } from '../config/depth-config';

export class GeometryController {
  private worker: Worker | null = null;
  private latestGeometry: GeometryResult | null = null;
  private stats = { vertices: 0, triangles: 0 };
  private onGeometryUpdate?: (geometry: GeometryResult) => void;
  private onCSMUpdate?: (csmText: string) => void;
  private macroDepth: number;
  private microDepth: number;

  constructor(macroDepth: number = getMacroDepth(), microDepth: number = getMicroDepth()) {
    this.macroDepth = macroDepth;
    this.microDepth = microDepth;
  }

  async initialize(
    onGeometryUpdate: (geometry: GeometryResult) => void,
    onCSMUpdate?: (csmText: string) => void
  ): Promise<void> {
    this.onGeometryUpdate = onGeometryUpdate;
    this.onCSMUpdate = onCSMUpdate;

    return new Promise((resolve) => {
      this.worker = new Worker(
        new URL('../workers/geometry-worker.ts', import.meta.url),
        { type: 'module' }
      );

      this.worker.addEventListener('message', (event) => {
        if (event.data.type === 'ready') {
          console.log('Geometry worker initialized');
          resolve();
        } else if (event.data.type === 'geometry') {
          this.handleGeometryUpdate(event.data.data);
        } else if (event.data.type === 'save-csm') {
          this.handleSaveCSM(event.data.csmText);
        }
      });

      this.worker.postMessage({ type: 'init', macroDepth: this.macroDepth, microDepth: this.microDepth });
    });
  }

  private handleGeometryUpdate(geometry: GeometryResult) {
    this.latestGeometry = geometry;
    this.stats = geometry.stats;

    if (this.onGeometryUpdate) {
      this.onGeometryUpdate(geometry);
    }
  }

  private handleSaveCSM(csmText: string) {
    console.log('[GeometryController] CSM update received');

    if (this.onCSMUpdate) {
      this.onCSMUpdate(csmText);
    }
  }

  getLatestGeometry(): GeometryResult | null {
    const geometry = this.latestGeometry;
    this.latestGeometry = null; // Clear after retrieving
    return geometry;
  }

  getStats() {
    return this.stats;
  }

  setGroundRenderMode(useCube: boolean) {
    if (this.worker) {
      this.worker.postMessage({ type: 'setGroundRenderMode', useCube });
    }
  }

  setFaceMeshMode(enabled: boolean) {
    if (this.worker) {
      this.worker.postMessage({ type: 'setFaceMeshMode', enabled });
    }
  }

  setVoxelAtDepth(x: number, y: number, z: number, depth: number, colorIndex: number) {
    console.log('[GeometryController] setVoxelAtDepth', { x, y, z, depth, colorIndex, hasWorker: !!this.worker });
    if (this.worker) {
      this.worker.postMessage({ type: 'setVoxelAtDepth', x, y, z, depth, colorIndex });
    }
  }

  setVoxel(x: number, y: number, z: number, colorIndex: number) {
    console.log('[GeometryController] setVoxel', { x, y, z, colorIndex, hasWorker: !!this.worker });
    if (this.worker) {
      this.worker.postMessage({ type: 'setVoxel', x, y, z, colorIndex });
    }
  }

  removeVoxelAtDepth(x: number, y: number, z: number, depth: number) {
    console.log('[GeometryController] removeVoxelAtDepth', { x, y, z, depth, hasWorker: !!this.worker });
    if (this.worker) {
      this.worker.postMessage({ type: 'removeVoxelAtDepth', x, y, z, depth });
    }
  }

  removeVoxel(x: number, y: number, z: number) {
    console.log('[GeometryController] removeVoxel', { x, y, z, hasWorker: !!this.worker });
    if (this.worker) {
      this.worker.postMessage({ type: 'removeVoxel', x, y, z });
    }
  }

  async reinitialize(macroDepth: number, microDepth: number, onGeometryUpdate: (geometry: GeometryResult) => void): Promise<void> {
    // Terminate existing worker
    this.destroy();

    // Update depths
    this.macroDepth = macroDepth;
    this.microDepth = microDepth;

    // Reinitialize with new depths
    await this.initialize(onGeometryUpdate);
  }

  destroy() {
    if (this.worker) {
      this.worker.terminate();
      this.worker = null;
    }
  }
}
