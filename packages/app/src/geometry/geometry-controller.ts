import { GeometryResult } from '../workers/geometry-worker';

export class GeometryController {
  private worker: Worker | null = null;
  private latestGeometry: GeometryResult | null = null;
  private stats = { vertices: 0, triangles: 0 };
  private onGeometryUpdate?: (geometry: GeometryResult) => void;

  constructor() {}

  async initialize(onGeometryUpdate: (geometry: GeometryResult) => void): Promise<void> {
    this.onGeometryUpdate = onGeometryUpdate;

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
        }
      });

      this.worker.postMessage({ type: 'init' });
    });
  }

  private handleGeometryUpdate(geometry: GeometryResult) {
    this.latestGeometry = geometry;
    this.stats = geometry.stats;

    if (this.onGeometryUpdate) {
      this.onGeometryUpdate(geometry);
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

  destroy() {
    if (this.worker) {
      this.worker.terminate();
      this.worker = null;
    }
  }
}
