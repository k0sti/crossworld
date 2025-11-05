import * as THREE from 'three';

/**
 * Transform - Represents position and rotation in 3D space
 *
 * Provides both 3D (position vector + quaternion) and 2D helpers (x, z, angle)
 * for working with avatar transforms in the world.
 */
export class Transform {
  private position: THREE.Vector3;
  private rotation: THREE.Quaternion;

  constructor(x: number = 0, y: number = 0, z: number = 0, quaternion?: THREE.Quaternion) {
    this.position = new THREE.Vector3(x, y, z);
    this.rotation = quaternion ? quaternion.clone() : new THREE.Quaternion();
  }

  /**
   * Create Transform from another Transform
   */
  static fromTransform(other: Transform): Transform {
    return new Transform(
      other.position.x,
      other.position.y,
      other.position.z,
      other.rotation
    );
  }

  /**
   * Create Transform from event data (position object with optional quaternion)
   */
  static fromEventData(data: {
    x: number;
    y: number;
    z: number;
    quaternion?: [number, number, number, number];
  }): Transform {
    let quaternion: THREE.Quaternion | undefined;
    if (data.quaternion) {
      quaternion = new THREE.Quaternion(
        data.quaternion[0],
        data.quaternion[1],
        data.quaternion[2],
        data.quaternion[3]
      );
    }
    return new Transform(data.x, data.y, data.z, quaternion);
  }

  // === 3D Getters ===

  getPosition(): THREE.Vector3 {
    return this.position.clone();
  }

  getRotation(): THREE.Quaternion {
    return this.rotation.clone();
  }

  // === 3D Setters ===

  setPosition(x: number, y: number, z: number): void {
    this.position.set(x, y, z);
  }

  setRotation(quaternion: THREE.Quaternion): void {
    this.rotation.copy(quaternion);
  }

  setFromVector3AndQuaternion(position: THREE.Vector3, quaternion: THREE.Quaternion): void {
    this.position.copy(position);
    this.rotation.copy(quaternion);
  }

  // === 2D Helpers (x, z, angle) ===

  /**
   * Get X coordinate (2D horizontal)
   */
  getX(): number {
    return this.position.x;
  }

  /**
   * Get Y coordinate (vertical/height)
   */
  getY(): number {
    return this.position.y;
  }

  /**
   * Get Z coordinate (2D depth)
   */
  getZ(): number {
    return this.position.z;
  }

  /**
   * Set 2D position (x, z), keeping y unchanged
   */
  setXZ(x: number, z: number): void {
    this.position.x = x;
    this.position.z = z;
  }

  /**
   * Set Y coordinate (vertical/height)
   */
  setY(y: number): void {
    this.position.y = y;
  }

  /**
   * Get Y rotation angle in radians (rotation around vertical axis)
   */
  getAngle(): number {
    const euler = new THREE.Euler().setFromQuaternion(this.rotation, 'YXZ');
    return euler.y;
  }

  /**
   * Set Y rotation angle in radians (rotation around vertical axis)
   */
  setAngle(angleRadians: number): void {
    this.rotation.setFromAxisAngle(new THREE.Vector3(0, 1, 0), angleRadians);
  }

  /**
   * Get direction vector based on current rotation (2D, in XZ plane)
   */
  getDirection2D(): THREE.Vector2 {
    const angle = this.getAngle();
    return new THREE.Vector2(Math.sin(angle), Math.cos(angle));
  }

  // === Utility Methods ===

  /**
   * Clone this transform
   */
  clone(): Transform {
    return Transform.fromTransform(this);
  }

  /**
   * Apply this transform to a Three.js Object3D
   */
  applyToObject3D(object: THREE.Object3D): void {
    object.position.copy(this.position);
    object.quaternion.copy(this.rotation);
  }

  /**
   * Extract transform from a Three.js Object3D
   */
  static fromObject3D(object: THREE.Object3D): Transform {
    return new Transform(
      object.position.x,
      object.position.y,
      object.position.z,
      object.quaternion
    );
  }

  /**
   * Convert to event data format (for publishing to Nostr)
   */
  toEventData(): {
    x: number;
    y: number;
    z: number;
    quaternion: [number, number, number, number];
  } {
    return {
      x: this.position.x,
      y: this.position.y,
      z: this.position.z,
      quaternion: [
        this.rotation.x,
        this.rotation.y,
        this.rotation.z,
        this.rotation.w,
      ],
    };
  }

  /**
   * Get distance to another transform (in XZ plane for 2D distance)
   */
  distanceTo2D(other: Transform): number {
    const dx = this.position.x - other.position.x;
    const dz = this.position.z - other.position.z;
    return Math.sqrt(dx * dx + dz * dz);
  }

  /**
   * Get full 3D distance to another transform
   */
  distanceTo3D(other: Transform): number {
    return this.position.distanceTo(other.position);
  }
}
