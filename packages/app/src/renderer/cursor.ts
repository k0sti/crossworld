import * as THREE from 'three';
import type { CubeCoord } from '../types/cube-coord';

/**
 * Cursor manager for voxel editing
 * Handles the preview cube (wireframe) and face highlighting
 */
export class VoxelCursor {
  private scene: THREE.Scene;
  private previewCube: THREE.LineSegments | null = null;
  private faceHighlightMesh: THREE.Mesh | null = null;
  private currentGridPosition: THREE.Vector3 = new THREE.Vector3();
  private currentCursorCoord: CubeCoord | null = null;
  private cursorDepth: number = 3; // Default depth
  private visible: boolean = false;

  constructor(scene: THREE.Scene, initialDepth: number = 3) {
    this.scene = scene;
    this.cursorDepth = initialDepth;
    this.setupPreviewCube();
    this.setupFaceHighlight();
  }

  /**
   * Create wireframe preview cube for voxel cursor
   */
  private setupPreviewCube(): void {
    const geometry = new THREE.BoxGeometry(1, 1, 1);
    const edges = new THREE.EdgesGeometry(geometry);
    const lineMaterial = new THREE.LineBasicMaterial({
      color: 0x00ff00,
      linewidth: 2,
      depthTest: false,
      depthWrite: false
    });
    this.previewCube = new THREE.LineSegments(edges, lineMaterial);
    this.previewCube.visible = false;
    this.previewCube.renderOrder = 999; // Render last
    this.scene.add(this.previewCube);
  }

  /**
   * Setup face highlight mesh (shown when hovering over a voxel face)
   */
  private setupFaceHighlight(): void {
    const geometry = new THREE.PlaneGeometry(1, 1);
    const material = new THREE.MeshBasicMaterial({
      color: 0x00ffff,
      transparent: true,
      opacity: 0.3,
      side: THREE.DoubleSide,
      depthTest: false,
      depthWrite: false
    });
    this.faceHighlightMesh = new THREE.Mesh(geometry, material);
    this.faceHighlightMesh.visible = false;
    this.faceHighlightMesh.renderOrder = 998; // Below voxel cursor (999)
    this.scene.add(this.faceHighlightMesh);
  }

  /**
   * Set cursor depth (voxel size)
   */
  setDepth(depth: number): void {
    this.cursorDepth = depth;
    this.updateCursorSize();
  }

  /**
   * Get current cursor depth
   */
  getDepth(): number {
    return this.cursorDepth;
  }

  /**
   * Get cursor size based on depth
   */
  getCursorSize(): number {
    return Math.pow(2, this.cursorDepth);
  }

  /**
   * Update cursor size based on depth
   */
  private updateCursorSize(): void {
    if (!this.previewCube) return;

    const size = this.getCursorSize();
    const scale = size;
    this.previewCube.scale.set(scale, scale, scale);

    // Recalculate cursor coordinate with new depth if cursor is visible
    if (this.currentCursorCoord && this.previewCube.visible) {
      const halfSize = size / 2;
      const voxelCenterX = this.currentGridPosition.x;
      const voxelCenterZ = this.currentGridPosition.z;

      // Convert depth to Y position (scale = 2^depth)
      const totalLevels = 1 << this.cursorDepth;
      const y = this.currentCursorCoord.y;
      const voxelY = (-totalLevels + y * 2 + 1) * halfSize;

      // Update preview cube position
      this.currentGridPosition.set(voxelCenterX, voxelY + halfSize, voxelCenterZ);
      this.previewCube.position.copy(this.currentGridPosition);
    }
  }

  /**
   * Position cursor at a specific world position
   */
  setPosition(x: number, y: number, z: number): void {
    this.currentGridPosition.set(x, y, z);
    if (this.previewCube) {
      this.previewCube.position.copy(this.currentGridPosition);
    }
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
   * Get current grid position
   */
  getGridPosition(): THREE.Vector3 {
    return this.currentGridPosition.clone();
  }

  /**
   * Show cursor
   */
  show(): void {
    if (this.previewCube) {
      this.previewCube.visible = true;
      this.visible = true;
    }
  }

  /**
   * Hide cursor
   */
  hide(): void {
    if (this.previewCube) {
      this.previewCube.visible = false;
      this.visible = false;
    }
    this.hideFaceHighlight();
  }

  /**
   * Check if cursor is visible
   */
  isVisible(): boolean {
    return this.visible;
  }

  /**
   * Update face highlight based on hit point and normal
   */
  updateFaceHighlight(point: THREE.Vector3, normal: THREE.Vector3, size: number): void {
    if (!this.faceHighlightMesh) return;

    // Offset the face highlight slightly along the normal to avoid z-fighting
    const offset = 0.01;
    const position = point.clone().add(normal.clone().multiplyScalar(offset));

    // Calculate rotation quaternion to align plane with face
    const quaternion = new THREE.Quaternion();
    if (Math.abs(normal.y) > 0.99) {
      // Top or bottom face - use default orientation
      quaternion.setFromAxisAngle(new THREE.Vector3(1, 0, 0), normal.y > 0 ? -Math.PI / 2 : Math.PI / 2);
    } else if (Math.abs(normal.x) > 0.99) {
      // Left or right face
      quaternion.setFromAxisAngle(new THREE.Vector3(0, 1, 0), normal.x > 0 ? Math.PI / 2 : -Math.PI / 2);
    } else {
      // Front or back face
      quaternion.setFromAxisAngle(new THREE.Vector3(0, 1, 0), normal.z > 0 ? 0 : Math.PI);
    }

    // Update face highlight mesh
    this.faceHighlightMesh.position.copy(position);
    this.faceHighlightMesh.scale.set(size, size, 1);
    this.faceHighlightMesh.setRotationFromQuaternion(quaternion);
    this.faceHighlightMesh.visible = true;
  }

  /**
   * Hide face highlight
   */
  hideFaceHighlight(): void {
    if (this.faceHighlightMesh) {
      this.faceHighlightMesh.visible = false;
    }
  }

  /**
   * Cleanup resources
   */
  dispose(): void {
    if (this.previewCube) {
      this.scene.remove(this.previewCube);
      this.previewCube.geometry.dispose();
      (this.previewCube.material as THREE.Material).dispose();
      this.previewCube = null;
    }
    if (this.faceHighlightMesh) {
      this.scene.remove(this.faceHighlightMesh);
      this.faceHighlightMesh.geometry.dispose();
      (this.faceHighlightMesh.material as THREE.Material).dispose();
      this.faceHighlightMesh = null;
    }
  }
}
