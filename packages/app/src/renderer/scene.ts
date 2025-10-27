import * as THREE from 'three';
import { Avatar } from './avatar';
import { VoxelAvatar } from './voxel-avatar';
import type { IAvatar } from './base-avatar';
import type { AvatarEngine } from '@workspace/wasm';
import type { AvatarState } from '../services/avatar-state';
import { Transform } from './transform';
import type { TeleportAnimationType } from './teleport-animation';
import { CameraController } from './camera-controller';
import {
  worldToCube,
  cubeToWorld,
  type CubeCoord,
  isWithinWorldBounds,
  snapToGrid,
  getVoxelSize as getVoxelSizeFromCubeCoord
} from '../types/cube-coord';
import { getWorldSize } from '../constants/geometry';
import { getMacroDepth, getMicroDepth, getTotalDepth } from '../config/depth-config';
import { CheckerPlane } from './checker-plane';
import { SunSystem } from './sun-system';
import { PostProcessing } from './post-processing';
import { profileCache } from '../services/profile-cache';
import { DEFAULT_RELAYS } from '../config';

/**
 * SceneManager - Manages the 3D scene with centered coordinate system
 *
 * Coordinate System:
 * - World space: All coordinates in range [-HALF_WORLD, HALF_WORLD] for X, Y, Z axes (WORLD_SIZE units)
 * - Origin (0, 0, 0): Center of the world cube at ground level
 * - CheckerPlane: Centered at origin with pivot at (0, 0, 0)
 * - Cube world mesh: Centered at origin with pivot at (0, 0, 0)
 * - Voxel cursor: Snaps to grid centered on raycast intersection point
 * - At max depth (SUBDIVISION_DEPTH), 1 voxel = 1 world unit
 *
 * Voxel placement uses corner coordinates (not center), which are then
 * converted to octree coordinates via CubeCoord system.
 */
export class SceneManager {
  private scene: THREE.Scene;
  private camera: THREE.PerspectiveCamera;
  private renderer: THREE.WebGLRenderer;
  private geometryMesh: THREE.Mesh | null = null;
  private checkerPlane: CheckerPlane | null = null;
  private groundPlane: THREE.Plane = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0); // Plane at y=0
  private currentAvatar: IAvatar | null = null;
  private avatarEngine: AvatarEngine | null = null;
  private raycaster: THREE.Raycaster;
  private mouse: THREE.Vector2;
  private lastTime: number = 0;
  private isEditMode: boolean = false;
  private previewCube: THREE.LineSegments | null = null;
  private currentGridPosition: THREE.Vector3 = new THREE.Vector3();
  private onPositionUpdate?: (x: number, y: number, z: number, quaternion: [number, number, number, number], moveStyle?: string) => void;
  private cameraController: CameraController | null = null;
  private selectedColorIndex: number = 0;

  // Continuous paint state
  private isLeftMousePressed: boolean = false;
  private isRightMousePressed: boolean = false;
  private lastPaintedVoxel: { x: number; y: number; z: number } | null = null;

  // Mouse mode: 1 = free pointer (paint/erase), 2 = grabbed pointer (camera rotation)
  private mouseMode: 1 | 2 = 1;
  private crosshair: HTMLElement | null = null;
  private shiftKeyPressed: boolean = false;

  // Depth voxel select mode: 1 = near side (y=0), 2 = far side (y=-1)
  private depthSelectMode: 1 | 2 = 1;

  // Cursor depth - single source of truth for current cursor depth
  // depth can be 0 to totalDepth (macro+micro, smaller depth = larger voxel size)
  // initialized to macroDepth (3)
  private cursorDepth: number = getMacroDepth();

  // Current cursor coordinate (null when not in edit mode or cursor not visible)
  private currentCursorCoord: CubeCoord | null = null;

  // Remote avatars for other users
  private remoteAvatars: Map<string, IAvatar> = new Map();
  private remoteAvatarConfigs: Map<string, { avatarType: string; avatarId?: string; avatarData?: string }> = new Map();
  private currentUserPubkey: string | null = null;

  // Position update tracking for player avatar (removed periodic updates)

  // Teleport animation settings
  private teleportAnimationType: TeleportAnimationType = 'fade';

  // Current movement style for position updates
  private currentMoveStyle: string = 'walk';

  // Snap to grid in walking mode (disabled by default for precise movement)
  private snapToGridInWalkMode: boolean = false;

  // Sun system and post-processing
  private sunSystem: SunSystem | null = null;
  private postProcessing: PostProcessing | null = null;

  // World grid helpers
  private worldGridHelpers: THREE.Object3D[] = [];

  // WASD movement state for avatar (when camera movement is not active)
  private avatarMoveForward = false;
  private avatarMoveBackward = false;
  private avatarMoveLeft = false;
  private avatarMoveRight = false;
  private avatarMoveSpeed = 5.0; // units per second

  // Event listener references for cleanup
  private boundKeyDown?: (event: KeyboardEvent) => void;
  private boundKeyUp?: (event: KeyboardEvent) => void;
  private boundPointerLockChange?: () => void;

  constructor() {
    this.scene = new THREE.Scene();
    this.camera = new THREE.PerspectiveCamera(
      75,
      window.innerWidth / window.innerHeight,
      0.1,
      1000
    );
    this.renderer = new THREE.WebGLRenderer();
    this.raycaster = new THREE.Raycaster();
    this.mouse = new THREE.Vector2();
  }

  initialize(canvas: HTMLCanvasElement): void {
    this.renderer = new THREE.WebGLRenderer({
      canvas,
      antialias: true,
      alpha: true
    });
    this.renderer.setSize(window.innerWidth, window.innerHeight);
    this.renderer.setPixelRatio(window.devicePixelRatio);
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.0;

    // Set fixed camera position for isometric-like view (centered at origin)
    // For 128-unit world (depth 7), position camera to see the whole world
    this.camera.position.set(100, 80, 100);
    this.camera.lookAt(0, 0, 0); // Look at origin (center of ground plane)

    // Dynamic sky color will be managed by sun system
    this.scene.background = new THREE.Color(0x87ceeb); // Sky blue
    this.scene.fog = new THREE.Fog(0x87ceeb, 100, 300);

    // Initialize sun system (replaces old static lights)
    this.sunSystem = new SunSystem(this.scene);
    this.sunSystem.setTimeOfDay(0.35); // Start slightly after sunrise
    this.sunSystem.setSunSpeed(0.01); // Slower movement for nice visuals
    this.sunSystem.setAutoMove(false); // Start with sun in fixed position

    // Initialize post-processing for bloom and sparkle effects
    this.postProcessing = new PostProcessing(this.renderer, this.scene, this.camera);

    this.setupMouseListener(canvas);
    this.setupMouseMoveListener(canvas);
    this.setupKeyboardListener(canvas);
    this.setupEditModeHelpers();
    this.setupOriginHelpers();
    this.setupCrosshair();
    this.setupCheckerPlane();

    // Initialize camera controller for both walk and edit modes
    this.cameraController = new CameraController(this.camera, canvas);
    this.cameraController.setUsePointerLock(false); // Don't use pointer lock by default
    this.cameraController.enable();

    this.lastTime = performance.now();
  }

  private setupCheckerPlane(): void {
    // Create checker plane (2^macroDepth × 2^macroDepth centered at origin)
    const checkerSize = 1 << getMacroDepth();
    this.checkerPlane = new CheckerPlane(checkerSize, checkerSize, 0.02);
    const checkerMesh = this.checkerPlane.getMesh();
    this.scene.add(checkerMesh);
    this.worldGridHelpers.push(checkerMesh);
  }

  private setupMouseListener(canvas: HTMLCanvasElement): void {
    // Left click handler
    canvas.addEventListener('click', (event) => {
      // Handle edit mode - place voxel
      if (this.isEditMode) {
        this.handleEditModeClick(event, canvas, true);
        return;
      }

      // Need avatar to handle clicks in walk mode
      if (!this.currentAvatar) return;

      // Calculate mouse position in normalized device coordinates (-1 to +1)
      // In mouse rotate mode (mode 2), use center crosshair position instead of mouse pointer
      if (this.mouseMode === 2) {
        // Center of screen
        this.mouse.x = 0;
        this.mouse.y = 0;
      } else {
        // Use actual mouse click position
        const rect = canvas.getBoundingClientRect();
        this.mouse.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
        this.mouse.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
      }

      // Update raycaster
      this.raycaster.setFromCamera(this.mouse, this.camera);

      // Raycast to ground plane at y=0 (same as edit mode)
      const intersectPoint = new THREE.Vector3();
      const didIntersect = this.raycaster.ray.intersectPlane(this.groundPlane, intersectPoint);

      if (didIntersect) {
        // Use exact raycast coordinates or snap to grid based on flag
        let targetX: number;
        let targetZ: number;

        if (this.snapToGridInWalkMode) {
          // Snap to grid centered on the intersection point (same as edit mode)
          const size = 1;
          targetX = snapToGrid(intersectPoint.x, size);
          targetZ = snapToGrid(intersectPoint.z, size);
        } else {
          // Use exact raycast coordinates for precise movement
          targetX = intersectPoint.x;
          targetZ = intersectPoint.z;
        }

        // Check if within valid world bounds (size=0 for point position, not voxel)
        if (isWithinWorldBounds(targetX, targetZ, 0)) {
          // Check modifiers: CTRL for teleport, SHIFT for run
          const useTeleport = event.ctrlKey;
          const useRun = event.shiftKey && !useTeleport;

          // Move avatar
          if (useTeleport) {
            this.currentAvatar.teleportTo(targetX, targetZ, this.teleportAnimationType);
            this.currentMoveStyle = `teleport:${this.teleportAnimationType}`;
            // Publish TARGET position with move style
            this.publishPlayerPositionAt(targetX, targetZ, this.currentMoveStyle);
          } else if (useRun) {
            this.currentAvatar.setRunSpeed(true);
            this.currentAvatar.setTargetPosition(targetX, targetZ);
            this.currentMoveStyle = 'run';
            // Publish TARGET position with move style
            this.publishPlayerPositionAt(targetX, targetZ, this.currentMoveStyle);
          } else {
            this.currentAvatar.setRunSpeed(false);
            this.currentAvatar.setTargetPosition(targetX, targetZ);
            this.currentMoveStyle = 'walk';
            // Publish TARGET position with move style
            this.publishPlayerPositionAt(targetX, targetZ, this.currentMoveStyle);
          }
        }
      }
    });

    // Mouse down handler - track continuous paint
    canvas.addEventListener('mousedown', (event) => {
      if (this.isEditMode) {
        if (event.button === 0) {
          // Left mouse button
          this.isLeftMousePressed = true;
          this.lastPaintedVoxel = null;
        } else if (event.button === 2) {
          // Right mouse button
          this.isRightMousePressed = true;
          this.lastPaintedVoxel = null;
        }
      }
    });

    // Mouse up handler - end continuous paint
    canvas.addEventListener('mouseup', (event) => {
      if (event.button === 0) {
        this.isLeftMousePressed = false;
        this.lastPaintedVoxel = null;
      } else if (event.button === 2) {
        this.isRightMousePressed = false;
        this.lastPaintedVoxel = null;
      }
    });

    // Mouse leave handler - end continuous paint when mouse leaves canvas
    canvas.addEventListener('mouseleave', () => {
      this.isLeftMousePressed = false;
      this.isRightMousePressed = false;
      this.lastPaintedVoxel = null;
    });

    // Right click handler - prevent context menu in edit mode (used for camera look)
    canvas.addEventListener('contextmenu', (event) => {
      if (this.isEditMode) {
        event.preventDefault();
      }
    });
  }

  private handleEditModeClick(event: MouseEvent, canvas: HTMLCanvasElement, isLeftClick: boolean): void {
    console.log('[Edit Click]', { isLeftClick, mouseMode: this.mouseMode, cursorDepth: this.cursorDepth });

    // Calculate mouse position
    // In mode 2 (shift rotate), raycast from center of screen
    // In mode 1 (free pointer), raycast from mouse position
    if (this.mouseMode === 2) {
      // Center of screen
      this.mouse.x = 0;
      this.mouse.y = 0;
    } else {
      // Mouse position
      const rect = canvas.getBoundingClientRect();
      this.mouse.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
      this.mouse.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
    }

    // Update raycaster
    this.raycaster.setFromCamera(this.mouse, this.camera);

    // Raycast to ground plane at y=0
    const intersectPoint = new THREE.Vector3();
    const didIntersect = this.raycaster.ray.intersectPlane(this.groundPlane, intersectPoint);

    console.log('[Raycast]', { didIntersect, intersectPoint: didIntersect ? intersectPoint : null });

    if (didIntersect) {
      const size = this.getCursorSize();
      const halfSize = size / 2;

      // Snap to grid centered on the intersection point
      const voxelCenterX = snapToGrid(intersectPoint.x, size);
      const voxelCenterZ = snapToGrid(intersectPoint.z, size);

      // Calculate corner position (world space)
      const voxelX = voxelCenterX - halfSize;
      const voxelZ = voxelCenterZ - halfSize;
      const voxelY = this.depthSelectMode === 1 ? 0 : -size;

      console.log('[Voxel Pos]', { voxelX, voxelY, voxelZ, voxelCenterX, voxelCenterZ, size, depthSelectMode: this.depthSelectMode });

      // Check if within valid world cube range (centered at origin)
      if (isWithinWorldBounds(voxelX, voxelZ, size)) {
        console.log('[Voxel Action]', isLeftClick ? 'paint' : 'erase');
        if (isLeftClick) {
          // Left click: use current color/erase mode
          this.paintVoxelWithSize(voxelX, voxelY, voxelZ, size);
        } else {
          // Right click always removes voxel
          this.eraseVoxelWithSize(voxelX, voxelY, voxelZ, size);
        }
      } else {
        console.log('[Out of Bounds]', { voxelX, voxelZ, size });
      }
    }
  }

  private onVoxelEdit?: (coord: CubeCoord, colorIndex: number) => void;

  setOnVoxelEdit(callback: (coord: CubeCoord, colorIndex: number) => void): void {
    this.onVoxelEdit = callback;
  }

  setSelectedColorIndex(colorIndex: number): void {
    this.selectedColorIndex = colorIndex;
  }

  /**
   * Get the current cursor coordinate (null if cursor not visible)
   */
  getCurrentCursorCoord(): CubeCoord | null {
    return this.currentCursorCoord;
  }

  /**
   * Get the current cursor depth
   */
  getCursorDepth(): number {
    return this.cursorDepth;
  }

  /**
   * Paint a voxel at the given world space corner position
   * @param x World space X coordinate (corner of voxel, not center)
   * @param y World space Y coordinate (corner of voxel, not center)
   * @param z World space Z coordinate (corner of voxel, not center)
   * @param size Size of voxel in world units
   */
  private paintVoxelWithSize(x: number, y: number, z: number, size: number): void {
    console.log('[Paint Voxel]', { x, y, z, size, selectedColor: this.selectedColorIndex });

    // Check if clear/eraser mode is selected (index -1)
    const isClearMode = this.selectedColorIndex === -1;

    if (isClearMode) {
      this.eraseVoxelWithSize(x, y, z, size);
      return;
    }

    // Place voxel with selected color (palette 0-31 maps to voxel values 32-63)
    const colorValue = this.selectedColorIndex + 32;

    // Convert world coordinates (corner) to cube coordinates (octree space)
    const coord = worldToCube(x, y, z, this.cursorDepth);

    console.log('[Paint -> CubeCoord]', { coord, colorValue, hasCallback: !!this.onVoxelEdit });

    // Call onVoxelEdit with CubeCoord
    this.onVoxelEdit?.(coord, colorValue);
  }

  private eraseVoxelWithSize(x: number, y: number, z: number, size: number): void {
    console.log('[Erase Voxel]', { x, y, z, size });

    // Convert world coordinates to cube coordinates
    const coord = worldToCube(x, y, z, this.cursorDepth);

    console.log('[Erase -> CubeCoord]', { coord, hasCallback: !!this.onVoxelEdit });

    // Call onVoxelEdit with CubeCoord
    this.onVoxelEdit?.(coord, 0);
  }

  private setupKeyboardListener(canvas: HTMLCanvasElement): void {
    this.boundKeyDown = (event: KeyboardEvent) => {
      // WASD movement for avatar (only when camera movement is not active)
      if (this.mouseMode === 1 && !this.isEditMode) {
        switch (event.key.toLowerCase()) {
          case 'w':
            this.avatarMoveForward = true;
            return;
          case 's':
            this.avatarMoveBackward = true;
            return;
          case 'a':
            this.avatarMoveLeft = true;
            return;
          case 'd':
            this.avatarMoveRight = true;
            return;
        }
      }

      // Toggle mouse mode with Shift key (works in both walk and edit modes)
      // Only toggle once per key press, not on repeated keydown events
      if (event.key === 'Shift') {
        if (this.shiftKeyPressed) {
          // Already handled this key press, ignore repeated keydown events
          return;
        }
        this.shiftKeyPressed = true;

        if (this.mouseMode === 1) {
          // Enter camera rotation mode
          this.mouseMode = 2;
          canvas.requestPointerLock();
          if (this.crosshair) {
            this.crosshair.style.display = 'block';
          }
          // Reset avatar movement state when entering camera mode
          this.resetAvatarMovementState();
          console.log('[Mouse Mode] Switched to mode 2 (first-person camera rotation)');
        } else if (this.mouseMode === 2) {
          // Exit camera rotation mode
          this.mouseMode = 1;
          document.exitPointerLock();
          if (this.crosshair) {
            this.crosshair.style.display = 'none';
          }
          // Reset paint state (only relevant in edit mode, but safe to always reset)
          this.isLeftMousePressed = false;
          this.isRightMousePressed = false;
          this.lastPaintedVoxel = null;
          console.log('[Mouse Mode] Switched to mode 1 (paint/erase)');
        }
        return;
      }

      // Toggle edit mode with 'e' key (works in both walk and edit modes)
      if (event.key === 'e' || event.key === 'E') {
        this.setEditMode(!this.isEditMode);
        // Reset avatar movement state when toggling edit mode
        this.resetAvatarMovementState();
        console.log(`[Edit Mode] Toggled to ${this.isEditMode ? 'ON' : 'OFF'}`);
        return;
      }

      // Edit mode specific controls
      if (!this.isEditMode) return;

      // Toggle depth select mode with Spacebar
      if (event.code === 'Space') {
        event.preventDefault();
        this.depthSelectMode = this.depthSelectMode === 1 ? 2 : 1;
        console.log(`[Depth Select] Switched to mode ${this.depthSelectMode} (y=${this.depthSelectMode === 1 ? 0 : -1})`);
      }

      // Cursor depth control with Arrow Up/Down
      if (event.code === 'ArrowUp') {
        event.preventDefault();
        this.cursorDepth = Math.min(getTotalDepth(), this.cursorDepth + 1);
        this.updateCursorSize();
        console.log(`[Cursor Depth] Increased to ${this.cursorDepth} (size=${this.getCursorSize()})`);
      }

      if (event.code === 'ArrowDown') {
        event.preventDefault();
        this.cursorDepth = Math.max(0, this.cursorDepth - 1);
        this.updateCursorSize();
        console.log(`[Cursor Depth] Decreased to ${this.cursorDepth} (size=${this.getCursorSize()})`);
      }
    };

    this.boundKeyUp = (event: KeyboardEvent) => {
      // Reset shift key flag when released
      if (event.key === 'Shift') {
        this.shiftKeyPressed = false;
      }

      // Reset WASD movement state
      switch (event.key.toLowerCase()) {
        case 'w':
          this.avatarMoveForward = false;
          break;
        case 's':
          this.avatarMoveBackward = false;
          break;
        case 'a':
          this.avatarMoveLeft = false;
          break;
        case 'd':
          this.avatarMoveRight = false;
          break;
      }
    };

    this.boundPointerLockChange = () => {
      if (!document.pointerLockElement && this.mouseMode === 2) {
        // Pointer lock was exited externally (e.g., Escape key), sync mode back to 1
        this.mouseMode = 1;
        if (this.crosshair) {
          this.crosshair.style.display = 'none';
        }
        this.isLeftMousePressed = false;
        this.isRightMousePressed = false;
        this.lastPaintedVoxel = null;
        console.log('[Mouse Mode] Pointer lock exited (Escape), switched to mode 1');
      }
    };

    // Register event listeners
    window.addEventListener('keydown', this.boundKeyDown);
    window.addEventListener('keyup', this.boundKeyUp);
    document.addEventListener('pointerlockchange', this.boundPointerLockChange);
  }

  private resetAvatarMovementState(): void {
    this.avatarMoveForward = false;
    this.avatarMoveBackward = false;
    this.avatarMoveLeft = false;
    this.avatarMoveRight = false;
  }

  // Removed: setupLights() - now using SunSystem for dynamic lighting

  private setupEditModeHelpers(): void {
    // Create preview cube (1x1x1 cube wireframe)
    const cubeGeometry = new THREE.BoxGeometry(1, 1, 1);
    const edges = new THREE.EdgesGeometry(cubeGeometry);
    const lineMaterial = new THREE.LineBasicMaterial({
      color: 0x00ff00,
      linewidth: 2,
      opacity: 0.8,
      transparent: true,
      depthTest: false, // Always render on top
      depthWrite: false
    });
    this.previewCube = new THREE.LineSegments(edges, lineMaterial);
    this.previewCube.visible = false;
    this.previewCube.renderOrder = 999; // Render last
    this.scene.add(this.previewCube);
  }

  private setupOriginHelpers(): void {
    // Create axis helper at origin
    // Axis extends 50% beyond unit cube (1.5 units)
    const axisHelper = new THREE.AxesHelper(1.5);
    axisHelper.position.set(0, 0, 0);
    // Make axis always visible (no depth testing)
    if (Array.isArray(axisHelper.material)) {
      axisHelper.material.forEach(mat => {
        mat.depthTest = false;
        mat.depthWrite = false;
      });
    } else {
      axisHelper.material.depthTest = false;
      axisHelper.material.depthWrite = false;
    }
    axisHelper.renderOrder = 999;
    this.scene.add(axisHelper);
    this.worldGridHelpers.push(axisHelper);

    // Create transparent wireframe for unit cube at origin
    const unitCubeGeometry = new THREE.BoxGeometry(1, 1, 1);
    const unitCubeEdges = new THREE.EdgesGeometry(unitCubeGeometry);
    const unitCubeLineMaterial = new THREE.LineBasicMaterial({
      color: 0xffffff,
      opacity: 0.2,
      transparent: true
    });
    const unitCubeWireframe = new THREE.LineSegments(unitCubeEdges, unitCubeLineMaterial);
    // Position cube so its corner is at origin (center at 0.5, 0.5, 0.5)
    unitCubeWireframe.position.set(0.5, 0.5, 0.5);
    this.scene.add(unitCubeWireframe);
    this.worldGridHelpers.push(unitCubeWireframe);

    // Create world bounds wireframe box
    // World is worldSize×worldSize×worldSize centered at origin
    const worldSize = getWorldSize(getTotalDepth(), getMicroDepth());
    const worldBoxGeometry = new THREE.BoxGeometry(worldSize, worldSize, worldSize);
    const worldBoxEdges = new THREE.EdgesGeometry(worldBoxGeometry);
    const worldBoxLineMaterial = new THREE.LineBasicMaterial({
      color: 0xffffff,
      opacity: 0.5,
      transparent: true
    });
    const worldBoxWireframe = new THREE.LineSegments(worldBoxEdges, worldBoxLineMaterial);
    worldBoxWireframe.position.set(0, 0, 0); // Centered at origin
    this.scene.add(worldBoxWireframe);
    this.worldGridHelpers.push(worldBoxWireframe);
  }

  private setupCrosshair(): void {
    // Create crosshair element for first-person mode
    this.crosshair = document.createElement('div');
    this.crosshair.style.position = 'fixed';
    this.crosshair.style.top = '50%';
    this.crosshair.style.left = '50%';
    this.crosshair.style.transform = 'translate(-50%, -50%)';
    this.crosshair.style.width = '20px';
    this.crosshair.style.height = '20px';
    this.crosshair.style.pointerEvents = 'none';
    this.crosshair.style.zIndex = '1000';
    this.crosshair.style.display = 'none';

    // Create crosshair lines
    this.crosshair.innerHTML = `
      <div style="position: absolute; top: 50%; left: 0; width: 100%; height: 2px; background: rgba(255, 255, 255, 0.8); transform: translateY(-50%);"></div>
      <div style="position: absolute; left: 50%; top: 0; height: 100%; width: 2px; background: rgba(255, 255, 255, 0.8); transform: translateX(-50%);"></div>
      <div style="position: absolute; top: 50%; left: 50%; width: 4px; height: 4px; background: rgba(255, 255, 255, 0.9); border-radius: 50%; transform: translate(-50%, -50%);"></div>
    `;

    document.body.appendChild(this.crosshair);
  }

  private getCursorSize(): number {
    return getVoxelSizeFromCubeCoord(this.cursorDepth);
  }

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

      // Recalculate corner position with new size
      const voxelX = voxelCenterX - halfSize;
      const voxelZ = voxelCenterZ - halfSize;
      const voxelY = this.depthSelectMode === 1 ? 0 : -size;

      // Update cursor coordinate with new depth
      this.currentCursorCoord = worldToCube(voxelX, voxelY, voxelZ, this.cursorDepth);

      // Update preview cube position
      this.currentGridPosition.set(voxelCenterX, voxelY + halfSize, voxelCenterZ);
      this.previewCube.position.copy(this.currentGridPosition);
    }
  }

  private updateVoxelCursorAtCenter(): void {
    if (!this.previewCube) return;

    // Raycast from center of screen
    this.mouse.x = 0;
    this.mouse.y = 0;
    this.raycaster.setFromCamera(this.mouse, this.camera);

    // Raycast to ground plane at y=0
    const intersectPoint = new THREE.Vector3();
    const didIntersect = this.raycaster.ray.intersectPlane(this.groundPlane, intersectPoint);

    if (didIntersect) {
      const size = this.getCursorSize();
      const halfSize = size / 2;

      // Snap to grid centered on the intersection point
      const voxelCenterX = snapToGrid(intersectPoint.x, size);
      const voxelCenterZ = snapToGrid(intersectPoint.z, size);

      // Calculate corner position (world space)
      const voxelX = voxelCenterX - halfSize;
      const voxelZ = voxelCenterZ - halfSize;
      const voxelY = this.depthSelectMode === 1 ? 0 : -size;

      // Check if within ground bounds (centered at origin)
      if (isWithinWorldBounds(voxelX, voxelZ, size)) {
        // Store current cursor coordinate (using corner position)
        this.currentCursorCoord = worldToCube(voxelX, voxelY, voxelZ, this.cursorDepth);

        // Position preview cube at center of voxel (world space)
        this.currentGridPosition.set(voxelCenterX, voxelY + halfSize, voxelCenterZ);
        this.previewCube.position.copy(this.currentGridPosition);
        this.previewCube.visible = true;

        // Continuous paint in shift mode: if mouse button is pressed, paint/erase voxel at new position
        if (this.isLeftMousePressed || this.isRightMousePressed) {
          // Check if this is a new voxel position (different from last painted)
          const isNewPosition = !this.lastPaintedVoxel ||
            this.lastPaintedVoxel.x !== voxelX ||
            this.lastPaintedVoxel.y !== voxelY ||
            this.lastPaintedVoxel.z !== voxelZ;

          if (isNewPosition) {
            if (this.isLeftMousePressed) {
              // Left mouse: draw with selected color
              this.paintVoxelWithSize(voxelX, voxelY, voxelZ, size);
            } else if (this.isRightMousePressed) {
              // Right mouse: erase
              this.eraseVoxelWithSize(voxelX, voxelY, voxelZ, size);
            }
            this.lastPaintedVoxel = { x: voxelX, y: voxelY, z: voxelZ };
          }
        }
      } else {
        // Outside bounds - hide cursor
        this.previewCube.visible = false;
        this.currentCursorCoord = null;
      }
    } else {
      this.previewCube.visible = false;
      this.currentCursorCoord = null;
    }
  }

  private setupMouseMoveListener(canvas: HTMLCanvasElement): void {
    canvas.addEventListener('mousemove', (event) => {
      // Mode 2: First-person camera rotation with grabbed pointer (works in both modes)
      if (this.mouseMode === 2) {
        const sensitivity = 0.002;
        const deltaX = event.movementX * sensitivity;
        const deltaY = event.movementY * sensitivity;

        // Get current camera euler angles
        const euler = new THREE.Euler().setFromQuaternion(this.camera.quaternion, 'YXZ');

        // Update yaw (left/right rotation around Y axis)
        // Moving mouse right rotates camera right
        euler.y -= deltaX;

        // Update pitch (up/down rotation around X axis)
        // Moving mouse down looks down
        euler.x -= deltaY;

        // Clamp pitch to prevent camera flipping
        const maxPitch = Math.PI / 2 - 0.1;
        euler.x = Math.max(-maxPitch, Math.min(maxPitch, euler.x));

        // Apply rotation back to camera
        this.camera.quaternion.setFromEuler(euler);

        // Don't return - continue to update voxel cursor below (if in edit mode)
      }

      // Edit mode only: update voxel cursor
      if (!this.isEditMode || !this.previewCube) return;

      // Calculate mouse position in normalized device coordinates
      // In mode 2 (shift rotate), raycast from center of screen
      // In mode 1 (free pointer), raycast from mouse position
      const rect = canvas.getBoundingClientRect();
      if (this.mouseMode === 2) {
        // Center of screen
        this.mouse.x = 0;
        this.mouse.y = 0;
      } else {
        // Mouse position
        this.mouse.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
        this.mouse.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
      }

      // Update raycaster
      this.raycaster.setFromCamera(this.mouse, this.camera);

      // Raycast to ground plane at y=0
      const intersectPoint = new THREE.Vector3();
      const didIntersect = this.raycaster.ray.intersectPlane(this.groundPlane, intersectPoint);

      if (didIntersect) {
        const size = this.getCursorSize();
        const halfSize = size / 2;

        // Snap to grid centered on the intersection point
        const voxelCenterX = snapToGrid(intersectPoint.x, size);
        const voxelCenterZ = snapToGrid(intersectPoint.z, size);

        // Calculate corner position (world space)
        const voxelX = voxelCenterX - halfSize;
        const voxelZ = voxelCenterZ - halfSize;
        const voxelY = this.depthSelectMode === 1 ? 0 : -size;

        // Check if within ground bounds (centered at origin)
        if (isWithinWorldBounds(voxelX, voxelZ, size)) {
          // Store current cursor coordinate (using corner position)
          this.currentCursorCoord = worldToCube(voxelX, voxelY, voxelZ, this.cursorDepth);

          // Position preview cube at center of voxel (world space)
          this.currentGridPosition.set(voxelCenterX, voxelY + halfSize, voxelCenterZ);
          this.previewCube.position.copy(this.currentGridPosition);
          this.previewCube.visible = true;

          // Continuous paint: if mouse button is pressed, paint/erase voxel at new position
          if (this.isLeftMousePressed || this.isRightMousePressed) {
            // Check if this is a new voxel position (different from last painted)
            const isNewPosition = !this.lastPaintedVoxel ||
              this.lastPaintedVoxel.x !== voxelX ||
              this.lastPaintedVoxel.y !== voxelY ||
              this.lastPaintedVoxel.z !== voxelZ;

            if (isNewPosition) {
              if (this.isLeftMousePressed) {
                // Left mouse: draw with selected color
                this.paintVoxelWithSize(voxelX, voxelY, voxelZ, size);
              } else if (this.isRightMousePressed) {
                // Right mouse: erase
                this.eraseVoxelWithSize(voxelX, voxelY, voxelZ, size);
              }
              this.lastPaintedVoxel = { x: voxelX, y: voxelY, z: voxelZ };
            }
          }
        } else {
          // Outside bounds - hide cursor
          this.previewCube.visible = false;
          this.currentCursorCoord = null;
        }
      } else {
        this.previewCube.visible = false;
        this.currentCursorCoord = null;
      }
    });
  }

  updateGeometry(vertices: Float32Array, indices: Uint32Array, normals: Float32Array, colors?: Float32Array): void {
    if (this.geometryMesh) {
      this.scene.remove(this.geometryMesh);
      this.geometryMesh.geometry.dispose();
      if (this.geometryMesh.material instanceof THREE.Material) {
        this.geometryMesh.material.dispose();
      }
    }

    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(vertices, 3));
    geometry.setAttribute('normal', new THREE.BufferAttribute(normals, 3));

    // Add vertex colors if provided
    if (colors && colors.length > 0) {
      geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));
    }

    geometry.setIndex(new THREE.BufferAttribute(indices, 1));

    const material = new THREE.MeshPhongMaterial({
      vertexColors: colors && colors.length > 0,
      color: colors && colors.length > 0 ? 0xffffff : 0x44aa44,
      specular: 0x333333,
      shininess: 15,
      wireframe: false,
      side: THREE.FrontSide,
      flatShading: false
    });

    this.geometryMesh = new THREE.Mesh(geometry, material);
    this.geometryMesh.castShadow = true;
    this.geometryMesh.receiveShadow = true;
    this.geometryMesh.renderOrder = 0; // Render world cube first
    this.scene.add(this.geometryMesh);
  }

  render(): void {
    const currentTime = performance.now();
    const deltaTime_s = (currentTime - this.lastTime) / 1000;
    this.lastTime = currentTime;

    // Update camera controller (always active for keyboard movement)
    if (this.cameraController) {
      this.cameraController.update(deltaTime_s);
    }

    // Update sun system for moving sun
    if (this.sunSystem) {
      this.sunSystem.update(deltaTime_s);
    }

    // Update voxel cursor in shift rotate mode (center screen raycast)
    if (this.isEditMode && this.mouseMode === 2 && this.previewCube) {
      this.updateVoxelCursorAtCenter();
    }

    // Update current avatar
    if (this.currentAvatar) {
      const wasTeleporting = this.currentAvatar.isTeleporting();
      this.currentAvatar.update(deltaTime_s);
      const isTeleporting = this.currentAvatar.isTeleporting();

      // Don't publish position during teleport animation
      if (!isTeleporting) {
        // Check if just finished teleporting
        if (wasTeleporting && !isTeleporting) {
          // Teleport completed - publish final position
          this.publishPlayerPosition();
        }
        // Note: Only publish on movement start, not when reaching target
        // Position is already published immediately when clicking (in setupMouseListener)
      }
    }

    // WASD keyboard movement for avatar (only when camera movement is not active)
    if (this.currentAvatar && this.mouseMode === 1 && !this.isEditMode) {
      const isMoving = this.avatarMoveForward || this.avatarMoveBackward || this.avatarMoveLeft || this.avatarMoveRight;

      if (isMoving) {
        // Get camera's forward direction (projected on XZ plane)
        const cameraDirection = new THREE.Vector3();
        this.camera.getWorldDirection(cameraDirection);
        const forward = new THREE.Vector3(cameraDirection.x, 0, cameraDirection.z).normalize();

        // Get right direction (perpendicular to forward)
        const right = new THREE.Vector3(-forward.z, 0, forward.x).normalize();

        // Calculate movement direction based on WASD input
        const moveDirection = new THREE.Vector3(0, 0, 0);

        if (this.avatarMoveForward) {
          moveDirection.add(forward);
        }
        if (this.avatarMoveBackward) {
          moveDirection.sub(forward);
        }
        if (this.avatarMoveRight) {
          moveDirection.add(right);
        }
        if (this.avatarMoveLeft) {
          moveDirection.sub(right);
        }

        // Normalize and scale by speed
        if (moveDirection.length() > 0) {
          moveDirection.normalize();
          const distance = this.avatarMoveSpeed * deltaTime_s;

          // Get current avatar position
          const currentPos = this.currentAvatar.getPosition();

          // Calculate new position
          const newX = currentPos.x + moveDirection.x * distance;
          const newZ = currentPos.z + moveDirection.z * distance;

          // Check if within world bounds
          if (isWithinWorldBounds(newX, newZ, 0)) {
            // Move avatar to new position
            this.currentAvatar.setRunSpeed(false);
            this.currentAvatar.setTargetPosition(newX, newZ);
            this.currentMoveStyle = 'walk';

            // Publish position update
            this.publishPlayerPositionAt(newX, newZ, this.currentMoveStyle);
          }
        }
      }
    }

    // Update all remote avatars
    for (const avatar of this.remoteAvatars.values()) {
      avatar.update(deltaTime_s);
    }

    // Render with post-processing (includes bloom for sparkle effects)
    if (this.postProcessing) {
      this.postProcessing.render(deltaTime_s);
    } else {
      this.renderer.render(this.scene, this.camera);
    }
  }

  handleResize(): void {
    const width = window.innerWidth;
    const height = window.innerHeight;

    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(width, height);

    // Update post-processing size
    if (this.postProcessing) {
      this.postProcessing.setSize(width, height);
    }
  }

  getCamera(): THREE.PerspectiveCamera {
    return this.camera;
  }

  getScene(): THREE.Scene {
    return this.scene;
  }

  createAvatar(modelUrl?: string, scale?: number, transform?: Transform): void {
    // Remove existing avatar
    if (this.currentAvatar) {
      this.scene.remove(this.currentAvatar.getObject3D());
      this.currentAvatar.dispose();
    }

    // Create new GLB avatar
    this.currentAvatar = new Avatar(transform, { modelUrl, scale }, this.scene);
    this.scene.add(this.currentAvatar.getObject3D());

    // Fetch and apply profile picture for current user
    if (this.currentUserPubkey) {
      this.fetchAndApplyProfilePicture(this.currentUserPubkey, this.currentAvatar);
    }
  }

  removeAvatar(): void {
    if (this.currentAvatar) {
      this.scene.remove(this.currentAvatar.getObject3D());
      this.currentAvatar.dispose();
      this.currentAvatar = null;

      // Reset camera to default position (centered at origin)
      this.camera.position.set(8, 6, 8);
      this.camera.lookAt(0, 0, 0);
    }
  }

  hasAvatar(): boolean {
    return this.currentAvatar !== null;
  }

  getCurrentTransform(): Transform | undefined {
    return this.currentAvatar?.getTransform();
  }

  /**
   * Initialize the avatar engine for voxel avatars
   */
  setAvatarEngine(engine: AvatarEngine): void {
    this.avatarEngine = engine;
  }

  /**
   * Set callback for position updates
   */
  setPositionUpdateCallback(callback: (x: number, y: number, z: number, quaternion: [number, number, number, number], moveStyle?: string) => void): void {
    this.onPositionUpdate = callback;
  }

  /**
   * Create a voxel avatar for a user
   */
  createVoxelAvatar(userNpub: string, scale: number = 1.0, transform?: Transform): void {
    if (!this.avatarEngine) {
      console.error('Avatar engine not initialized');
      return;
    }

    // Remove existing avatar
    if (this.currentAvatar) {
      this.scene.remove(this.currentAvatar.getObject3D());
      this.currentAvatar.dispose();
    }

    // Create new voxel avatar
    const voxelAvatar = new VoxelAvatar({
      userNpub: userNpub || '',
      scale,
    }, transform, this.scene);

    // Generate geometry from Rust
    const geometryData = this.avatarEngine.generate_avatar(userNpub);

    // Apply geometry to avatar
    voxelAvatar.applyGeometry(geometryData);

    // Add to scene
    this.scene.add(voxelAvatar.getObject3D());
    this.currentAvatar = voxelAvatar;

    // Fetch and apply profile picture for current user
    if (this.currentUserPubkey) {
      this.fetchAndApplyProfilePicture(this.currentUserPubkey, this.currentAvatar);
    }

    console.log(`Created voxel avatar for ${userNpub}`);
  }

  /**
   * Create a voxel avatar from a .vox file
   */
  async createVoxelAvatarFromVoxFile(voxUrl: string, userNpub: string | undefined = undefined, scale: number = 1.0, transform?: Transform): Promise<void> {
    // Import the loadVoxFromUrl function
    const { loadVoxFromUrl } = await import('../utils/voxLoader');

    try {
      // Load .vox file and get geometry (pass undefined for original colors)
      const geometryData = await loadVoxFromUrl(voxUrl, userNpub ?? undefined);

      // Remove existing avatar
      if (this.currentAvatar) {
        this.scene.remove(this.currentAvatar.getObject3D());
        this.currentAvatar.dispose();
      }

      // Create new voxel avatar
      const voxelAvatar = new VoxelAvatar({
        userNpub: userNpub ?? '',
        scale,
      }, transform, this.scene);

      // Apply geometry from .vox file
      voxelAvatar.applyGeometry(geometryData);

      // Add to scene
      this.scene.add(voxelAvatar.getObject3D());
      this.currentAvatar = voxelAvatar;

      // Fetch and apply profile picture for current user
      console.log(`[Scene] After avatar creation, currentUserPubkey: ${this.currentUserPubkey}`);
      if (this.currentUserPubkey) {
        this.fetchAndApplyProfilePicture(this.currentUserPubkey, this.currentAvatar);
      } else {
        console.log(`[Scene] No currentUserPubkey set, skipping profile fetch`);
      }

      console.log(`Created voxel avatar from .vox file: ${voxUrl}`);
    } catch (error) {
      console.error('Failed to load .vox avatar:', error);
      throw error;
    }
  }

  /**
   * Create a CSM avatar from parsed mesh data
   */
  createCsmAvatar(meshData: { vertices: number[]; indices: number[]; normals: number[]; colors: number[] }, userNpub: string | undefined = undefined, scale: number = 1.0, transform?: Transform): void {
    // Remove existing avatar
    if (this.currentAvatar) {
      this.scene.remove(this.currentAvatar.getObject3D());
      this.currentAvatar.dispose();
    }

    // Create new voxel avatar (CSM avatars use VoxelAvatar class)
    const voxelAvatar = new VoxelAvatar({
      userNpub: userNpub ?? '',
      scale,
    }, transform, this.scene);

    // Convert mesh data to the format VoxelAvatar expects
    const geometryData = {
      vertices: new Float32Array(meshData.vertices),
      indices: new Uint32Array(meshData.indices),
      normals: new Float32Array(meshData.normals),
      colors: new Float32Array(meshData.colors),
    };

    // Apply geometry from CSM mesh
    voxelAvatar.applyGeometry(geometryData);

    // Add to scene
    this.scene.add(voxelAvatar.getObject3D());
    this.currentAvatar = voxelAvatar;

    // Fetch and apply profile picture for current user
    if (this.currentUserPubkey) {
      this.fetchAndApplyProfilePicture(this.currentUserPubkey, this.currentAvatar);
    }

    console.log('Created CSM avatar');
  }

  /**
   * Set edit mode to show/hide grid helpers
   */
  setEditMode(isEditMode: boolean): void {
    this.isEditMode = isEditMode;

    if (this.previewCube && !isEditMode) {
      this.previewCube.visible = false;
    }

    // Reset mouse mode and depth select when exiting edit mode
    if (!isEditMode) {
      if (this.mouseMode === 2) {
        this.mouseMode = 1;
        document.exitPointerLock();
        // Hide crosshair
        if (this.crosshair) {
          this.crosshair.style.display = 'none';
        }
      }
      this.isLeftMousePressed = false;
      this.isRightMousePressed = false;
      this.lastPaintedVoxel = null;
      this.depthSelectMode = 1;
      this.currentCursorCoord = null; // Clear cursor coordinate
    }

    // Camera controller stays enabled in both walk and edit modes
    // No need to toggle it based on edit mode
  }

  /**
   * Set camera mode to enable/disable free camera movement
   * Note: Camera controller is now always enabled, this is kept for compatibility
   */
  setCameraMode(_isCameraMode: boolean): void {
    // Camera controller stays enabled all the time now
    // This method is kept for compatibility but does nothing
  }

  /**
   * Set callback for when camera mode exits (e.g., pointer lock lost)
   */
  setOnCameraModeExit(callback: () => void): void {
    if (this.cameraController) {
      this.cameraController.setOnExitCallback(callback);
    }
  }

  /**
   * Set the current user's pubkey (to exclude from remote avatars)
   */
  setCurrentUserPubkey(pubkey: string | null): void {
    console.log(`[Scene] setCurrentUserPubkey called: ${pubkey}`);
    this.currentUserPubkey = pubkey;

    // If we have an avatar already and now have a pubkey, fetch profile retroactively
    if (pubkey && this.currentAvatar) {
      console.log(`[Scene] Avatar exists, fetching profile retroactively`);
      this.fetchAndApplyProfilePicture(pubkey, this.currentAvatar);
    }
  }

  /**
   * Refresh profile picture for current avatar (call after avatar is loaded)
   */
  refreshCurrentAvatarProfile(): void {
    console.log(`[Scene] refreshCurrentAvatarProfile called, pubkey: ${this.currentUserPubkey}, hasAvatar: ${!!this.currentAvatar}`);
    if (this.currentUserPubkey && this.currentAvatar) {
      console.log(`[Scene] Refreshing profile for current avatar`);
      this.fetchAndApplyProfilePicture(this.currentUserPubkey, this.currentAvatar);
    }
  }

  /**
   * Publish current player position
   */
  private publishPlayerPosition(): void {
    if (!this.currentAvatar || !this.onPositionUpdate) return;

    const transform = this.currentAvatar.getTransform();
    const eventData = transform.toEventData();

    this.onPositionUpdate(
      eventData.x,
      eventData.y,
      eventData.z,
      eventData.quaternion
    );
  }

  /**
   * Publish specific position (used when setting target position)
   */
  private publishPlayerPositionAt(x: number, z: number, moveStyle: string): void {
    if (!this.currentAvatar || !this.onPositionUpdate) return;

    // Get current rotation from avatar
    const transform = this.currentAvatar.getTransform();

    // TODO: Pass moveStyle to onPositionUpdate
    this.onPositionUpdate(
      x,
      0, // y is always 0 on the ground
      z,
      transform.toEventData().quaternion,
      moveStyle
    );
  }

  /**
   * Update remote avatars based on avatar states from other users
   */
  updateRemoteAvatars(states: Map<string, AvatarState>): void {
    if (!this.avatarEngine) return;

    // Get list of pubkeys that should have avatars
    const activePubkeys = new Set<string>();
    states.forEach((_state, pubkey) => {
      // Skip current user
      if (pubkey === this.currentUserPubkey) return;
      // Note: 'away' status users are already removed from states map by avatar-state service
      activePubkeys.add(pubkey);
    });

    // Remove avatars for users that are no longer active
    for (const [pubkey, avatar] of this.remoteAvatars.entries()) {
      if (!activePubkeys.has(pubkey)) {
        this.scene.remove(avatar.getObject3D());
        avatar.dispose();
        this.remoteAvatars.delete(pubkey);
        this.remoteAvatarConfigs.delete(pubkey);
        console.log(`Removed remote avatar for ${pubkey}`);
      }
    }

    // Create or update avatars for active users
    states.forEach((state, pubkey) => {
      // Skip current user
      if (pubkey === this.currentUserPubkey) return;

      const existing = this.remoteAvatars.get(pubkey);
      const existingConfig = this.remoteAvatarConfigs.get(pubkey);

      // Check if avatar model changed
      const modelChanged = existingConfig && (
        existingConfig.avatarType !== state.avatarType ||
        existingConfig.avatarId !== state.avatarId ||
        existingConfig.avatarData !== state.avatarData
      );

      if (modelChanged) {
        console.log(`[Scene] Model changed for ${state.npub}:`, {
          old: existingConfig,
          new: { avatarType: state.avatarType, avatarId: state.avatarId, avatarDataLength: state.avatarData?.length }
        });
      }

      // Check if we need to create a new avatar
      if (!existing || modelChanged) {
        // Remove old avatar if model changed
        if (modelChanged && existing) {
          this.scene.remove(existing.getObject3D());
          existing.dispose();
          this.remoteAvatars.delete(pubkey);
          this.remoteAvatarConfigs.delete(pubkey);
          console.log(`Recreating remote avatar for ${state.npub} due to model change`);
        }
        this.createRemoteAvatar(pubkey, state);
      } else {
        // Update position for existing avatar
        this.updateRemoteAvatarPosition(pubkey, state);
      }
    });
  }

  /**
   * Create a remote avatar for another user
   */
  private createRemoteAvatar(pubkey: string, state: AvatarState): void {
    if (!this.avatarEngine) return;

    const { position, avatarType, avatarId, avatarUrl, avatarData, npub } = state;

    console.log(`[Scene] Creating remote avatar for ${npub}:`, { avatarType, avatarId, avatarUrl, avatarDataLength: avatarData?.length });

    // Create transform from position data
    const transform = Transform.fromEventData(position);

    if (avatarType === 'vox') {
      // Create voxel avatar
      const voxelAvatar = new VoxelAvatar({
        userNpub: npub,
        scale: 1.0,
      }, transform, this.scene);

      // Generate or load geometry (use undefined for npub to preserve original colors)
      if (avatarId && avatarId !== 'generated') {
        console.log(`[Scene] Loading VOX model from avatarId: ${avatarId}`);
        // Load from .vox file using model config
        import('../utils/modelConfig').then(({ getModelUrl }) => {
          const voxUrl = getModelUrl(avatarId, 'vox');

          if (!voxUrl) {
            console.warn(`No model found for avatarId: ${avatarId}, using generated`);
            const geometryData = this.avatarEngine!.generate_avatar(npub);
            voxelAvatar.applyGeometry(geometryData);
            return;
          }

          import('../utils/voxLoader').then(({ loadVoxFromUrl }) => {
            // Pass undefined to preserve original colors
            loadVoxFromUrl(voxUrl, undefined).then((geometryData) => {
              voxelAvatar.applyGeometry(geometryData);
            }).catch(error => {
              console.error('Failed to load .vox avatar for remote user:', error);
              // Fallback to generated
              const geometryData = this.avatarEngine!.generate_avatar(npub);
              voxelAvatar.applyGeometry(geometryData);
            });
          }).catch(console.error);
        }).catch(console.error);
      } else {
        // Use procedurally generated model
        console.log(`[Scene] No avatarId for ${npub}, using procedurally generated avatar`);
        const geometryData = this.avatarEngine.generate_avatar(npub);
        voxelAvatar.applyGeometry(geometryData);
      }

      // Add to scene
      this.scene.add(voxelAvatar.getObject3D());
      this.remoteAvatars.set(pubkey, voxelAvatar);
      this.remoteAvatarConfigs.set(pubkey, { avatarType, avatarId, avatarData });

      // Fetch and apply profile picture
      this.fetchAndApplyProfilePicture(pubkey, voxelAvatar, npub);

      console.log(`Created remote voxel avatar for ${npub}`);
    } else if (avatarType === 'csm') {
      // Create CSM avatar
      if (avatarData) {
        console.log(`Creating remote CSM avatar for ${npub}`);

        // Parse CSM code to mesh
        import('../utils/cubeWasm').then(({ parseCsmToMesh }) => {
          parseCsmToMesh(avatarData).then((result) => {
            if ('error' in result) {
              console.error(`Failed to parse CSM for remote avatar ${npub}:`, result.error);
              // Fallback to generated avatar
              const geometryData = this.avatarEngine!.generate_avatar(npub);
              const voxelAvatar = new VoxelAvatar({
                userNpub: npub,
                scale: 1.0,
              }, transform, this.scene);
              voxelAvatar.applyGeometry(geometryData);
              this.scene.add(voxelAvatar.getObject3D());
              this.remoteAvatars.set(pubkey, voxelAvatar);
              this.remoteAvatarConfigs.set(pubkey, { avatarType, avatarId, avatarData });

              // Fetch and apply profile picture
              this.fetchAndApplyProfilePicture(pubkey, voxelAvatar, npub);

              return;
            }

            // Create voxel avatar with CSM mesh
            const voxelAvatar = new VoxelAvatar({
              userNpub: npub,
              scale: 1.0,
            }, transform, this.scene);

            // Convert mesh data to typed arrays
            const geometryData = {
              vertices: new Float32Array(result.vertices),
              indices: new Uint32Array(result.indices),
              normals: new Float32Array(result.normals),
              colors: new Float32Array(result.colors),
            };

            voxelAvatar.applyGeometry(geometryData);
            this.scene.add(voxelAvatar.getObject3D());
            this.remoteAvatars.set(pubkey, voxelAvatar);
            this.remoteAvatarConfigs.set(pubkey, { avatarType, avatarId, avatarData });

            // Fetch and apply profile picture
            this.fetchAndApplyProfilePicture(pubkey, voxelAvatar, npub);

            console.log(`Created remote CSM avatar for ${npub}`);
          }).catch(error => {
            console.error(`Failed to load CSM avatar for remote user ${npub}:`, error);
            // Fallback to generated avatar
            const geometryData = this.avatarEngine!.generate_avatar(npub);
            const voxelAvatar = new VoxelAvatar({
              userNpub: npub,
              scale: 1.0,
            }, transform, this.scene);
            voxelAvatar.applyGeometry(geometryData);
            this.scene.add(voxelAvatar.getObject3D());
            this.remoteAvatars.set(pubkey, voxelAvatar);
            this.remoteAvatarConfigs.set(pubkey, { avatarType, avatarId, avatarData });
          });
        }).catch(console.error);
      } else {
        console.warn(`No avatarData provided for remote CSM avatar ${npub}, using generated`);
        // Fallback to generated avatar
        const geometryData = this.avatarEngine.generate_avatar(npub);
        const voxelAvatar = new VoxelAvatar({
          userNpub: npub,
          scale: 1.0,
        }, transform, this.scene);
        voxelAvatar.applyGeometry(geometryData);
        this.scene.add(voxelAvatar.getObject3D());
        this.remoteAvatars.set(pubkey, voxelAvatar);
        this.remoteAvatarConfigs.set(pubkey, { avatarType, avatarId, avatarData });
      }
    } else {
      // Create GLB avatar
      const glbAvatar = new Avatar(transform, {
        modelUrl: avatarUrl,
        scale: 1.0,
      }, this.scene);
      this.scene.add(glbAvatar.getObject3D());
      this.remoteAvatars.set(pubkey, glbAvatar);
      this.remoteAvatarConfigs.set(pubkey, { avatarType, avatarId, avatarData });

      // Fetch and apply profile picture
      this.fetchAndApplyProfilePicture(pubkey, glbAvatar, npub);

      console.log(`Created remote GLB avatar for ${npub}`);
    }
  }

  /**
   * Update remote avatar position
   */
  private updateRemoteAvatarPosition(pubkey: string, state: AvatarState): void {
    const avatar = this.remoteAvatars.get(pubkey);
    if (!avatar) return;

    const { position, moveStyle } = state;

    if (!moveStyle || moveStyle === 'walk') {
      // Walk: animate from current position at normal speed
      avatar.setRunSpeed(false);
      avatar.setTargetPosition(position.x, position.z);
    } else if (moveStyle === 'run') {
      // Run: animate from current position at double speed
      avatar.setRunSpeed(true);
      avatar.setTargetPosition(position.x, position.z);
    } else if (moveStyle.startsWith('teleport:')) {
      // Teleport with specified animation
      const animationType = moveStyle.substring(9) as TeleportAnimationType; // Remove 'teleport:' prefix
      avatar.teleportTo(position.x, position.z, animationType);
    } else {
      // Unknown move style - default to teleport with fade
      avatar.teleportTo(position.x, position.z, 'fade');
    }
  }

  /**
   * Fetch and apply profile picture and display name to an avatar
   */
  private async fetchAndApplyProfilePicture(pubkey: string, avatar: IAvatar, npub?: string): Promise<void> {
    console.log(`[Scene] fetchAndApplyProfilePicture called for pubkey: ${pubkey}, npub: ${npub}`);
    try {
      const profile = await profileCache.getProfile(pubkey, DEFAULT_RELAYS);
      console.log(`[Scene] Profile fetched:`, profile);

      // Set display name for initials fallback
      const displayName = profile?.display_name || profile?.name || npub?.slice(0, 12) || '';
      console.log(`[Scene] Setting display name: ${displayName}`);
      avatar.setDisplayName(displayName);

      // Set profile picture if available
      if (profile?.picture) {
        console.log(`[Scene] Setting profile picture: ${profile.picture}`);
        await avatar.setProfilePicture(profile.picture);
      } else {
        console.log(`[Scene] No profile picture available`);
      }
    } catch (error) {
      console.warn(`[Scene] Failed to fetch profile picture for ${pubkey}:`, error);
      // Still set display name for initials even if profile fetch fails
      if (npub) {
        avatar.setDisplayName(npub.slice(0, 12));
      }
    }
  }

  /**
   * Set teleport animation type
   */
  setTeleportAnimationType(type: TeleportAnimationType): void {
    this.teleportAnimationType = type;
  }

  /**
   * Get current teleport animation type
   */
  getTeleportAnimationType(): TeleportAnimationType {
    return this.teleportAnimationType;
  }

  /**
   * Get debug info for display
   */
  getDebugInfo() {
    const cursorSize = this.getCursorSize();
    const avatarPos = this.currentAvatar?.getPosition();

    const worldSize = getWorldSize(getTotalDepth(), getMicroDepth());

    return {
      cursorWorld: this.currentCursorCoord
        ? (() => {
            const [x, y, z] = cubeToWorld(this.currentCursorCoord);
            return { x, y, z };
          })()
        : undefined,
      cursorOctree: this.currentCursorCoord,
      cursorDepth: this.cursorDepth,
      cursorSize: cursorSize,
      avatarPos: avatarPos
        ? { x: avatarPos.x, y: avatarPos.y, z: avatarPos.z }
        : undefined,
      cameraPos: {
        x: this.camera.position.x,
        y: this.camera.position.y,
        z: this.camera.position.z
      },
      worldSize: worldSize,
      isEditMode: this.isEditMode,
      timeOfDay: this.sunSystem?.getTimeOfDay()
    };
  }

  /**
   * Get the sun system for external control
   */
  getSunSystem(): SunSystem | null {
    return this.sunSystem;
  }

  /**
   * Set time of day (0 to 1, where 0.5 is noon)
   */
  setTimeOfDay(time: number): void {
    this.sunSystem?.setTimeOfDay(time);
  }

  /**
   * Set sun movement speed
   */
  setSunSpeed(speed: number): void {
    this.sunSystem?.setSunSpeed(speed);
  }

  /**
   * Toggle automatic sun movement
   */
  setSunAutoMove(auto: boolean): void {
    this.sunSystem?.setAutoMove(auto);
  }

  /**
   * Set world grid visibility (origin axis, unit cube, world bounds, checkerboard)
   */
  setWorldGridVisible(visible: boolean): void {
    for (const helper of this.worldGridHelpers) {
      helper.visible = visible;
    }
  }

  /**
   * Cleanup all event listeners and DOM elements
   */
  dispose(): void {
    // Remove event listeners
    if (this.boundKeyDown) {
      window.removeEventListener('keydown', this.boundKeyDown);
    }
    if (this.boundKeyUp) {
      window.removeEventListener('keyup', this.boundKeyUp);
    }
    if (this.boundPointerLockChange) {
      document.removeEventListener('pointerlockchange', this.boundPointerLockChange);
    }

    // Remove crosshair element
    if (this.crosshair && this.crosshair.parentNode) {
      this.crosshair.parentNode.removeChild(this.crosshair);
      this.crosshair = null;
    }

    // Exit pointer lock if active
    if (document.pointerLockElement) {
      document.exitPointerLock();
    }

    console.log('[SceneManager] Disposed');
  }
}
