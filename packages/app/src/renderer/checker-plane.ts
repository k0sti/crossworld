import * as THREE from 'three';

/**
 * CheckerPlane - A checkerboard pattern ground plane centered at origin
 *
 * Coordinate system:
 * - Size: 2^macroDepth × 2^macroDepth world units
 * - Centered at origin (0, 0, 0)
 * - Y position is configurable (default: 0)
 */
export class CheckerPlane {
  private mesh: THREE.Mesh;
  private texture: THREE.CanvasTexture;

  constructor(size: number = 8, segments: number = 8, yPosition: number = 0.02) {
    // Create checkerboard texture using canvas
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d')!;

    // Set canvas size to match segments (one pixel per square)
    canvas.width = segments;
    canvas.height = segments;

    // Draw checkerboard pattern
    for (let x = 0; x < segments; x++) {
      for (let z = 0; z < segments; z++) {
        const isLight = (x + z) % 2 === 0;
        ctx.fillStyle = isLight ? '#ffffff' : '#000000';
        ctx.fillRect(x, z, 1, 1);
      }
    }

    // Create texture from canvas
    this.texture = new THREE.CanvasTexture(canvas);
    this.texture.magFilter = THREE.NearestFilter;
    this.texture.minFilter = THREE.NearestFilter;
    this.texture.wrapS = THREE.ClampToEdgeWrapping;
    this.texture.wrapT = THREE.ClampToEdgeWrapping;

    // Create plane geometry centered at origin
    const geometry = new THREE.PlaneGeometry(size, size, 1, 1);
    geometry.rotateX(-Math.PI / 2); // Rotate to be horizontal

    // Create material with checkerboard texture (50% transparency)
    const material = new THREE.MeshBasicMaterial({
      map: this.texture,
      transparent: true,
      opacity: 0.1,
      side: THREE.DoubleSide,
      depthWrite: false
    });

    // Create mesh centered at origin, raised slightly above y=0
    this.mesh = new THREE.Mesh(geometry, material);
    this.mesh.position.set(0, yPosition, 0);
    this.mesh.receiveShadow = true;

    // Ensure proper render order (render after world cube)
    this.mesh.renderOrder = 1;
  }

  /**
   * Get the THREE.Mesh object to add to scene
   */
  getMesh(): THREE.Mesh {
    return this.mesh;
  }

  /**
   * Set visibility of the checker plane
   */
  setVisible(visible: boolean): void {
    this.mesh.visible = visible;
  }

  /**
   * Set opacity of the checker plane
   */
  setOpacity(opacity: number): void {
    const material = this.mesh.material as THREE.MeshBasicMaterial;
    material.opacity = opacity;
  }

  /**
   * Dispose of resources
   */
  dispose(): void {
    this.mesh.geometry.dispose();
    this.texture.dispose();
    if (this.mesh.material instanceof THREE.Material) {
      this.mesh.material.dispose();
    }
  }
}
