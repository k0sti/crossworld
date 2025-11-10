/**
 * App Initializer
 *
 * Single initialization orchestrator for the Crossworld app.
 * Manages all subsystems: WASM, physics world, renderer, and scene.
 */

import * as THREE from 'three';
import { World } from './physics/world';
import { SceneManager } from './renderer/scene';
import { ensureCubeWasmInitialized } from './utils/cubeWasm';
import initPhysicsWasm from '../../wasm-physics/crossworld_physics.js';
import * as logger from './utils/logger';

/**
 * Initialization phases
 */
export type InitPhase = 'idle' | 'wasm' | 'rendering' | 'ready' | 'error';

/**
 * Initialization state
 */
export interface InitState {
  phase: InitPhase;
  progress: number; // 0-100
  message: string;
  error?: Error;
}

/**
 * State change callback
 */
export type StateCallback = (state: InitState) => void;

/**
 * AppInitializer - Single source of truth for app initialization
 *
 * Manages initialization phases:
 * 1. WASM modules (cube + physics) - parallel
 * 2. Rendering infrastructure (renderer, scene, physics world)
 * 3. Ready to use
 */
export class AppInitializer {
  private state: InitState = {
    phase: 'idle',
    progress: 0,
    message: 'Not started',
  };

  private callbacks: StateCallback[] = [];
  private world: World | null = null;
  private sceneManager: SceneManager | null = null;
  private renderer: THREE.WebGLRenderer | null = null;

  /**
   * Subscribe to state changes
   */
  onStateChange(callback: StateCallback): () => void {
    this.callbacks.push(callback);
    return () => {
      const idx = this.callbacks.indexOf(callback);
      if (idx > -1) this.callbacks.splice(idx, 1);
    };
  }

  /**
   * Get current state
   */
  getState(): InitState {
    return { ...this.state };
  }

  /**
   * Get physics world (available after initialization)
   */
  getWorld(): World {
    if (!this.world) throw new Error('World not initialized');
    return this.world;
  }

  /**
   * Get scene manager (available after initialization)
   */
  getSceneManager(): SceneManager {
    if (!this.sceneManager) throw new Error('Scene manager not initialized');
    return this.sceneManager;
  }

  /**
   * Get renderer (available after initialization)
   */
  getRenderer(): THREE.WebGLRenderer {
    if (!this.renderer) throw new Error('Renderer not initialized');
    return this.renderer;
  }

  /**
   * Initialize all subsystems
   */
  async initialize(canvas: HTMLCanvasElement): Promise<void> {
    try {
      logger.log('common', '[AppInitializer] Starting initialization...');
      const totalStart = performance.now();

      // Phase 1: WASM modules
      this.updateState('wasm', 0, 'Loading WASM modules...');
      await this.loadWasmModules();
      this.updateState('wasm', 100, 'WASM loaded');

      // Phase 2: Rendering + Physics
      this.updateState('rendering', 0, 'Initializing physics...');

      // Create physics world
      this.world = new World(new THREE.Vector3(0, -9.8, 0));
      await this.world.init();
      this.updateState('rendering', 33, 'Physics initialized');

      // Create scene manager and initialize
      this.sceneManager = new SceneManager();
      await this.sceneManager.initialize(canvas);
      this.renderer = this.sceneManager.getRenderer();
      this.updateState('rendering', 100, 'Renderer initialized');

      // Ready!
      this.updateState('ready', 100, 'Ready');

      const elapsed = performance.now() - totalStart;
      logger.log('common', `[AppInitializer] Complete in ${elapsed.toFixed(0)}ms`);
    } catch (error) {
      logger.error('common', '[AppInitializer] Failed:', error);
      this.updateState('error', 0, 'Initialization failed', error as Error);
      throw error;
    }
  }

  /**
   * Dispose all resources
   */
  dispose(): void {
    logger.log('common', '[AppInitializer] Disposing...');
    this.sceneManager?.dispose();
    this.world?.dispose();
    this.sceneManager = null;
    this.world = null;
    this.renderer = null;
  }

  // Private methods

  private async loadWasmModules(): Promise<void> {
    logger.log('common', '[AppInitializer] Loading WASM modules...');
    const start = performance.now();

    // Load both in parallel
    await Promise.all([
      this.loadCubeWasm(),
      this.loadPhysicsWasm(),
    ]);

    const elapsed = performance.now() - start;
    logger.log('common', `[AppInitializer] WASM loaded in ${elapsed.toFixed(0)}ms`);
  }

  private async loadCubeWasm(): Promise<void> {
    await ensureCubeWasmInitialized();
    logger.log('common', '[AppInitializer] Cube WASM ready');
  }

  private async loadPhysicsWasm(): Promise<void> {
    await initPhysicsWasm();
    logger.log('common', '[AppInitializer] Physics WASM ready');
  }

  private updateState(
    phase: InitPhase,
    progress: number,
    message: string,
    error?: Error
  ): void {
    this.state = { phase, progress, message, error };
    this.callbacks.forEach(cb => {
      try {
        cb({ ...this.state });
      } catch (err) {
        logger.error('common', '[AppInitializer] Callback error:', err);
      }
    });
  }
}
