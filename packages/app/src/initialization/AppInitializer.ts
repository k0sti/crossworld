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
  type SubComponentStatus,
  isValidTransition,
} from './types';
import { loadAllWasmModules } from './WasmLoader';
import { initializePhysics } from './PhysicsInitializer';
import { initializeRenderer } from './RendererInitializer';
import type { World } from '../physics/world';
import type { SceneManager } from '../renderer/scene';
import { AvatarStateService } from '../services/avatar-state';
import * as logger from '../utils/logger';

/**
 * AppInitializer - Single source of truth for app initialization
 *
 * Manages initialization of all subsystems with parallel loading:
 * 1. WASM modules (cube + physics) + Network (Nostr) - parallel
 * 2. Rendering infrastructure (renderer, scene, physics world)
 * 3. User session (login, profile)
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
      subComponents: new Map(),
    };

    // Initialize sub-component tracking
    this.initializeSubComponents();
  }

  /**
   * Initialize sub-component status tracking
   */
  private initializeSubComponents(): void {
    const components: SubComponentStatus[] = [
      {
        id: 'wasm',
        name: 'WASM',
        color: 'purple',
        status: 'pending',
        progress: 0,
      },
      {
        id: 'rendering',
        name: 'Rendering',
        color: 'blue',
        status: 'pending',
        progress: 0,
      },
      {
        id: 'network',
        name: 'Network',
        color: 'green',
        status: 'pending',
        progress: 0,
      },
      {
        id: 'session',
        name: 'Session',
        color: 'orange',
        status: 'pending',
        progress: 0,
      },
    ];

    components.forEach((component) => {
      this.state.subComponents.set(component.id, component);
    });
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
  getWorld(): World {
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
   * Get account manager (available after 'network' phase)
   */
  getAccountManager() {
    return this.systems.accountManager;
  }

  /**
   * Get avatar state service (available after 'network' phase)
   */
  getAvatarStateService() {
    return this.systems.avatarStateService;
  }

  /**
   * Initialize all subsystems
   *
   * This is the main entry point for app initialization.
   * Uses parallel loading for WASM and Network phases.
   */
  async initialize(config: AppInitializerConfig): Promise<void> {
    try {
      logger.log('common', '[AppInitializer] Starting initialization...');
      const totalStartTime = performance.now();

      // Phase 1: Parallel loading of WASM and Network
      this.updatePhase('wasm', 0, 'Starting parallel initialization...');

      const [wasmResult, networkResult] = await Promise.allSettled([
        // WASM track
        this.initializeWasm(),
        // Network track
        this.initializeNetwork(config.accountManager),
      ]);

      // Check for errors in parallel phase
      if (wasmResult.status === 'rejected') {
        throw new Error(`WASM initialization failed: ${wasmResult.reason}`);
      }
      if (networkResult.status === 'rejected') {
        logger.warn('common', `Network initialization failed: ${networkResult.reason}`);
        // Continue without network - it's not critical for core functionality
      } else {
        // Store network systems
        const { accountManager, avatarStateService } = networkResult.value;
        this.systems.accountManager = accountManager;
        this.systems.avatarStateService = avatarStateService;
      }

      // Phase 2: Initialize rendering (requires WASM physics)
      await this.initializeRendering(config.canvas, wasmResult.value);

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
   * Initialize WASM modules
   */
  private async initializeWasm(): Promise<World> {
    this.updateSubComponent('wasm', { status: 'loading', progress: 0 });

    await loadAllWasmModules();
    this.updateSubComponent('wasm', { status: 'loading', progress: 50, message: 'WASM loaded' });

    const physicsBridge = await initializePhysics();
    this.systems.physicsBridge = physicsBridge;
    this.updateSubComponent('wasm', { status: 'complete', progress: 100, message: 'Complete' });

    return physicsBridge;
  }

  /**
   * Initialize Network (Nostr) subsystems
   */
  private async initializeNetwork(accountManager: any) {
    this.updateSubComponent('network', { status: 'loading', progress: 0, message: 'Connecting...' });

    // Create avatar state service if we have an account manager
    let avatarStateService: AvatarStateService | undefined;

    if (accountManager) {
      avatarStateService = new AvatarStateService(accountManager);
      this.updateSubComponent('network', { status: 'loading', progress: 50, message: 'Service created' });

      // Start subscription to avatar states
      avatarStateService.startSubscription();
      this.updateSubComponent('network', { status: 'loading', progress: 75, message: 'Subscribed' });
    }

    this.updateSubComponent('network', { status: 'complete', progress: 100, message: 'Complete' });

    return { accountManager, avatarStateService };
  }

  /**
   * Initialize rendering subsystems
   */
  private async initializeRendering(canvas: HTMLCanvasElement, physicsBridge: World): Promise<void> {
    this.updatePhase('rendering', 0, 'Initializing renderer...');
    this.updateSubComponent('rendering', { status: 'loading', progress: 0 });

    const { sceneManager, renderer } = await initializeRenderer({
      canvas,
      physicsBridge,
    });

    this.systems.sceneManager = sceneManager;
    this.systems.renderer = renderer;
    this.systems.canvas = canvas;

    this.updateSubComponent('rendering', { status: 'complete', progress: 100, message: 'Complete' });
    this.updateProgress(100, 'Renderer initialized');
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
   * Update current phase
   */
  private updatePhase(phase: InitializationPhase, progress: number, message: string): void {
    if (!isValidTransition(this.state.phase, phase)) {
      logger.warn('common', `[AppInitializer] Invalid phase transition: ${this.state.phase} → ${phase}`);
    }

    this.state = {
      ...this.state,
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
   * Update sub-component status
   */
  private updateSubComponent(
    id: string,
    update: Partial<Omit<SubComponentStatus, 'id' | 'name' | 'color'>>
  ): void {
    const current = this.state.subComponents.get(id);
    if (!current) {
      logger.warn('common', `[AppInitializer] Unknown sub-component: ${id}`);
      return;
    }

    this.state.subComponents.set(id, {
      ...current,
      ...update,
    });

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
