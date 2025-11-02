import * as THREE from 'three';
import type { CubeCoord } from '../types/cube-coord';
import { VoxelCursor } from './cursor';

/**
 * Edit mode brush properties
 */
export interface EditBrush {
  size: number;          // depth in octree (cursor size)
  paletteIndex: number;  // selected color from palette
}

/**
 * Edit mode manager
 * Handles voxel editing with brush properties
 */
export class EditMode {
  private cursor: VoxelCursor;
  private brush: EditBrush;

  constructor(cursor: VoxelCursor) {
    this.cursor = cursor;

    // Initialize brush with defaults
    this.brush = {
      size: 3,           // Default depth
      paletteIndex: 0    // Default color index
    };

    // Sync cursor with brush size
    this.cursor.setDepth(this.brush.size);
  }

  /**
   * Set brush size (depth)
   */
  setSize(size: number): void {
    this.brush.size = size;
    this.cursor.setDepth(size);
  }

  /**
   * Get brush size
   */
  getSize(): number {
    return this.brush.size;
  }

  /**
   * Get cursor size in world units
   */
  getCursorSize(): number {
    return this.cursor.getCursorSize();
  }

  /**
   * Set palette index (color)
   */
  setPaletteIndex(index: number): void {
    this.brush.paletteIndex = index;
  }

  /**
   * Get palette index
   */
  getPaletteIndex(): number {
    return this.brush.paletteIndex;
  }

  /**
   * Get current brush configuration
   */
  getBrush(): EditBrush {
    return { ...this.brush };
  }

  /**
   * Position cursor at a specific world position
   */
  setCursorPosition(x: number, y: number, z: number): void {
    this.cursor.setPosition(x, y, z);
  }

  /**
   * Set cursor coordinate (octree coordinate)
   */
  setCursorCoord(coord: CubeCoord | null): void {
    this.cursor.setCursorCoord(coord);
  }

  /**
   * Get current cursor coordinate
   */
  getCursorCoord(): CubeCoord | null {
    return this.cursor.getCursorCoord();
  }

  /**
   * Update face highlight
   */
  updateFaceHighlight(point: THREE.Vector3, normal: THREE.Vector3, size: number): void {
    this.cursor.updateFaceHighlight(point, normal, size);
  }

  /**
   * Hide face highlight
   */
  hideFaceHighlight(): void {
    this.cursor.hideFaceHighlight();
  }

  /**
   * Show cursor
   */
  show(): void {
    this.cursor.show();
  }

  /**
   * Hide cursor
   */
  hide(): void {
    this.cursor.hide();
  }

  /**
   * Check if cursor is visible
   */
  isVisible(): boolean {
    return this.cursor.isVisible();
  }

  /**
   * Get cursor reference
   */
  getCursor(): VoxelCursor {
    return this.cursor;
  }

  /**
   * Paint or erase voxel at current cursor position
   * Returns action info for octree modification
   */
  paintVoxel(erase: boolean = false): { coord: CubeCoord; colorIndex: number; erase: boolean } | null {
    const coord = this.cursor.getCursorCoord();
    if (!coord) return null;

    return {
      coord,
      colorIndex: erase ? -1 : this.brush.paletteIndex,
      erase
    };
  }
}
