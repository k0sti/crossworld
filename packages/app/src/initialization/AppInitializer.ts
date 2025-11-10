/**
 * App Initializer
 *
 * Orchestrates initialization of all app subsystems in the correct order
 */

import * as THREE from 'three';
import {
  type InitializationState,
  type InitializedSystems,
  type AppInitializerConfig,
  type InitializationPhase,
  type InitializationStateCallback,
  isValidTransition,
} from './types';
import { loadAllWasmModules } from './WasmLoader';
import { initializePhysics } from './PhysicsInitializer';
import { initializeRenderer } from './RendererInitializer';
import type { PhysicsBridge } from '../physics/physics-bridge';
import type { SceneManager } from '../renderer/scene';
import * as logger from '../utils/logger';

/**
 * AppInitializer - Single source of truth for app initialization
 *
 * Manages initialization of all subsystems in the correct order:
 * 1. WASM modules (cube + physics) - parallel
 * 2. Rendering infrastructure (renderer, scene, physics world)
 * 3. User session (login, profile) - handled by UI
 * 4. Game world (materials, textures)
 * 5. Avatar spawning
 * 6. Ready to interact
 */
export class AppInitializer {
  private state: InitializationState;
  private callbacks: InitializationStateCallback[] = [];
  private systems: Partial<InitializedSystems> = {};

  constructor() {
    this.state = {
      phase: 'idle',
      progress: 0,
      message: 'Initialization not started',
      timestamp: Date.now(),
    };
  }

  /**
   * Get current initialization state
   */
  getState(): InitializationState {
    return { ...this.state };
  }

  /**
   * Subscribe to state updates
   */
  onStateChange(callback: InitializationStateCallback): () => void {
    this.callbacks.push(callback);
    // Return unsubscribe function
    return () => {
      const index = this.callbacks.indexOf(callback);
      if (index > -1) {
        this.callbacks.splice(index, 1);
      }
    };
  }

  /**
   * Get initialized subsystems (throws if not ready)
   */
  getSystems(): InitializedSystems {
    if (this.state.phase !== 'ready') {
      throw new Error('Systems not ready yet. Current phase: ' + this.state.phase);
    }
    return this.systems as InitializedSystems;
  }

  /**
   * Get physics bridge (available after 'wasm' phase)
   */
  getPhysicsBridge(): PhysicsBridge {
    if (!this.systems.physicsBridge) {
      throw new Error('Physics bridge not initialized yet');
    }
    return this.systems.physicsBridge;
  }

  /**
   * Get scene manager (available after 'rendering' phase)
   */
  getSceneManager(): SceneManager {
    if (!this.systems.sceneManager) {
      throw new Error('Scene manager not initialized yet');
    }
    return this.systems.sceneManager;
  }

  /**
   * Get renderer (available after 'rendering' phase)
   */
  getRenderer(): THREE.WebGLRenderer {
    if (!this.systems.renderer) {
      throw new Error('Renderer not initialized yet');
    }
    return this.systems.renderer;
  }

  /**
   * Initialize all subsystems
   *
   * This is the main entry point for app initialization.
   * Progresses through phases: wasm → rendering → ready
   * (session and world phases are handled by UI components)
   */
  async initialize(config: AppInitializerConfig): Promise<void> {
    try {
      logger.log('common', '[AppInitializer] Starting initialization...');
      const totalStartTime = performance.now();

      // Phase 1: Load WASM modules
      await this.executePhase('wasm', async () => {
        this.updateProgress(0, 'Loading WASM modules...');
        await loadAllWasmModules();
        this.updateProgress(100, 'WASM modules loaded');
      });

      // Phase 2: Initialize physics
      this.updatePhase('rendering', 0, 'Initializing physics...');
      const physicsBridge = await initializePhysics();
      this.systems.physicsBridge = physicsBridge;
      this.updateProgress(33, 'Physics initialized');

      // Phase 3: Initialize renderer and scene
      this.updateProgress(33, 'Initializing renderer...');
      const { sceneManager, renderer } = await initializeRenderer({
        canvas: config.canvas,
        physicsBridge,
      });
      this.systems.sceneManager = sceneManager;
      this.systems.renderer = renderer;
      this.systems.canvas = config.canvas;
      this.updateProgress(100, 'Renderer initialized');

      // Ready!
      this.updatePhase('ready', 100, 'Initialization complete');

      const totalElapsed = performance.now() - totalStartTime;
      logger.log('common', `[AppInitializer] Initialization complete in ${totalElapsed.toFixed(0)}ms`);
    } catch (error) {
      logger.error('common', '[AppInitializer] Initialization failed:', error);
      this.updatePhase('error', 0, 'Initialization failed');
      this.state.error = error as Error;
      this.notifyCallbacks();
      throw error;
    }
  }

  /**
   * Clean up all resources
   */
  dispose(): void {
    logger.log('common', '[AppInitializer] Disposing...');

    if (this.systems.sceneManager) {
      this.systems.sceneManager.dispose();
    }

    if (this.systems.physicsBridge) {
      this.systems.physicsBridge.dispose();
    }

    this.systems = {};
    this.updatePhase('idle', 0, 'Disposed');
  }

  // Private helper methods

  /**
   * Execute a phase with error handling
   */
  private async executePhase(
    phase: InitializationPhase,
    action: () => Promise<void>
  ): Promise<void> {
    this.updatePhase(phase, 0, `Starting ${phase} phase...`);
    await action();
  }

  /**
   * Update current phase
   */
  private updatePhase(phase: InitializationPhase, progress: number, message: string): void {
    if (!isValidTransition(this.state.phase, phase)) {
      logger.warn('common', `[AppInitializer] Invalid phase transition: ${this.state.phase} → ${phase}`);
    }

    this.state = {
      phase,
      progress,
      message,
      timestamp: Date.now(),
    };

    this.notifyCallbacks();
  }

  /**
   * Update progress within current phase
   */
  private updateProgress(progress: number, message: string): void {
    this.state = {
      ...this.state,
      progress,
      message,
      timestamp: Date.now(),
    };

    this.notifyCallbacks();
  }

  /**
   * Notify all state change callbacks
   */
  private notifyCallbacks(): void {
    const stateCopy = { ...this.state };
    this.callbacks.forEach((callback) => {
      try {
        callback(stateCopy);
      } catch (error) {
        logger.error('common', '[AppInitializer] Error in state callback:', error);
      }
    });
  }
}
