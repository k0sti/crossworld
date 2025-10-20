import * as THREE from 'three';

export type TeleportAnimationType = 'fade' | 'scale' | 'spin' | 'slide' | 'burst';

// Teleport animation duration in milliseconds
export const TELEPORT_ANIMATION_DURATION_MS = 300;

/**
 * Manages teleport animation for an avatar
 * Creates a ghost that fades out at old position while real avatar fades in at new position
 */
export class TeleportAnimation {
  private startTime: number = 0;
  private isAnimating: boolean = false;
  private animationType: TeleportAnimationType;
  private fadeInObject: THREE.Object3D; // The real avatar (fading in at new position)
  private fadeOutGhost: THREE.Group | null = null; // Ghost clone (fading out at old position)
  private originalScale: THREE.Vector3;
  private originalY: number; // Target Y position to restore after animation
  private ghostOriginalY: number = 0; // Original Y position of ghost
  private scene: THREE.Scene;
  private firstUpdate: boolean = true;

  constructor(object: THREE.Object3D, scene: THREE.Scene, animationType: TeleportAnimationType = 'fade') {
    this.fadeInObject = object;
    this.scene = scene;
    this.animationType = animationType;
    this.originalScale = object.scale.clone();
    this.originalY = object.position.y; // Will be updated in first update() call
  }

  /**
   * Start teleport animation
   * Creates ghost at old position and immediately returns (no waiting)
   */
  start(): void {
    this.startTime = performance.now();
    this.isAnimating = true;

    // Set fade-in object to invisible (will fade in during animation)
    this.setOpacity(this.fadeInObject, 0);

    // Create ghost clone at current position/rotation
    this.fadeOutGhost = this.createGhost();
    if (this.fadeOutGhost) {
      this.ghostOriginalY = this.fadeOutGhost.position.y;
      this.scene.add(this.fadeOutGhost);
    }
  }

  /**
   * Create a ghost clone of the object for fade-out animation
   */
  private createGhost(): THREE.Group | null {
    const ghost = new THREE.Group();

    // Clone the object's position and rotation
    ghost.position.copy(this.fadeInObject.position);
    ghost.quaternion.copy(this.fadeInObject.quaternion);
    ghost.scale.copy(this.fadeInObject.scale);

    // Clone all meshes from the object
    this.fadeInObject.traverse((child) => {
      if ((child as THREE.Mesh).isMesh) {
        const mesh = child as THREE.Mesh;
        const clonedGeometry = mesh.geometry.clone();
        const clonedMaterial = Array.isArray(mesh.material)
          ? mesh.material.map(m => m.clone())
          : mesh.material.clone();

        const clonedMesh = new THREE.Mesh(clonedGeometry, clonedMaterial);
        clonedMesh.position.copy(mesh.position);
        clonedMesh.quaternion.copy(mesh.quaternion);
        clonedMesh.scale.copy(mesh.scale);

        ghost.add(clonedMesh);
      }
    });

    return ghost;
  }

  /**
   * Update animation state
   * @param currentTime Current time in milliseconds
   * @returns true if animation is still running
   */
  update(currentTime: number): boolean {
    if (!this.isAnimating) return false;

    // On first update, capture the NEW Y position (after teleport move)
    if (this.firstUpdate) {
      this.originalY = this.fadeInObject.position.y;
      this.firstUpdate = false;
    }

    const elapsed = currentTime - this.startTime;
    const progress = Math.min(elapsed / TELEPORT_ANIMATION_DURATION_MS, 1.0);

    // Animate fade-out on ghost (progress 0 -> 1)
    if (this.fadeOutGhost) {
      this.applyFadeOutAnimation(this.fadeOutGhost, progress);
    }

    // Animate fade-in on real object (progress 0 -> 1)
    this.applyFadeInAnimation(this.fadeInObject, progress);

    if (progress >= 1.0) {
      this.isAnimating = false;
      this.cleanup();
      return false;
    }

    return true;
  }

  private applyFadeOutAnimation(object: THREE.Object3D, progress: number): void {
    const originalScale = this.originalScale;

    switch (this.animationType) {
      case 'fade':
        this.setOpacity(object, 1 - progress);
        break;
      case 'scale':
        object.scale.copy(originalScale).multiplyScalar(1 - progress);
        this.setOpacity(object, 1 - progress);
        break;
      case 'spin':
        object.rotation.y += progress * Math.PI * 2;
        this.setOpacity(object, 1 - progress);
        break;
      case 'slide':
        object.position.y = this.ghostOriginalY - progress * 2;
        this.setOpacity(object, 1 - progress);
        break;
      case 'burst':
        object.scale.copy(originalScale).multiplyScalar(1 + progress * 0.3);
        this.setOpacity(object, 1 - progress);
        break;
    }
  }

  private applyFadeInAnimation(object: THREE.Object3D, progress: number): void {
    const originalScale = this.originalScale;

    switch (this.animationType) {
      case 'fade':
        this.setOpacity(object, progress);
        break;
      case 'scale':
        object.scale.copy(originalScale).multiplyScalar(progress);
        this.setOpacity(object, progress);
        break;
      case 'spin':
        // Reverse spin direction for fade-in
        object.rotation.y -= (1 - progress) * Math.PI * 2;
        this.setOpacity(object, progress);
        break;
      case 'slide':
        object.position.y = this.originalY + (1 - progress) * 2;
        this.setOpacity(object, progress);
        break;
      case 'burst':
        object.scale.copy(originalScale).multiplyScalar(1 + (1 - progress) * 0.3);
        this.setOpacity(object, progress);
        break;
    }
  }

  private setOpacity(object: THREE.Object3D, opacity: number): void {
    object.traverse((child) => {
      if ((child as THREE.Mesh).isMesh) {
        const mesh = child as THREE.Mesh;
        if (Array.isArray(mesh.material)) {
          mesh.material.forEach(mat => {
            mat.transparent = true;
            mat.opacity = opacity;
          });
        } else {
          mesh.material.transparent = true;
          mesh.material.opacity = opacity;
        }
      }
    });
  }

  private cleanup(): void {
    // Remove and dispose ghost
    if (this.fadeOutGhost) {
      this.scene.remove(this.fadeOutGhost);
      this.fadeOutGhost.traverse((child) => {
        if ((child as THREE.Mesh).isMesh) {
          const mesh = child as THREE.Mesh;
          mesh.geometry.dispose();
          if (Array.isArray(mesh.material)) {
            mesh.material.forEach(m => m.dispose());
          } else {
            mesh.material.dispose();
          }
        }
      });
      this.fadeOutGhost = null;
    }

    // Reset real object to full opacity, original scale, and original Y position
    this.fadeInObject.scale.copy(this.originalScale);
    this.fadeInObject.position.y = this.originalY;
    this.setOpacity(this.fadeInObject, 1.0);
  }

  isActive(): boolean {
    return this.isAnimating;
  }

  /**
   * Cancel the animation early and clean up
   */
  cancel(): void {
    if (this.isAnimating) {
      this.isAnimating = false;
      this.cleanup();
    }
  }
}
