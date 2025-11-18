import * as logger from '../utils/logger';
import { WorldCube, GeometryData } from 'crossworld-world';
import { getMacroDepth, getMicroDepth, getBorderDepth, getSeed } from '../config/depth-config';

/**
 * Operation types for the command queue
 */
type VoxelOperation =
  | { type: 'setVoxelAtDepth'; x: number; y: number; z: number; depth: number; colorIndex: number }
  | { type: 'removeVoxelAtDepth'; x: number; y: number; z: number; depth: number }
  | { type: 'exportCSM' }
  | { type: 'setRoot'; csmCode: string };

/**
 * CubeManager - Manages all operations on the cube world
 *
 * This manager ensures thread-safety by:
 * 1. Queuing all modification operations
 * 2. Processing operations only between mesh generations
 * 3. Preventing RefCell borrow conflicts
 */
export class CubeManager {
  private worldCube: WorldCube | null = null;
  private operationQueue: VoxelOperation[] = [];
  private macroDepth: number;
  private microDepth: number;
  private borderDepth: number;
  private seed: number;
  private isProcessing = false;

  constructor(
    macroDepth: number = getMacroDepth(),
    microDepth: number = getMicroDepth(),
    borderDepth: number = getBorderDepth(),
    seed: number = getSeed()
  ) {
    this.macroDepth = macroDepth;
    this.microDepth = microDepth;
    this.borderDepth = borderDepth;
    this.seed = seed;
    logger.log('geometry', `CubeManager created: macro=${macroDepth}, micro=${microDepth}, border=${borderDepth}, seed=${seed}`);
  }

  /**
   * Initialize the world cube
   */
  async initialize(): Promise<void> {
    logger.log('geometry', `[CubeManager] Initializing WorldCube: macro=${this.macroDepth}, micro=${this.microDepth}, border=${this.borderDepth}, seed=${this.seed}`);
    this.worldCube = new WorldCube(this.macroDepth, this.microDepth, this.borderDepth, this.seed);

    // Load material colors from materials.json
    await this.loadMaterialColors();

    logger.log('geometry', '[CubeManager] WorldCube initialized with seed:', this.seed);
  }

  /**
   * Load material colors from materials.json and set them in the WorldCube
   */
  private async loadMaterialColors(): Promise<void> {
    try {
      const response = await fetch('/crossworld/assets/materials.json');
      if (!response.ok) {
        logger.warn('geometry', `Failed to load materials.json: ${response.statusText}`);
        return;
      }

      const data = await response.json();

      // Convert materials to flat RGB array
      const colors: number[] = [];
      for (let i = 0; i < 128; i++) {
        const material = data.materials.find((m: any) => m.index === i);
        if (material) {
          // Parse hex color #AARRGGBB format (skip alpha channel)
          const hex = material.color.startsWith('#') ? material.color.substring(1) : material.color;
          const r = parseInt(hex.substring(2, 4), 16) / 255;
          const g = parseInt(hex.substring(4, 6), 16) / 255;
          const b = parseInt(hex.substring(6, 8), 16) / 255;
          colors.push(r, g, b);
        } else {
          // Default to black if material not found
          colors.push(0, 0, 0);
        }
      }

      if (this.worldCube) {
        this.worldCube.setMaterialColors(new Float32Array(colors));
        logger.log('geometry', `Loaded ${data.materials.length} material colors`);
        logger.log('geometry', `Sample colors - water(17): [${colors[17*3]}, ${colors[17*3+1]}, ${colors[17*3+2]}]`);
      }
    } catch (error) {
      logger.error('geometry', 'Failed to load material colors:', error);
    }
  }

  /**
   * Queue a voxel set operation
   */
  setVoxelAtDepth(x: number, y: number, z: number, depth: number, colorIndex: number): void {
    this.operationQueue.push({ type: 'setVoxelAtDepth', x, y, z, depth, colorIndex });
    logger.log('geometry', `Queued setVoxelAtDepth: (${x}, ${y}, ${z}) depth=${depth} color=${colorIndex}, queue length: ${this.operationQueue.length}`);
  }

  /**
   * Queue a voxel remove operation
   */
  removeVoxelAtDepth(x: number, y: number, z: number, depth: number): void {
    this.operationQueue.push({ type: 'removeVoxelAtDepth', x, y, z, depth });
    logger.log('geometry', `Queued removeVoxelAtDepth: (${x}, ${y}, ${z}) depth=${depth}, queue length: ${this.operationQueue.length}`);
  }

  /**
   * Export to CSM format
   * Processes all pending operations first, then exports
   */
  exportCSM(): string | null {
    // Process all pending operations before exporting
    this.processOperations();

    if (this.worldCube) {
      const csm = this.worldCube.exportToCSM();
      logger.log('geometry', `Exported world to CSM (${csm.length} chars)`);
      return csm;
    }

    return null;
  }

  /**
   * Queue a set root operation
   */
  setRoot(csmCode: string): void {
    this.operationQueue.push({ type: 'setRoot', csmCode });
    logger.log('geometry', `Queued setRoot: ${csmCode.length} chars`);
  }

  /**
   * Process all queued operations
   * This should be called BEFORE generating the mesh
   */
  private processOperations(): void {
    if (this.isProcessing || !this.worldCube) {
      return;
    }

    if (this.operationQueue.length === 0) {
      return;
    }

    this.isProcessing = true;
    const operationCount = this.operationQueue.length;

    try {
      logger.log('geometry', `Processing ${operationCount} queued operations...`);

      // Process all queued operations
      while (this.operationQueue.length > 0) {
        const operation = this.operationQueue.shift()!;

        switch (operation.type) {
          case 'setVoxelAtDepth':
            // @ts-ignore - WASM binding exists but TypeScript can't see it
            this.worldCube.setVoxelAtDepth(
              operation.x,
              operation.y,
              operation.z,
              operation.depth,
              operation.colorIndex
            );
            break;

          case 'removeVoxelAtDepth':
            // @ts-ignore - WASM binding exists but TypeScript can't see it
            this.worldCube.removeVoxelAtDepth(
              operation.x,
              operation.y,
              operation.z,
              operation.depth
            );
            break;

          case 'setRoot':
            this.worldCube.setRoot(operation.csmCode);
            break;

          case 'exportCSM':
            // Handled separately in exportCSM()
            break;
        }
      }

      logger.log('geometry', `Completed processing ${operationCount} operations`);
    } catch (error) {
      logger.error('geometry', 'Error processing operations:', error);
      // Clear the queue to prevent repeated errors
      this.operationQueue = [];
    } finally {
      this.isProcessing = false;
    }
  }

  /**
   * Generate mesh frame
   * Processes all queued operations first, then generates the mesh
   */
  generateFrame(): GeometryData | null {
    if (!this.worldCube) {
      logger.warn('geometry', 'Cannot generate frame: WorldCube not initialized');
      return null;
    }

    // Process all queued operations before generating mesh
    this.processOperations();

    // Now generate the mesh
    const data = this.worldCube.generateFrame();
    logger.log('geometry', `Generated mesh: ${data.vertices.length / 3} vertices, ${data.indices.length / 3} triangles`);
    return data;
  }

  /**
   * Check if there are pending operations
   */
  hasPendingOperations(): boolean {
    return this.operationQueue.length > 0;
  }

  /**
   * Get the number of pending operations
   */
  getPendingOperationCount(): number {
    return this.operationQueue.length;
  }
}
