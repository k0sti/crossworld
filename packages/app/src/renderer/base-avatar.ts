import * as THREE from 'three';
import { Transform } from './transform';
import { TeleportAnimation, type TeleportAnimationType } from './teleport-animation';
import { ProfileIcon } from './profile-icon';
import type { PhysicsBridge } from '../physics/physics-bridge';
import * as logger from '../utils/logger';

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
  jump(): void;

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
  setRaycastMesh(mesh: THREE.Mesh): void;
}

/**
 * Avatar pivot/alignment point in normalized coordinates (0-1)
 * (0.5, 0, 0.5) means:
 * - X: centered (0.5 = middle of width)
 * - Y: bottom (0 = feet on ground)
 * - Z: centered (0.5 = middle of depth)
 */
export const AVATAR_PIVOT = new THREE.Vector3(0.5, 0, 0.5);

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
  protected raycaster: THREE.Raycaster = new THREE.Raycaster();
  protected raycastMesh: THREE.Mesh | null = null;

  // Physics integration (optional)
  protected physicsBridge: PhysicsBridge | null = null;
  protected physicsHandle: number | null = null;
  protected usePhysics: boolean = false;

  // Avatar pivot point for placement in world
  public static readonly PIVOT = AVATAR_PIVOT;

  // Smooth movement properties
  protected velocity: THREE.Vector2 = new THREE.Vector2(0, 0);
  protected currentDirection: number = 0; // Current facing angle in radians
  protected targetDirection: number = 0; // Target facing angle in radians
  protected turnSpeed: number = 15.0; // Radians per second
  protected velocityAcceleration: number = 40.0; // Acceleration constant
  protected velocityDamping: number = 0.90; // Damping factor (0.5 = half previous velocity)

  protected get moveSpeed(): number {
    return this.baseMoveSpeed * (this.isRunning ? 2.0 : 1.0);
  }

  constructor(
    initialTransform?: Transform,
    scene?: THREE.Scene,
    physicsBridge?: PhysicsBridge
  ) {
    // Default spawn position: 1.0 unit above ground (character center at Y=1.0, feet at ~Y=0.1)
    this.transform = initialTransform ? Transform.fromTransform(initialTransform) : new Transform(4, 1.0, 4);
    this.targetTransform = this.transform.clone();
    this.scene = scene || null;
    this.currentDirection = this.transform.getAngle();
    this.targetDirection = this.currentDirection;

    logger.log('avatar', `Avatar spawning at position: (${this.transform.getX()}, ${this.transform.getY()}, ${this.transform.getZ()})`);

    this.group = new THREE.Group();
    this.transform.applyToObject3D(this.group);

    // Create profile icon and add to group
    this.profileIcon = new ProfileIcon(0.8);
    this.profileIcon.setPosition(0, 2.1, 0); // Position above avatar (lowered to be closer)
    this.group.add(this.profileIcon.getSprite());

    // Initialize physics if provided
    if (physicsBridge) {
      logger.log('avatar', 'Initializing physics for avatar');
      this.physicsBridge = physicsBridge;
      this.usePhysics = true;
      const pos = this.transform.getPosition();
      logger.log('avatar', `Creating physics character at (${pos.x}, ${pos.y}, ${pos.z})`);
      try {
        this.physicsHandle = physicsBridge.createCharacter(
          new THREE.Vector3(pos.x, pos.y, pos.z)
        );
        logger.log('avatar', `Physics character created with handle: ${this.physicsHandle}`);
      } catch (error) {
        logger.error('avatar', 'Failed to create physics character:', error);
        this.usePhysics = false;
        this.physicsBridge = null;
      }
    } else {
      logger.log('avatar', 'No physics bridge provided, avatar will use non-physics movement');
    }
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

  /**
   * Called when avatar jumps (for animation/visual feedback)
   */
  protected abstract onJump(): void;

  // ========== Shared implementation ==========

  setRunSpeed(isRunning: boolean): void {
    this.isRunning = isRunning;
  }

  jump(): void {
    // If physics is enabled, use physics jump
    if (this.usePhysics && this.physicsBridge && this.physicsHandle !== null) {
      const isGrounded = this.physicsBridge.isGrounded(this.physicsHandle);
      logger.log('avatar', `Jump requested (physics enabled, grounded=${isGrounded})`);
      this.physicsBridge.jump(this.physicsHandle);
      this.onJump();
    } else {
      logger.log('avatar', 'Jump requested (non-physics mode)');
      // Fallback: just trigger visual feedback without physics
      this.onJump();
    }
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

  setRaycastMesh(mesh: THREE.Mesh): void {
    this.raycastMesh = mesh;
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

    // Raycast down to place avatar on ground
    const groundY = this.getGroundHeight(x, z, this.transform.getY());
    this.transform.setY(groundY);

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

    // Raycast down to place avatar on ground
    const groundY = this.getGroundHeight(x, z, this.transform.getY());
    this.transform.setY(groundY);

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

  /**
   * Raycast down from position to find ground height
   * Returns Y position, or 0 if no mesh exists, or currentY if no intersection found
   */
  private getGroundHeight(x: number, z: number, currentY: number): number {
    if (!this.raycastMesh) {
      // No geometry mesh available - default to ground plane at y=0
      return 0;
    }

    // Raycast from high above down to find ground
    const rayOrigin = new THREE.Vector3(x, 100, z);
    const rayDirection = new THREE.Vector3(0, -1, 0);

    this.raycaster.set(rayOrigin, rayDirection);
    const intersects = this.raycaster.intersectObject(this.raycastMesh, false);

    if (intersects.length > 0) {
      // Return the intersection point's Y coordinate
      return intersects[0].point.y;
    }

    // No intersection found - keep current Y position (avatar might be on a platform or in mid-air)
    return currentY;
  }

  update(deltaTime_s: number): void {
    // Update teleport animation if active
    if (this.teleportAnimation?.isActive()) {
      this.teleportAnimation.update(performance.now());
      return;
    }

    // If physics is enabled, sync position and rotation from physics
    if (this.usePhysics && this.physicsBridge && this.physicsHandle !== null) {
      // Send movement velocity to physics
      if (this.isMoving) {
        const dx = this.targetTransform.getX() - this.transform.getX();
        const dz = this.targetTransform.getZ() - this.transform.getZ();
        const distance = Math.sqrt(dx * dx + dz * dz);

        if (distance > 0.01) {
          const targetDir = new THREE.Vector2(dx / distance, dz / distance);
          const velocity3D = new THREE.Vector3(
            targetDir.x * this.moveSpeed,
            0,
            targetDir.y * this.moveSpeed
          );
          this.physicsBridge.moveCharacter(this.physicsHandle, velocity3D, deltaTime_s);
        } else {
          // Reached target
          this.isMoving = false;
          this.onStopMoving();
        }
      } else {
        // Not moving, send zero velocity
        this.physicsBridge.moveCharacter(this.physicsHandle, new THREE.Vector3(0, 0, 0), deltaTime_s);
      }

      // Sync position from physics
      const physicsPos = this.physicsBridge.getCharacterPosition(this.physicsHandle);
      const isGrounded = this.physicsBridge.isGrounded(this.physicsHandle);

      // Physics position is at the center of the capsule (height=1.8m)
      // Visual model should have feet on ground, so offset down by half height
      const characterHeight = 1.8;
      const visualOffset = characterHeight / 2.0;
      const visualY = physicsPos.y - visualOffset;

      // Log physics state (throttled - only log occasionally)
      if (Math.random() < 0.05) { // ~5% of frames for more frequent updates during debugging
        logger.log('avatar', `Physics: pos=(${physicsPos.x.toFixed(2)}, ${physicsPos.y.toFixed(2)}, ${physicsPos.z.toFixed(2)}), visual_y=${visualY.toFixed(2)}, grounded=${isGrounded}`);
      }

      this.transform.setXZ(physicsPos.x, physicsPos.z);
      this.transform.setY(visualY);
      this.group.position.set(physicsPos.x, visualY, physicsPos.z);

      // Update rotation to face movement direction if moving
      if (this.isMoving) {
        const dx = this.targetTransform.getX() - this.transform.getX();
        const dz = this.targetTransform.getZ() - this.transform.getZ();
        this.targetDirection = Math.atan2(dx, dz) + this.getRotationOffset();

        const angleDiff = this.angleDifference(this.targetDirection, this.currentDirection);
        const turnAmount = this.turnSpeed * deltaTime_s;

        if (Math.abs(angleDiff) < turnAmount) {
          this.currentDirection = this.targetDirection;
        } else {
          this.currentDirection += Math.sign(angleDiff) * turnAmount;
        }

        while (this.currentDirection > Math.PI) this.currentDirection -= 2 * Math.PI;
        while (this.currentDirection < -Math.PI) this.currentDirection += 2 * Math.PI;

        this.transform.setAngle(this.currentDirection);
        this.group.quaternion.copy(this.transform.getRotation());
      }

      return;
    }

    // Original non-physics movement logic
    if (!this.isMoving) {
      return;
    }

    // Calculate direction to target
    const dx = this.targetTransform.getX() - this.transform.getX();
    const dz = this.targetTransform.getZ() - this.transform.getZ();
    const distance = Math.sqrt(dx * dx + dz * dz);

    // Check if reached target
    if (distance < 0.01) {
      this.transform.setXZ(this.targetTransform.getX(), this.targetTransform.getZ());
      this.group.position.set(this.transform.getX(), this.transform.getY(), this.transform.getZ());
      this.velocity.set(0, 0);
      this.isMoving = false;
      this.onStopMoving();
      return;
    }

    // Calculate target direction for rotation
    this.targetDirection = Math.atan2(dx, dz) + this.getRotationOffset();

    // Smooth rotation towards target direction
    const angleDiff = this.angleDifference(this.targetDirection, this.currentDirection);
    const turnAmount = this.turnSpeed * deltaTime_s;

    if (Math.abs(angleDiff) < turnAmount) {
      this.currentDirection = this.targetDirection;
    } else {
      this.currentDirection += Math.sign(angleDiff) * turnAmount;
    }

    // Normalize current direction to [-π, π]
    while (this.currentDirection > Math.PI) this.currentDirection -= 2 * Math.PI;
    while (this.currentDirection < -Math.PI) this.currentDirection += 2 * Math.PI;

    // Update transform rotation
    this.transform.setAngle(this.currentDirection);
    this.group.quaternion.copy(this.transform.getRotation());

    // Move directly toward target (no angle-based acceleration)
    const targetDir = new THREE.Vector2(dx / distance, dz / distance);

    // Apply acceleration directly toward target
    this.velocity.multiplyScalar(this.velocityDamping);
    const speedMultiplier = this.isRunning ? 2.0 : 1.0;
    const accelerationForce = this.velocityAcceleration * speedMultiplier * deltaTime_s;
    this.velocity.add(targetDir.clone().multiplyScalar(accelerationForce));

    // Cap velocity at moveSpeed
    const currentSpeed = this.velocity.length();
    if (currentSpeed > this.moveSpeed) {
      this.velocity.normalize().multiplyScalar(this.moveSpeed);
    }

    // Update position
    const newX = this.transform.getX() + this.velocity.x * deltaTime_s;
    const newZ = this.transform.getZ() + this.velocity.y * deltaTime_s;

    this.transform.setXZ(newX, newZ);

    // Raycast down to place avatar on ground
    const groundY = this.getGroundHeight(newX, newZ, this.transform.getY());
    this.transform.setY(groundY);

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

  dispose(): void {
    // Cleanup physics if enabled
    if (this.usePhysics && this.physicsBridge && this.physicsHandle !== null) {
      this.physicsBridge.removeCharacter(this.physicsHandle);
      this.physicsHandle = null;
    }

    // Subclasses can override for additional cleanup
    this.onDispose();
  }

  /**
   * Called when avatar is being disposed
   * Subclasses should override to clean up resources
   */
  protected abstract onDispose(): void;
}
