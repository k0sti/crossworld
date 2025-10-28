import * as THREE from 'three';
import { Transform } from './transform';
import { TeleportAnimation, type TeleportAnimationType } from './teleport-animation';
import { ProfileIcon } from './profile-icon';

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
  setPositionImmediate(x: number, z: number): void;
  setRunSpeed(isRunning: boolean): void;

  // Profile icon
  setProfilePicture(pictureUrl: string | null): Promise<void>;
  setDisplayName(displayName: string): void;

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
  protected baseMoveSpeed: number = 3.0;
  protected isRunning: boolean = false;
  protected teleportAnimation: TeleportAnimation | null = null;
  protected scene: THREE.Scene | null = null;
  protected profileIcon: ProfileIcon | null = null;

  // Smooth movement properties
  protected velocity: THREE.Vector2 = new THREE.Vector2(0, 0);
  protected currentDirection: number = 0; // Current facing angle in radians
  protected targetDirection: number = 0; // Target facing angle in radians
  protected turnSpeed: number = 5.0; // Radians per second
  protected velocityAcceleration: number = 10.0; // Acceleration constant
  protected velocityDamping: number = 0.95; // Damping factor (0.5 = half previous velocity)

  protected get moveSpeed(): number {
    return this.baseMoveSpeed * (this.isRunning ? 2.0 : 1.0);
  }

  constructor(initialTransform?: Transform, scene?: THREE.Scene) {
    this.transform = initialTransform ? Transform.fromTransform(initialTransform) : new Transform(4, 0, 4);
    this.targetTransform = this.transform.clone();
    this.scene = scene || null;
    this.currentDirection = this.transform.getAngle();
    this.targetDirection = this.currentDirection;

    this.group = new THREE.Group();
    this.transform.applyToObject3D(this.group);

    // Create profile icon and add to group
    this.profileIcon = new ProfileIcon(0.8);
    this.profileIcon.setPosition(0, 2.1, 0); // Position above avatar (lowered to be closer)
    this.group.add(this.profileIcon.getSprite());
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

  setRunSpeed(isRunning: boolean): void {
    this.isRunning = isRunning;
  }

  setTargetPosition(x: number, z: number): void {
    this.targetTransform.setXZ(x, z);

    const wasMoving = this.isMoving;
    this.isMoving = true;

    if (!wasMoving) {
      this.onStartMoving();
    }

    // Target direction will be calculated continuously in update()
  }

  setScene(scene: THREE.Scene): void {
    this.scene = scene;
  }

  async setProfilePicture(pictureUrl: string | null): Promise<void> {
    if (!this.profileIcon) return;

    if (pictureUrl) {
      await this.profileIcon.loadPicture(pictureUrl);
    } else {
      this.profileIcon.resetToDefault();
    }
  }

  setDisplayName(displayName: string): void {
    if (!this.profileIcon) return;
    this.profileIcon.setDisplayName(displayName);
  }

  setPositionImmediate(x: number, z: number): void {
    // Check if position actually changed
    const dx = x - this.transform.getX();
    const dz = z - this.transform.getZ();
    const distance = Math.sqrt(dx * dx + dz * dz);

    if (distance < 0.01) {
      return;
    }

    // Stop any current movement
    this.isMoving = false;
    this.velocity.set(0, 0);
    this.onStopMoving();

    // Calculate target rotation
    const targetAngle = Math.atan2(dx, dz) + this.getRotationOffset();

    // Cancel any existing teleport animation
    if (this.teleportAnimation?.isActive()) {
      this.teleportAnimation.cancel();
    }

    // Immediately set new position and orientation (no animation)
    this.transform.setXZ(x, z);
    this.transform.setAngle(targetAngle);
    this.currentDirection = targetAngle;
    this.targetDirection = targetAngle;
    this.targetTransform.setXZ(x, z);
    this.targetTransform.setAngle(targetAngle);
    this.group.position.set(this.transform.getX(), this.transform.getY(), this.transform.getZ());
    this.group.quaternion.copy(this.transform.getRotation());
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
    this.velocity.set(0, 0);
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
    this.currentDirection = targetAngle;
    this.targetDirection = targetAngle;
    this.targetTransform.setXZ(x, z);
    this.targetTransform.setAngle(targetAngle);
    this.group.position.set(this.transform.getX(), this.transform.getY(), this.transform.getZ());
    this.group.quaternion.copy(this.transform.getRotation());
  }

  /**
   * Calculate shortest angular difference between two angles
   * Returns value in range [-π, π]
   */
  private angleDifference(target: number, current: number): number {
    let diff = target - current;
    // Normalize to [-π, π]
    while (diff > Math.PI) diff -= 2 * Math.PI;
    while (diff < -Math.PI) diff += 2 * Math.PI;
    return diff;
  }

  update(deltaTime_s: number): void {
    // Update teleport animation if active
    if (this.teleportAnimation?.isActive()) {
      this.teleportAnimation.update(performance.now());
      return;
    }


    const distance = this.transform.distanceTo2D(this.targetTransform);

    if (!this.isMoving) {
      return;
    } else if (distance < 0.01) {
      // Reached target
      this.transform.setXZ(this.targetTransform.getX(), this.targetTransform.getZ());
      this.group.position.set(this.transform.getX(), this.transform.getY(), this.transform.getZ());
      this.velocity.set(0, 0);
      this.isMoving = false;
      this.onStopMoving();
      return;
    }

    // Calculate target direction continuously based on current position
    const dx = this.targetTransform.getX() - this.transform.getX();
    const dz = this.targetTransform.getZ() - this.transform.getZ();
    this.targetDirection = Math.atan2(dx, dz) + this.getRotationOffset();

    // Smooth rotation towards target direction
    const angleDiff = this.angleDifference(this.targetDirection, this.currentDirection);
    const turnAmount = this.turnSpeed * deltaTime_s;

    if (Math.abs(angleDiff) < turnAmount) {
      // Close enough, snap to target
      this.currentDirection = this.targetDirection;
    } else {
      // Turn towards target (shortest path)
      this.currentDirection += Math.sign(angleDiff) * turnAmount;
    }

    // Normalize current direction to [-π, π]
    while (this.currentDirection > Math.PI) this.currentDirection -= 2 * Math.PI;
    while (this.currentDirection < -Math.PI) this.currentDirection += 2 * Math.PI;

    // Update transform rotation
    this.transform.setAngle(this.currentDirection);
    this.group.quaternion.copy(this.transform.getRotation());

    // Calculate direction to target (reusing dx, dz from above)
    const targetOffset = new THREE.Vector2(dx, dz);
    const targetDir = targetOffset.normalize();

    // Current direction vector (without rotation offset for velocity calculation)
    const currentDirNoOffset = this.currentDirection - this.getRotationOffset();
    const currentDir = new THREE.Vector2(Math.sin(currentDirNoOffset), Math.cos(currentDirNoOffset));

    // Calculate dot product (alignment with target)
    const alignment = Math.max(0, 0.5+targetDir.dot(currentDir));

    // Update velocity: velocity = damping * lastVelocity + alignment * acceleration * currentDir
    this.velocity.multiplyScalar(this.velocityDamping);
    
    // brake when close
    // if (targetOffset.length()<1) this.velocity.multiplyScalar(0.8);

    const accelerationForce = alignment * this.velocityAcceleration * deltaTime_s;
    this.velocity.add(currentDir.clone().multiplyScalar(accelerationForce));

    // Update position
    const newX = this.transform.getX() + this.velocity.x * deltaTime_s;
    const newZ = this.transform.getZ() + this.velocity.y * deltaTime_s;

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
