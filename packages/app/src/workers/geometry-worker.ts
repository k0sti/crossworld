import { GeometryGenerator } from '../geometry/geometry-lib';

export interface GeometryMessage {
  type: 'init' | 'update' | 'setGroundRenderMode';
  useCube?: boolean;
}

export interface GeometryResult {
  vertices: Float32Array;
  indices: Uint32Array;
  normals: Float32Array;
  colors: Float32Array;
  stats: {
    vertices: number;
    triangles: number;
  };
  timestamp: number;
}

class GeometryWorkerManager {
  private generator: GeometryGenerator | null = null;
  private isRunning = false;
  private updateInterval = 33; // ~30 FPS for geometry updates
  private lastUpdate = 0;

  async initialize() {
    this.generator = new GeometryGenerator();
    await this.generator.initialize();
    self.postMessage({ type: 'ready' });
    this.startUpdateLoop();
  }

  private startUpdateLoop() {
    this.isRunning = true;
    this.update();
  }

  private update = () => {
    if (!this.isRunning || !this.generator) return;

    const now = performance.now();
    if (now - this.lastUpdate >= this.updateInterval) {
      this.generateGeometry();
      this.lastUpdate = now;
    }

    setTimeout(this.update, 16); // Check roughly 60 times per second
  }

  private generateGeometry() {
    if (!this.generator) return;

    const geometryData = this.generator.generateFrame();

    if (geometryData) {
      const vertices = new Float32Array(geometryData.vertices);
      const indices = new Uint32Array(geometryData.indices);
      const normals = new Float32Array(geometryData.normals);
      const colors = new Float32Array(geometryData.colors);

      const result: GeometryResult = {
        vertices,
        indices,
        normals,
        colors,
        stats: {
          vertices: vertices.length / 3,
          triangles: indices.length / 3
        },
        timestamp: performance.now()
      };

      // Transfer ownership of the buffers to main thread
      self.postMessage({ type: 'geometry', data: result }, {
        transfer: [
          vertices.buffer,
          indices.buffer,
          normals.buffer,
          colors.buffer
        ]
      });
    }
  }

  stop() {
    this.isRunning = false;
  }

  setGroundRenderMode(useCube: boolean) {
    if (this.generator) {
      this.generator.setGroundRenderMode(useCube);
    }
  }
}

// Worker message handler
const manager = new GeometryWorkerManager();

self.addEventListener('message', async (event) => {
  const message = event.data as GeometryMessage;

  switch (message.type) {
    case 'init':
      await manager.initialize();
      break;

    case 'update':
      // For now, we don't need to handle updates
      break;

    case 'setGroundRenderMode':
      if (message.useCube !== undefined) {
        manager.setGroundRenderMode(message.useCube);
      }
      break;
  }
});
