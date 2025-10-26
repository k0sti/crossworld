import * as THREE from 'three';

export class CameraController {
  private camera: THREE.PerspectiveCamera;
  private canvas: HTMLCanvasElement;
  private enabled: boolean = false;
  private onExitCallback: (() => void) | null = null;
  private usePointerLock: boolean = true;

  // Movement state
  private moveForward = false;
  private moveBackward = false;
  private moveLeft = false;
  private moveRight = false;
  private moveUp = false;
  private moveDown = false;

  // Force-based movement
  private moveForce: THREE.Vector3 = new THREE.Vector3(0, 0, 0);
  private forceMultiplier: number = 10000.0; // Constant multiplier for converting force to velocity

  // Camera rotation
  private yaw = 0; // Left/right rotation
  private pitch = 0; // Up/down rotation

  // Movement speed
  private moveSpeed = 5.0; // units per second
  private lookSpeed = 0.002; // radians per pixel

  // Smooth camera movement
  private targetPosition: THREE.Vector3 | null = null;
  private velocity: THREE.Vector3 = new THREE.Vector3(0, 0, 0);
  private smoothingFactor = 0.15; // Lower = smoother (0-1)
  private velocityDamping = 0.92; // Damping when no target

  // Saved camera state
  private savedPosition: THREE.Vector3 | null = null;
  private savedRotation: { yaw: number; pitch: number } | null = null;

  // Event handlers (need to store references for cleanup)
  private boundKeyDown: (e: KeyboardEvent) => void;
  private boundKeyUp: (e: KeyboardEvent) => void;
  private boundMouseMove: (e: MouseEvent) => void;
  private boundPointerLockChange: () => void;

  constructor(camera: THREE.PerspectiveCamera, canvas: HTMLCanvasElement) {
    this.camera = camera;
    this.canvas = canvas;

    // Bind event handlers
    this.boundKeyDown = this.onKeyDown.bind(this);
    this.boundKeyUp = this.onKeyUp.bind(this);
    this.boundMouseMove = this.onMouseMove.bind(this);
    this.boundPointerLockChange = this.onPointerLockChange.bind(this);
  }

  setOnExitCallback(callback: () => void): void {
    this.onExitCallback = callback;
  }

  setUsePointerLock(usePointerLock: boolean): void {
    this.usePointerLock = usePointerLock;
  }

  enable(): void {
    if (this.enabled) return;
    this.enabled = true;

    // Save current camera state
    this.savedPosition = this.camera.position.clone();
    const direction = new THREE.Vector3();
    this.camera.getWorldDirection(direction);
    this.savedRotation = {
      yaw: Math.atan2(direction.x, direction.z),
      pitch: Math.asin(-direction.y),
    };

    // Restore saved camera position if available (from previous camera mode session)
    const savedCameraData = this.loadCameraState();
    if (savedCameraData) {
      this.camera.position.copy(savedCameraData.position);
      this.yaw = savedCameraData.yaw;
      this.pitch = savedCameraData.pitch;
      this.updateCameraRotation();
    } else {
      // Initialize rotation from current camera orientation
      const direction = new THREE.Vector3();
      this.camera.getWorldDirection(direction);
      this.yaw = Math.atan2(direction.x, direction.z);
      this.pitch = Math.asin(-direction.y);
    }

    // Request pointer lock only if enabled
    if (this.usePointerLock) {
      this.canvas.requestPointerLock();
    }

    // Add event listeners
    document.addEventListener('keydown', this.boundKeyDown);
    document.addEventListener('keyup', this.boundKeyUp);
    document.addEventListener('mousemove', this.boundMouseMove);

    if (this.usePointerLock) {
      document.addEventListener('pointerlockchange', this.boundPointerLockChange);
    }
  }

  disable(): void {
    if (!this.enabled) return;
    this.enabled = false;

    // Save camera state for next time
    this.saveCameraState();

    // Release pointer lock
    if (this.usePointerLock && document.pointerLockElement === this.canvas) {
      document.exitPointerLock();
    }

    // Restore original camera state
    if (this.savedPosition && this.savedRotation) {
      this.camera.position.copy(this.savedPosition);
      this.yaw = this.savedRotation.yaw;
      this.pitch = this.savedRotation.pitch;
      this.updateCameraRotation();
    }

    // Remove event listeners
    document.removeEventListener('keydown', this.boundKeyDown);
    document.removeEventListener('keyup', this.boundKeyUp);
    document.removeEventListener('mousemove', this.boundMouseMove);

    if (this.usePointerLock) {
      document.removeEventListener('pointerlockchange', this.boundPointerLockChange);
    }

    // Reset movement state
    this.resetMovementState();
  }

  private onKeyDown(event: KeyboardEvent): void {
    if (!this.enabled) return;

    switch (event.key.toLowerCase()) {
      case 'w':
        this.moveForward = true;
        break;
      case 's':
        this.moveBackward = true;
        break;
      case 'a':
        this.moveLeft = true;
        break;
      case 'd':
        this.moveRight = true;
        break;
      case 'v':
        this.moveDown = true;
        break;
      case 'f':
        this.moveUp = true;
        break;
      case 'escape':
        // This will be handled by pointer lock change
        break;
    }
  }

  private onKeyUp(event: KeyboardEvent): void {
    if (!this.enabled) return;

    switch (event.key.toLowerCase()) {
      case 'w':
        this.moveForward = false;
        break;
      case 's':
        this.moveBackward = false;
        break;
      case 'a':
        this.moveLeft = false;
        break;
      case 'd':
        this.moveRight = false;
        break;
      case 'v':
        this.moveDown = false;
        break;
      case 'f':
        this.moveUp = false;
        break;
    }
  }

  private onMouseMove(event: MouseEvent): void {
    if (!this.enabled) return;

    // In pointer lock mode, require pointer lock
    if (this.usePointerLock && document.pointerLockElement !== this.canvas) return;

    // In non-pointer-lock mode, don't rotate camera (mouselook handled by scene manager)
    if (!this.usePointerLock) {
      return;
    }

    // Update camera rotation based on mouse movement
    this.yaw -= event.movementX * this.lookSpeed;
    this.pitch += event.movementY * this.lookSpeed; // Flipped: moving mouse up looks up

    // Clamp pitch to prevent looking directly up or down (80 degrees max)
    const maxPitch = Math.PI * 80 / 180; // 80 degrees in radians
    this.pitch = Math.max(-maxPitch, Math.min(maxPitch, this.pitch));

    this.updateCameraRotation();
  }

  private onPointerLockChange(): void {
    // If pointer lock is lost while enabled, exit camera mode
    if (this.enabled && document.pointerLockElement !== this.canvas) {
      this.disable();
      // Notify parent component that camera mode was exited
      if (this.onExitCallback) {
        this.onExitCallback();
      }
    }
  }

  private updateCameraRotation(): void {
    // Apply rotation to camera
    const direction = new THREE.Vector3(
      Math.sin(this.yaw) * Math.cos(this.pitch),
      -Math.sin(this.pitch),
      Math.cos(this.yaw) * Math.cos(this.pitch)
    );

    const target = this.camera.position.clone().add(direction);
    this.camera.lookAt(target);
  }

  /**
   * Set target position for smooth camera movement
   * Camera will smoothly move to this position while maintaining orientation
   */
  setTargetPosition(position: THREE.Vector3): void {
    this.targetPosition = position.clone();
  }

  /**
   * Clear target position (camera stops smoothly)
   */
  clearTargetPosition(): void {
    this.targetPosition = null;
  }

  update(deltaTime: number): void {
    if (!this.enabled) return;

    // Smooth movement towards target position (if set)
    if (this.targetPosition) {
      const toTarget = new THREE.Vector3().subVectors(this.targetPosition, this.camera.position);
      const distance = toTarget.length();

      if (distance < 0.01) {
        // Reached target, snap to it
        this.camera.position.copy(this.targetPosition);
        this.velocity.set(0, 0, 0);
        this.targetPosition = null;
      } else {
        // Calculate desired velocity towards target
        const desiredVelocity = toTarget.normalize().multiplyScalar(this.moveSpeed);

        // Smoothly interpolate velocity
        this.velocity.lerp(desiredVelocity, this.smoothingFactor);

        // Update position
        this.camera.position.addScaledVector(this.velocity, deltaTime);
      }
    } else {
      // Calculate movement direction based on camera's actual orientation
      const direction = new THREE.Vector3();
      this.camera.getWorldDirection(direction);

      // Forward direction (projected onto XZ plane)
      const forward = new THREE.Vector3(direction.x, 0, direction.z).normalize();

      // Right direction (perpendicular to forward on XZ plane)
      const right = new THREE.Vector3(-forward.z, 0, forward.x).normalize();

      const up = new THREE.Vector3(0, 1, 0);

      // Calculate move force from keyboard input
      this.moveForce.set(0, 0, 0);

      if (this.moveForward) {
        this.moveForce.add(forward);
      }
      if (this.moveBackward) {
        this.moveForce.sub(forward);
      }
      if (this.moveLeft) {
        this.moveForce.sub(right);
      }
      if (this.moveRight) {
        this.moveForce.add(right);
      }
      if (this.moveUp) {
        this.moveForce.add(up);
      }
      if (this.moveDown) {
        this.moveForce.sub(up);
      }

      // Add force to velocity with constant multiplier
      this.velocity.addScaledVector(this.moveForce, this.forceMultiplier * deltaTime);

      // Apply damping to velocity
      this.velocity.multiplyScalar(this.velocityDamping);

      // Clamp velocity to max speed
      const currentSpeed = this.velocity.length();
      if (currentSpeed > this.moveSpeed) {
        this.velocity.normalize().multiplyScalar(this.moveSpeed);
      }

      // Update position
      this.camera.position.addScaledVector(this.velocity, deltaTime);

      // Zero out very small velocities
      if (this.velocity.length() < 0.01) {
        this.velocity.set(0, 0, 0);
      }
    }
  }

  private resetMovementState(): void {
    this.moveForward = false;
    this.moveBackward = false;
    this.moveLeft = false;
    this.moveRight = false;
    this.moveUp = false;
    this.moveDown = false;
  }

  private saveCameraState(): void {
    const state = {
      position: {
        x: this.camera.position.x,
        y: this.camera.position.y,
        z: this.camera.position.z,
      },
      yaw: this.yaw,
      pitch: this.pitch,
    };
    localStorage.setItem('cameraControllerState', JSON.stringify(state));
  }

  private loadCameraState(): { position: THREE.Vector3; yaw: number; pitch: number } | null {
    const saved = localStorage.getItem('cameraControllerState');
    if (!saved) return null;

    try {
      const state = JSON.parse(saved);
      return {
        position: new THREE.Vector3(state.position.x, state.position.y, state.position.z),
        yaw: state.yaw,
        pitch: state.pitch,
      };
    } catch (error) {
      console.error('Failed to load camera state:', error);
      return null;
    }
  }

  isEnabled(): boolean {
    return this.enabled;
  }
}
