import * as logger from '../utils/logger';
import { GeometryGenerator } from '../geometry/geometry-lib';
import { getMacroDepth } from '../config/depth-config';

export interface GeometryMessage {
  type: 'init' | 'update' | 'setVoxelAtDepth' | 'setVoxel' | 'removeVoxelAtDepth' | 'removeVoxel' | 'setFaceMeshMode' | 'setGroundRenderMode' | 'forceUpdate' | 'exportCSM';
  x?: number;
  y?: number;
  z?: number;
  depth?: number;
  colorIndex?: number;
  macroDepth?: number;
  microDepth?: number;
  enabled?: boolean;
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
  private saveTimeout: ReturnType<typeof setTimeout> | null = null;
  private saveDebounceMs = 2000; // Save 2 seconds after last modification

  async initialize(macroDepth: number = getMacroDepth(), microDepth: number = 0) {
    this.generator = new GeometryGenerator(macroDepth, microDepth);
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

  setVoxelAtDepth(x: number, y: number, z: number, depth: number, colorIndex: number) {
    logger.log('worker', '[GeometryWorker] setVoxelAtDepth', { x, y, z, depth, colorIndex, hasGenerator: !!this.generator });
    if (this.generator) {
      this.generator.setVoxelAtDepth(x, y, z, depth, colorIndex);
      this.scheduleAutoSave();
    }
  }

  setVoxel(x: number, y: number, z: number, colorIndex: number) {
    logger.log('worker', '[GeometryWorker] setVoxel', { x, y, z, colorIndex, hasGenerator: !!this.generator });
    if (this.generator) {
      this.generator.setVoxel(x, y, z, colorIndex);
      this.scheduleAutoSave();
    }
  }

  removeVoxelAtDepth(x: number, y: number, z: number, depth: number) {
    logger.log('worker', '[GeometryWorker] removeVoxelAtDepth', { x, y, z, depth, hasGenerator: !!this.generator });
    if (this.generator) {
      this.generator.removeVoxelAtDepth(x, y, z, depth);
      this.scheduleAutoSave();
    }
  }

  removeVoxel(x: number, y: number, z: number) {
    logger.log('worker', '[GeometryWorker] removeVoxel', { x, y, z, hasGenerator: !!this.generator });
    if (this.generator) {
      this.generator.removeVoxel(x, y, z);
      this.scheduleAutoSave();
    }
  }

  private scheduleAutoSave() {
    // Clear existing timeout
    if (this.saveTimeout) {
      clearTimeout(this.saveTimeout);
    }

    // Schedule new save
    this.saveTimeout = setTimeout(() => {
      this.saveWorld();
      this.saveTimeout = null;
    }, this.saveDebounceMs);
  }

  private saveWorld() {
    if (!this.generator) return;

    const csmText = this.generator.exportToCSM();
    if (csmText) {
      logger.log('worker', '[GeometryWorker] Exporting world to CSM...');
      logger.log('worker', '[GeometryWorker] CSM Preview:', csmText.substring(0, 200));

      // Send CSM data to main thread for download
      self.postMessage({ type: 'save-csm', csmText });
    }
  }

  setFaceMeshMode(enabled: boolean) {
    if (this.generator) {
      this.generator.setFaceMeshMode(enabled);
    }
  }

  setGroundRenderMode(useCube: boolean) {
    if (this.generator) {
      this.generator.setGroundRenderMode(useCube);
    }
  }

  forceUpdate() {
    this.generateGeometry();
  }

  exportCSM() {
    if (!this.generator) {
      self.postMessage({
        type: 'csm-export',
        error: 'Generator not initialized'
      });
      return;
    }

    try {
      const csmText = this.generator.exportToCSM();
      if (csmText) {
        self.postMessage({
          type: 'csm-export',
          csmText
        });
      } else {
        self.postMessage({
          type: 'csm-export',
          error: 'Failed to export CSM - no data returned'
        });
      }
    } catch (error) {
      self.postMessage({
        type: 'csm-export',
        error: error instanceof Error ? error.message : 'Unknown error'
      });
    }
  }
}

// Worker message handler
const manager = new GeometryWorkerManager();

self.addEventListener('message', async (event) => {
  const message = event.data as GeometryMessage;

  switch (message.type) {
    case 'init':
      await manager.initialize(message.macroDepth, message.microDepth);
      break;

    case 'update':
      // For now, we don't need to handle updates
      break;

    case 'setVoxelAtDepth':
      if (message.x !== undefined && message.y !== undefined && message.z !== undefined && message.depth !== undefined && message.colorIndex !== undefined) {
        manager.setVoxelAtDepth(message.x, message.y, message.z, message.depth, message.colorIndex);
      }
      break;

    case 'setVoxel':
      if (message.x !== undefined && message.y !== undefined && message.z !== undefined && message.colorIndex !== undefined) {
        manager.setVoxel(message.x, message.y, message.z, message.colorIndex);
      }
      break;

    case 'removeVoxelAtDepth':
      if (message.x !== undefined && message.y !== undefined && message.z !== undefined && message.depth !== undefined) {
        manager.removeVoxelAtDepth(message.x, message.y, message.z, message.depth);
      }
      break;

    case 'removeVoxel':
      if (message.x !== undefined && message.y !== undefined && message.z !== undefined) {
        manager.removeVoxel(message.x, message.y, message.z);
      }
      break;

    case 'setFaceMeshMode':
      if (message.enabled !== undefined) {
        manager.setFaceMeshMode(message.enabled);
      }
      break;

    case 'setGroundRenderMode':
      if (message.useCube !== undefined) {
        manager.setGroundRenderMode(message.useCube);
      }
      break;

    case 'forceUpdate':
      manager.forceUpdate();
      break;

    case 'exportCSM':
      manager.exportCSM();
      break;
  }
});
