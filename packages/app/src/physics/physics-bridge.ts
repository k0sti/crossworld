// @ts-ignore - physics module import (built separately)
import initPhysicsWasm, { WasmPhysicsWorld } from '../../../wasm-physics/crossworld_physics.js';
import * as THREE from 'three';
import * as logger from '../utils/logger';

let wasmInitialized = false;

/**
 * Initialize the physics WASM module (idempotent)
 */
async function ensurePhysicsWasmInitialized(): Promise<void> {
  if (!wasmInitialized) {
    await initPhysicsWasm();
    wasmInitialized = true;
    logger.log('common', '[PhysicsWasm] WASM module initialized');
  }
}

/**
 * PhysicsBridge - TypeScript wrapper for the WASM physics engine
 *
 * Provides a clean interface for character controller physics,
 * managing the lifecycle of physics characters and synchronizing
 * state between the physics simulation and Three.js rendering.
 */
export class PhysicsBridge {
  private world: WasmPhysicsWorld | null = null;
  private initialized: boolean = false;
  private gravity: THREE.Vector3;

  constructor(gravity: THREE.Vector3 = new THREE.Vector3(0, -9.8, 0)) {
    this.gravity = gravity;
  }

  /**
   * Initialize the physics world asynchronously
   * Must be called before using any physics methods
   */
  async init(): Promise<void> {
    if (this.initialized) return;

    await ensurePhysicsWasmInitialized();
    this.world = new WasmPhysicsWorld(this.gravity.x, this.gravity.y, this.gravity.z);

    // Create ground plane at Y=0
    this.world.createGroundPlane();
    logger.log('common', '[PhysicsWorld] Ground plane created at Y=0');

    this.initialized = true;
  }

  /**
   * Create a character controller in the physics world
   *
   * @param position - Initial position in world space
   * @param height - Character height (default: 1.8m for human)
   * @param radius - Character radius (default: 0.3m)
   * @returns Physics handle (character ID) for this character
   */
  createCharacter(
    position: THREE.Vector3,
    height: number = 1.8,
    radius: number = 0.3
  ): number {
    if (!this.world) {
      throw new Error('PhysicsBridge not initialized. Call init() first.');
    }
    return this.world.createCharacter(position.x, position.y, position.z, height, radius);
  }

  /**
   * Move a character with the given horizontal velocity
   *
   * @param characterId - Physics handle for the character
   * @param velocity - Horizontal velocity (Y component is ignored)
   * @param dt - Time step in seconds
   */
  moveCharacter(characterId: number, velocity: THREE.Vector3, dt: number): void {
    if (!this.world) return;
    this.world.moveCharacter(characterId, velocity.x, velocity.z, dt);
  }

  /**
   * Make a character jump if they are grounded
   *
   * @param characterId - Physics handle for the character
   */
  jump(characterId: number): void {
    if (!this.world) return;
    this.world.jumpCharacter(characterId);
  }

  /**
   * Get the current position of a character from physics
   *
   * @param characterId - Physics handle for the character
   * @returns Position vector
   */
  getCharacterPosition(characterId: number): THREE.Vector3 {
    if (!this.world) {
      return new THREE.Vector3(0, 0, 0);
    }
    const pos = this.world.getCharacterPosition(characterId);
    return new THREE.Vector3(pos[0], pos[1], pos[2]);
  }

  /**
   * Get the current rotation of a character from physics
   *
   * @param characterId - Physics handle for the character
   * @returns Quaternion rotation
   */
  getCharacterRotation(characterId: number): THREE.Quaternion {
    if (!this.world) {
      return new THREE.Quaternion(0, 0, 0, 1);
    }
    const bodyHandle = characterId;
    const rot = this.world.getRotation(bodyHandle);
    return new THREE.Quaternion(rot[0], rot[1], rot[2], rot[3]);
  }

  /**
   * Check if a character is currently on the ground
   *
   * @param characterId - Physics handle for the character
   * @returns True if grounded
   */
  isGrounded(characterId: number): boolean {
    if (!this.world) return false;
    return this.world.isObjectGrounded(characterId);
  }

  /**
   * Get the ground normal vector for a character
   *
   * @param characterId - Physics handle for the character
   * @returns Ground normal vector
   */
  getGroundNormal(characterId: number): THREE.Vector3 {
    if (!this.world) {
      return new THREE.Vector3(0, 1, 0);
    }
    const normal = this.world.getObjectGroundNormal(characterId);
    return new THREE.Vector3(normal[0], normal[1], normal[2]);
  }

  /**
   * Set the position of a character (e.g., for teleportation)
   *
   * @param characterId - Physics handle for the character
   * @param position - New position
   */
  setCharacterPosition(characterId: number, position: THREE.Vector3): void {
    if (!this.world) return;
    const bodyHandle = characterId;
    this.world.setPosition(bodyHandle, position.x, position.y, position.z);
  }

  /**
   * Set the rotation of a character
   *
   * @param characterId - Physics handle for the character
   * @param rotation - New rotation quaternion
   */
  setCharacterRotation(characterId: number, rotation: THREE.Quaternion): void {
    if (!this.world) return;
    const bodyHandle = characterId;
    this.world.setRotation(bodyHandle, rotation.x, rotation.y, rotation.z, rotation.w);
  }

  /**
   * Remove a character from the physics world
   *
   * @param characterId - Physics handle for the character
   */
  removeCharacter(characterId: number): void {
    if (!this.world) return;
    const bodyHandle = characterId;
    this.world.removeObject(bodyHandle);
  }

  /**
   * Step the physics simulation forward by dt seconds
   *
   * @param dt - Time step in seconds (typically 1/60)
   */
  step(dt: number): void {
    if (!this.world) return;
    this.world.step(dt);
  }

  /**
   * Get the gravity vector
   *
   * @returns Gravity vector
   */
  getGravity(): THREE.Vector3 {
    if (!this.world) {
      return this.gravity.clone();
    }
    const g = this.world.getGravity();
    return new THREE.Vector3(g[0], g[1], g[2]);
  }

  /**
   * Set the gravity vector
   *
   * @param gravity - New gravity vector
   */
  setGravity(gravity: THREE.Vector3): void {
    this.gravity = gravity;
    if (!this.world) return;
    this.world.setGravity(gravity.x, gravity.y, gravity.z);
  }

  /**
   * Clean up physics resources
   */
  dispose(): void {
    this.initialized = false;
    // WASM cleanup will happen automatically
  }
}
