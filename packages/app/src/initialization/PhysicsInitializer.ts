/**
 * Physics Initializer
 *
 * Handles physics world setup and configuration
 */

import * as THREE from 'three';
import { World } from '../physics/world';
import * as logger from '../utils/logger';

export interface PhysicsConfig {
  /** Gravity vector (default: 0, -9.8, 0) */
  gravity?: THREE.Vector3;

  /** Whether to create ground plane at Y=0 */
  createGroundPlane?: boolean;
}

/**
 * Initialize physics world
 *
 * Creates World and sets up the physics world with ground plane
 *
 * @param config Physics configuration
 * @returns Initialized World
 */
export async function initializePhysics(config: PhysicsConfig = {}): Promise<World> {
  const {
    gravity = new THREE.Vector3(0, -9.8, 0),
  } = config;

  logger.log('common', '[PhysicsInit] Initializing physics world...');
  const startTime = performance.now();

  // Create physics bridge
  const physicsBridge = new World(gravity);

  // Initialize physics world (creates WasmPhysicsWorld internally)
  await physicsBridge.init();

  // Ground plane is created automatically in World.init()
  // No need to create it again here

  const elapsed = performance.now() - startTime;
  logger.log('common', `[PhysicsInit] Physics world initialized in ${elapsed.toFixed(0)}ms`);
  logger.log('common', `[PhysicsInit] Gravity: (${gravity.x}, ${gravity.y}, ${gravity.z})`);

  return physicsBridge;
}

/**
 * Dispose physics world
 */
export function disposePhysics(physicsBridge: World): void {
  logger.log('common', '[PhysicsInit] Disposing physics world...');
  physicsBridge.dispose();
}
