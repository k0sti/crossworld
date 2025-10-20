import * as THREE from 'three';

export class CameraController {
  private camera: THREE.PerspectiveCamera;
  private canvas: HTMLCanvasElement;
  private enabled: boolean = false;

  // Movement state
  private moveForward = false;
  private moveBackward = false;
  private moveLeft = false;
  private moveRight = false;
  private moveUp = false;
  private moveDown = false;

  // Camera rotation
  private yaw = 0; // Left/right rotation
  private pitch = 0; // Up/down rotation

  // Movement speed
  private moveSpeed = 5.0; // units per second
  private lookSpeed = 0.002; // radians per pixel

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

    // Request pointer lock
    this.canvas.requestPointerLock();

    // Add event listeners
    document.addEventListener('keydown', this.boundKeyDown);
    document.addEventListener('keyup', this.boundKeyUp);
    document.addEventListener('mousemove', this.boundMouseMove);
    document.addEventListener('pointerlockchange', this.boundPointerLockChange);
  }

  disable(): void {
    if (!this.enabled) return;
    this.enabled = false;

    // Save camera state for next time
    this.saveCameraState();

    // Release pointer lock
    if (document.pointerLockElement === this.canvas) {
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
    document.removeEventListener('pointerlockchange', this.boundPointerLockChange);

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
      case 'q':
        this.moveDown = true;
        break;
      case 'e':
        this.moveUp = true;
        break;
      case 'escape':
        // This will be handled by the parent component
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
      case 'q':
        this.moveDown = false;
        break;
      case 'e':
        this.moveUp = false;
        break;
    }
  }

  private onMouseMove(event: MouseEvent): void {
    if (!this.enabled || document.pointerLockElement !== this.canvas) return;

    // Update camera rotation based on mouse movement
    this.yaw -= event.movementX * this.lookSpeed;
    this.pitch += event.movementY * this.lookSpeed; // Flipped: moving mouse up looks up

    // Clamp pitch to prevent looking directly up or down (80 degrees max)
    const maxPitch = Math.PI * 80 / 180; // 80 degrees in radians
    this.pitch = Math.max(-maxPitch, Math.min(maxPitch, this.pitch));

    this.updateCameraRotation();
  }

  private onPointerLockChange(): void {
    // If pointer lock is lost while enabled, disable camera mode
    if (this.enabled && document.pointerLockElement !== this.canvas) {
      // Don't disable here - let the parent component handle it via ESC
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

  update(deltaTime: number): void {
    if (!this.enabled) return;

    const moveDistance = this.moveSpeed * deltaTime;

    // Calculate movement direction based on camera orientation
    const forward = new THREE.Vector3(
      Math.sin(this.yaw),
      0,
      Math.cos(this.yaw)
    ).normalize();

    const right = new THREE.Vector3(
      Math.sin(this.yaw + Math.PI / 2),
      0,
      Math.cos(this.yaw + Math.PI / 2)
    ).normalize();

    const up = new THREE.Vector3(0, 1, 0);

    // Apply movement
    if (this.moveForward) {
      this.camera.position.addScaledVector(forward, moveDistance);
    }
    if (this.moveBackward) {
      this.camera.position.addScaledVector(forward, -moveDistance);
    }
    if (this.moveLeft) {
      this.camera.position.addScaledVector(right, -moveDistance);
    }
    if (this.moveRight) {
      this.camera.position.addScaledVector(right, moveDistance);
    }
    if (this.moveUp) {
      this.camera.position.addScaledVector(up, moveDistance);
    }
    if (this.moveDown) {
      this.camera.position.addScaledVector(up, -moveDistance);
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
