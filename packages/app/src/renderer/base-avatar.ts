import * as THREE from 'three';
import { Transform } from './transform';
import { TeleportAnimation, type TeleportAnimationType } from './teleport-animation';

/**
 * Common interface for all avatar types (GLB, VOX, etc.)
 */
export interface IAvatar {
  // Rendering
  getObject3D(): THREE.Group;

  // Transform
  getTransform(): Transform;
  getPosition(): THREE.Vector3;

  // Movement
  setTargetPosition(x: number, z: number): void;
  teleportTo(x: number, z: number, animationType: TeleportAnimationType): void;

  // State
  isCurrentlyMoving(): boolean;
  isTeleporting(): boolean;

  // Lifecycle
  update(deltaTime_s: number): void;
  dispose(): void;
  setScene(scene: THREE.Scene): void;
}

/**
 * Base class implementing common avatar behavior
 *
 * This extracts the 287 lines of duplicated movement/teleport logic
 * that was previously copied between Avatar and VoxelAvatar classes.
 */
export abstract class BaseAvatar implements IAvatar {
  protected group: THREE.Group;
  protected transform: Transform;
  protected targetTransform: Transform;
  protected isMoving: boolean = false;
  protected moveSpeed: number = 3.0;
  protected teleportAnimation: TeleportAnimation | null = null;
  protected scene: THREE.Scene | null = null;

  constructor(initialTransform?: Transform, scene?: THREE.Scene) {
    this.transform = initialTransform ? Transform.fromTransform(initialTransform) : new Transform(4, 0, 4);
    this.targetTransform = this.transform.clone();
    this.scene = scene || null;

    this.group = new THREE.Group();
    this.transform.applyToObject3D(this.group);
  }

  // ========== Abstract methods (subclass-specific) ==========

  /**
   * Get the renderable model inside the group
   * Used by teleport animation to clone the visual
   */
  protected abstract getModel(): THREE.Object3D | null;

  /**
   * Rotation offset to apply when facing a direction
   * GLB: 0, VOX: π (models face opposite directions)
   */
  protected abstract getRotationOffset(): number;

  /**
   * Called when avatar stops moving (for animation changes)
   */
  protected abstract onStopMoving(): void;

  /**
   * Called when avatar starts moving (for animation changes)
   */
  protected abstract onStartMoving(): void;

  // ========== Shared implementation ==========

  setTargetPosition(x: number, z: number): void {
    this.targetTransform.setXZ(x, z);

    const wasMoving = this.isMoving;
    this.isMoving = true;

    if (!wasMoving) {
      this.onStartMoving();
    }

    // Calculate direction and rotate to face it
    const dx = x - this.transform.getX();
    const dz = z - this.transform.getZ();
    const distance = Math.sqrt(dx * dx + dz * dz);

    if (distance > 0.01) {
      const angle = Math.atan2(dx, dz) + this.getRotationOffset();
      this.transform.setAngle(angle);
      this.targetTransform.setAngle(angle);
      this.group.quaternion.copy(this.transform.getRotation());
    }
  }

  setScene(scene: THREE.Scene): void {
    this.scene = scene;
  }

  teleportTo(x: number, z: number, animationType: TeleportAnimationType = 'fade'): void {
    const model = this.getModel();
    if (!model || !this.scene) return;

    // Check if position actually changed
    const dx = x - this.transform.getX();
    const dz = z - this.transform.getZ();
    const distance = Math.sqrt(dx * dx + dz * dz);

    if (distance < 0.01) {
      return;
    }

    // Stop any current movement
    this.isMoving = false;
    this.onStopMoving();

    // Calculate target rotation
    const targetAngle = Math.atan2(dx, dz) + this.getRotationOffset();

    // Cancel any existing teleport animation
    if (this.teleportAnimation?.isActive()) {
      this.teleportAnimation.cancel();
    }

    // Create and start teleport animation
    this.teleportAnimation = new TeleportAnimation(this.group, this.scene, animationType);
    this.teleportAnimation.start();

    // Immediately set new position and orientation
    this.transform.setXZ(x, z);
    this.transform.setAngle(targetAngle);
    this.targetTransform.setXZ(x, z);
    this.targetTransform.setAngle(targetAngle);
    this.group.position.set(this.transform.getX(), this.transform.getY(), this.transform.getZ());
    this.group.quaternion.copy(this.transform.getRotation());
  }

  update(deltaTime_s: number): void {
    // Update teleport animation if active
    if (this.teleportAnimation?.isActive()) {
      this.teleportAnimation.update(performance.now());
      return;
    }

    if (!this.isMoving) return;

    const distance = this.transform.distanceTo2D(this.targetTransform);

    if (distance < 0.1) {
      // Reached target
      this.transform.setXZ(this.targetTransform.getX(), this.targetTransform.getZ());
      this.group.position.set(this.transform.getX(), this.transform.getY(), this.transform.getZ());
      this.isMoving = false;
      this.onStopMoving();
      return;
    }

    // Move towards target
    const moveDistance = this.moveSpeed * deltaTime_s;
    const dx = this.targetTransform.getX() - this.transform.getX();
    const dz = this.targetTransform.getZ() - this.transform.getZ();
    const direction = new THREE.Vector2(dx, dz).normalize();

    const actualMove = Math.min(moveDistance, distance);
    const newX = this.transform.getX() + direction.x * actualMove;
    const newZ = this.transform.getZ() + direction.y * actualMove;

    this.transform.setXZ(newX, newZ);
    this.group.position.set(this.transform.getX(), this.transform.getY(), this.transform.getZ());
  }

  getObject3D(): THREE.Group {
    return this.group;
  }

  getTransform(): Transform {
    return this.transform.clone();
  }

  getPosition(): THREE.Vector3 {
    return this.transform.getPosition();
  }

  isCurrentlyMoving(): boolean {
    return this.isMoving;
  }

  isTeleporting(): boolean {
    return this.teleportAnimation?.isActive() ?? false;
  }

  abstract dispose(): void;
}
