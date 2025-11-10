import * as logger from '../utils/logger';
import { GeometryGenerator } from '../geometry/geometry-lib';
import { getMacroDepth, getMicroDepth } from '../config/depth-config';

export interface GeometryMessage {
  type: 'init' | 'update' | 'setVoxelAtDepth' | 'setVoxel' | 'removeVoxelAtDepth' | 'removeVoxel' | 'forceUpdate' | 'exportCSM';
  x?: number;
  y?: number;
  z?: number;
  depth?: number;
  colorIndex?: number;
  macroDepth?: number;
  microDepth?: number;
  borderDepth?: number;
  seed?: number;
}

export interface GeometryResult {
  vertices: Float32Array;
  indices: Uint32Array;
  normals: Float32Array;
  colors: Float32Array;
  uvs: Float32Array;
  materialIds: Uint8Array;
  stats: {
    vertices: number;
    triangles: number;
  };
  timestamp: number;
}

class GeometryWorkerManager {
  private generator: GeometryGenerator | null = null;
  private isRunning = false;
  private needsGeometryUpdate = false; // Flag to trigger mesh regeneration
  private updateInterval = 33; // ~30 FPS for geometry updates
  private lastUpdate = 0;
  private saveTimeout: ReturnType<typeof setTimeout> | null = null;
  private saveDebounceMs = 2000; // Save 2 seconds after last modification

  async initialize(macroDepth: number = getMacroDepth(), microDepth: number = 0, borderDepth: number = 0, seed: number = 0) {
    this.generator = new GeometryGenerator(macroDepth, microDepth, borderDepth, seed);
    await this.generator.initialize();
    self.postMessage({ type: 'ready' });
    // Generate initial geometry for the ground cube
    this.needsGeometryUpdate = true;
    this.startUpdateLoop();
  }

  private startUpdateLoop() {
    this.isRunning = true;
    this.update();
  }

  private update = () => {
    if (!this.isRunning || !this.generator) return;

    const now = performance.now();
    // Only regenerate mesh when needed (after voxel modifications)
    // CubeManager handles queuing, so it's safe to call generateGeometry
    if (this.needsGeometryUpdate && now - this.lastUpdate >= this.updateInterval) {
      this.generateGeometry();
      this.needsGeometryUpdate = false;
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
      const uvs = new Float32Array((geometryData as any).uvs || []);
      const materialIds = new Uint8Array((geometryData as any).materialIds || []);

      const result: GeometryResult = {
        vertices,
        indices,
        normals,
        colors,
        uvs,
        materialIds,
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
          colors.buffer,
          uvs.buffer,
          materialIds.buffer
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
      // CubeManager queues this operation
      this.generator.setVoxelAtDepth(x, y, z, depth, colorIndex);
      this.needsGeometryUpdate = true; // Request mesh regeneration
      this.scheduleAutoSave();
    }
  }

  setVoxel(x: number, y: number, z: number, colorIndex: number) {
    logger.log('worker', '[GeometryWorker] setVoxel', { x, y, z, colorIndex, hasGenerator: !!this.generator });
    if (this.generator) {
      // Use setVoxelAtDepth with the total depth (finest level of detail)
      const depth = getMacroDepth() + getMicroDepth();
      this.generator.setVoxelAtDepth(x, y, z, depth, colorIndex);
      this.needsGeometryUpdate = true; // Request mesh regeneration
      this.scheduleAutoSave();
    }
  }

  removeVoxelAtDepth(x: number, y: number, z: number, depth: number) {
    logger.log('worker', '[GeometryWorker] removeVoxelAtDepth', { x, y, z, depth, hasGenerator: !!this.generator });
    if (this.generator) {
      // CubeManager queues this operation
      this.generator.removeVoxelAtDepth(x, y, z, depth);
      this.needsGeometryUpdate = true; // Request mesh regeneration
      this.scheduleAutoSave();
    }
  }

  removeVoxel(x: number, y: number, z: number) {
    logger.log('worker', '[GeometryWorker] removeVoxel', { x, y, z, hasGenerator: !!this.generator });
    if (this.generator) {
      // Use removeVoxelAtDepth with the total depth (finest level of detail)
      const depth = getMacroDepth() + getMicroDepth();
      this.generator.removeVoxelAtDepth(x, y, z, depth);
      this.needsGeometryUpdate = true; // Request mesh regeneration
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

  forceUpdate() {
    this.needsGeometryUpdate = true;
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
      await manager.initialize(message.macroDepth, message.microDepth, message.borderDepth, message.seed);
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

    case 'forceUpdate':
      manager.forceUpdate();
      break;

    case 'exportCSM':
      manager.exportCSM();
      break;
  }
});
