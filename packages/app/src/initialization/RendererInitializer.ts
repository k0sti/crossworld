/**
 * Renderer Initializer
 *
 * Handles Three.js renderer and scene setup
 */

import * as THREE from 'three';
import { SceneManager } from '../renderer/scene';
import { PhysicsBridge } from '../physics/physics-bridge';
import * as logger from '../utils/logger';

export interface RendererConfig {
  /** Canvas element to render to */
  canvas: HTMLCanvasElement;

  /** Physics bridge (must be initialized) */
  physicsBridge: PhysicsBridge;

  /** Enable antialiasing (default: true) */
  antialias?: boolean;

  /** Enable shadows (default: true) */
  shadows?: boolean;

  /** Pixel ratio (default: window.devicePixelRatio) */
  pixelRatio?: number;
}

export interface InitializedRenderer {
  /** Scene manager */
  sceneManager: SceneManager;

  /** WebGL renderer */
  renderer: THREE.WebGLRenderer;
}

/**
 * Initialize renderer and scene
 *
 * @param config Renderer configuration
 * @returns Initialized renderer and scene manager
 */
export async function initializeRenderer(config: RendererConfig): Promise<InitializedRenderer> {
  const { canvas } = config;

  logger.log('common', '[RendererInit] Initializing renderer and scene...');
  const startTime = performance.now();

  // Check for WebGL 2.0 support
  const gl = canvas.getContext('webgl2');
  if (!gl) {
    logger.warn('renderer', '[RendererInit] WebGL 2.0 not available, falling back to WebGL 1.0');
  } else {
    logger.log('renderer', '[RendererInit] Using WebGL 2.0 context');
  }

  // Create scene manager
  const sceneManager = new SceneManager();

  // Initialize scene (this will set up renderer, scene, camera, etc.)
  await sceneManager.initialize(canvas);

  // Get the renderer from scene manager
  const renderer = sceneManager.getRenderer();

  const elapsed = performance.now() - startTime;
  logger.log('common', `[RendererInit] Renderer initialized in ${elapsed.toFixed(0)}ms`);

  return { sceneManager, renderer };
}

/**
 * Dispose renderer and scene
 */
export function disposeRenderer(sceneManager: SceneManager): void {
  logger.log('common', '[RendererInit] Disposing renderer...');
  sceneManager.dispose();
}
