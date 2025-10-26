import * as THREE from 'three';

/**
 * CheckerPlane - A checkerboard pattern ground plane centered at origin
 *
 * Coordinate system:
 * - Size: 128×128 world units (for depth 7)
 * - Centered at origin (0, 0, 0)
 * - Extends from [-64, 64] in X and Z axes
 * - Y position is configurable (default: 0)
 */
export class CheckerPlane {
  private mesh: THREE.Mesh;

  constructor(size: number = 128, segments: number = 128, yPosition: number = 0.02) {
    // Create plane geometry centered at origin
    const geometry = new THREE.PlaneGeometry(size, size, segments, segments);
    geometry.rotateX(-Math.PI / 2); // Rotate to be horizontal

    // Create checkerboard pattern with vertex colors
    const colors = new Float32Array(geometry.attributes.position.count * 3);
    const positions = geometry.attributes.position;

    for (let i = 0; i < positions.count; i++) {
      const worldX = positions.getX(i);
      const worldZ = positions.getZ(i);

      // Convert world position to grid coordinates (centered at origin)
      // World coords are [-64, 64], shift to [0, 128] for grid indexing
      const gridX = Math.floor(worldX + size / 2);
      const gridZ = Math.floor(worldZ + size / 2);

      // Create checkerboard pattern
      const isLight = (gridX + gridZ) % 2 === 0;
      const color = isLight ? 0.9 : 0.5;

      colors[i * 3] = color;
      colors[i * 3 + 1] = color;
      colors[i * 3 + 2] = color;
    }

    geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));

    // Create material with vertex colors
    const material = new THREE.MeshPhongMaterial({
      vertexColors: true,
      transparent: true,
      opacity: 0.4,
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
    const material = this.mesh.material as THREE.MeshPhongMaterial;
    material.opacity = opacity;
  }

  /**
   * Dispose of resources
   */
  dispose(): void {
    this.mesh.geometry.dispose();
    if (this.mesh.material instanceof THREE.Material) {
      this.mesh.material.dispose();
    }
  }
}
