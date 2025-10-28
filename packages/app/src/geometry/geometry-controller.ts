import * as logger from '../utils/logger';
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
    logger.log('geometry', '[GeometryController] CSM update received');

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

  getMacroDepth(): number {
    return this.macroDepth;
  }

  getMicroDepth(): number {
    return this.microDepth;
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
    logger.log('geometry', '[GeometryController] setVoxelAtDepth', { x, y, z, depth, colorIndex, hasWorker: !!this.worker });
    if (this.worker) {
      this.worker.postMessage({ type: 'setVoxelAtDepth', x, y, z, depth, colorIndex });
    }
  }

  setVoxel(x: number, y: number, z: number, colorIndex: number) {
    logger.log('geometry', '[GeometryController] setVoxel', { x, y, z, colorIndex, hasWorker: !!this.worker });
    if (this.worker) {
      this.worker.postMessage({ type: 'setVoxel', x, y, z, colorIndex });
    }
  }

  removeVoxelAtDepth(x: number, y: number, z: number, depth: number) {
    logger.log('geometry', '[GeometryController] removeVoxelAtDepth', { x, y, z, depth, hasWorker: !!this.worker });
    if (this.worker) {
      this.worker.postMessage({ type: 'removeVoxelAtDepth', x, y, z, depth });
    }
  }

  removeVoxel(x: number, y: number, z: number) {
    logger.log('geometry', '[GeometryController] removeVoxel', { x, y, z, hasWorker: !!this.worker });
    if (this.worker) {
      this.worker.postMessage({ type: 'removeVoxel', x, y, z });
    }
  }

  forceUpdate() {
    logger.log('geometry', '[GeometryController] forceUpdate - triggering mesh regeneration');
    if (this.worker) {
      this.worker.postMessage({ type: 'forceUpdate' });
    }
  }

  async getCSM(): Promise<string> {
    return new Promise((resolve, reject) => {
      if (!this.worker) {
        reject(new Error('Worker not initialized'));
        return;
      }

      const handler = (event: MessageEvent) => {
        if (event.data.type === 'csm-export') {
          this.worker?.removeEventListener('message', handler);
          if (event.data.error) {
            reject(new Error(event.data.error));
          } else {
            resolve(event.data.csmText);
          }
        }
      };

      this.worker.addEventListener('message', handler);
      this.worker.postMessage({ type: 'exportCSM' });

      // Timeout after 5 seconds
      setTimeout(() => {
        this.worker?.removeEventListener('message', handler);
        reject(new Error('CSM export timeout'));
      }, 5000);
    });
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
