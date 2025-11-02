import * as THREE from 'three';
import type { CubeCoord } from '../types/cube-coord';
import { loadVoxFromUrl } from '../utils/voxLoader';

/**
 * Placement mode brush properties
 */
export interface PlacementBrush {
  modelPath: string | null;
  size: number;        // depth in octree (cursor size)
  scale: number;       // upscale depth (model scale)
  position: THREE.Vector3;
}

/**
 * Placement mode manager
 * Handles model placement with preview
 */
export class PlacementMode {
  private scene: THREE.Scene;
  private brush: PlacementBrush;

  // Model preview (voxel mesh + wireframe)
  private previewMesh: THREE.Mesh | null = null;
  private previewWireframe: THREE.LineSegments | null = null;
  private previewGroup: THREE.Group;

  private visible: boolean = false;
  private currentCursorCoord: CubeCoord | null = null;

  constructor(scene: THREE.Scene) {
    this.scene = scene;
    this.previewGroup = new THREE.Group();
    this.scene.add(this.previewGroup);

    // Initialize brush with defaults
    this.brush = {
      modelPath: null,
      size: 3,      // Default depth
      scale: 0,     // Default scale (no upscaling)
      position: new THREE.Vector3()
    };
  }

  /**
   * Set the model to place
   */
  async setModel(modelPath: string): Promise<void> {
    this.brush.modelPath = modelPath;

    // Clear existing preview
    this.clearPreview();

    if (!modelPath) return;

    try {
      // Load VOX model
      const fullPath = `/crossworld/assets/${modelPath}`;
      const voxData = await loadVoxFromUrl(fullPath);

      // Create mesh from voxel data
      const mesh = this.createMeshFromVoxData(voxData);
      if (mesh) {
        this.previewMesh = mesh;
        this.previewGroup.add(mesh);

        // Create wireframe for the model
        const edges = new THREE.EdgesGeometry(mesh.geometry);
        const lineMaterial = new THREE.LineBasicMaterial({
          color: 0x00ff00,
          linewidth: 2,
          depthTest: false,
          depthWrite: false
        });
        this.previewWireframe = new THREE.LineSegments(edges, lineMaterial);
        this.previewWireframe.renderOrder = 999;
        this.previewGroup.add(this.previewWireframe);

        this.updatePreviewScale();
      }
    } catch (error) {
      console.error('Failed to load model:', error);
    }
  }

  /**
   * Create Three.js mesh from VOX data
   */
  private createMeshFromVoxData(voxData: any): THREE.Mesh | null {
    if (!voxData || !voxData.vertices || !voxData.indices) {
      console.error('Invalid vox data');
      return null;
    }

    // Create geometry from vox data
    const geometry = new THREE.BufferGeometry();

    // Set vertices (positions)
    const positions = new Float32Array(voxData.vertices);
    geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));

    // Set indices
    const indices = new Uint32Array(voxData.indices);
    geometry.setIndex(new THREE.BufferAttribute(indices, 1));

    // Set normals if available
    if (voxData.normals && voxData.normals.length > 0) {
      const normals = new Float32Array(voxData.normals);
      geometry.setAttribute('normal', new THREE.BufferAttribute(normals, 3));
    } else {
      geometry.computeVertexNormals();
    }

    // Set colors if available
    if (voxData.colors && voxData.colors.length > 0) {
      const colors = new Float32Array(voxData.colors);
      geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));
    }

    // Create material with vertex colors
    const material = new THREE.MeshStandardMaterial({
      vertexColors: true,
      transparent: true,
      opacity: 0.7,
      side: THREE.DoubleSide
    });

    return new THREE.Mesh(geometry, material);
  }

  /**
   * Clear preview meshes
   */
  private clearPreview(): void {
    if (this.previewMesh) {
      this.previewGroup.remove(this.previewMesh);
      this.previewMesh.geometry.dispose();
      (this.previewMesh.material as THREE.Material).dispose();
      this.previewMesh = null;
    }
    if (this.previewWireframe) {
      this.previewGroup.remove(this.previewWireframe);
      this.previewWireframe.geometry.dispose();
      (this.previewWireframe.material as THREE.Material).dispose();
      this.previewWireframe = null;
    }
  }

  /**
   * Set brush size (depth)
   */
  setSize(size: number): void {
    this.brush.size = size;
    this.updatePreviewScale();
  }

  /**
   * Get brush size
   */
  getSize(): number {
    return this.brush.size;
  }

  /**
   * Set brush scale (upscale depth)
   */
  setScale(scale: number): void {
    this.brush.scale = scale;
    this.updatePreviewScale();
  }

  /**
   * Get brush scale
   */
  getScale(): number {
    return this.brush.scale;
  }

  /**
   * Update preview scale based on size and scale parameters
   */
  private updatePreviewScale(): void {
    if (!this.previewGroup) return;

    // Calculate final scale
    // size determines base voxel size: 2^size
    // scale adds additional scaling: 2^scale
    const baseSize = Math.pow(2, this.brush.size);
    const scaleMultiplier = Math.pow(2, this.brush.scale);
    const finalScale = baseSize * scaleMultiplier;

    this.previewGroup.scale.set(finalScale, finalScale, finalScale);
  }

  /**
   * Set position from raycast
   */
  setPosition(x: number, y: number, z: number): void {
    this.brush.position.set(x, y, z);
    this.previewGroup.position.copy(this.brush.position);
  }

  /**
   * Set cursor coordinate (octree coordinate)
   */
  setCursorCoord(coord: CubeCoord | null): void {
    this.currentCursorCoord = coord;
  }

  /**
   * Get current cursor coordinate
   */
  getCursorCoord(): CubeCoord | null {
    return this.currentCursorCoord;
  }

  /**
   * Show preview
   */
  show(): void {
    this.previewGroup.visible = true;
    this.visible = true;
  }

  /**
   * Hide preview
   */
  hide(): void {
    this.previewGroup.visible = false;
    this.visible = false;
  }

  /**
   * Check if preview is visible
   */
  isVisible(): boolean {
    return this.visible;
  }

  /**
   * Get current brush configuration
   */
  getBrush(): PlacementBrush {
    return { ...this.brush };
  }

  /**
   * Place the model at current position
   * Returns the placed model info for octree insertion
   */
  placeModel(): { coord: CubeCoord; modelPath: string } | null {
    if (!this.currentCursorCoord || !this.brush.modelPath) {
      return null;
    }

    return {
      coord: this.currentCursorCoord,
      modelPath: this.brush.modelPath
    };
  }

  /**
   * Cleanup resources
   */
  dispose(): void {
    this.clearPreview();
    this.scene.remove(this.previewGroup);
  }
}
