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
// import { scaleCubeCoord } from '../types/raycast-utils'; // TODO: Fix cursor scaling logic before re-enabling
import { getWorldSize } from '../constants/geometry';
import { getMacroDepth, getTotalDepth } from '../config/depth-config';
import { CheckerPlane } from './checker-plane';
import { loadCubeFromCsm, raycastWasm } from '../utils/cubeWasm';
import { raycastMesh, type MeshRaycastResult } from '../utils/meshRaycast';
import { SunSystem } from './sun-system';
import { PostProcessing } from './post-processing';
import { profileCache } from '../services/profile-cache';
import { DEFAULT_RELAYS } from '../config';
import * as logger from '../utils/logger';
// import { VoxelCursor } from './cursor';
// import { EditMode } from './edit-mode';
import { PlacementMode } from './placement-mode';
import type { MainMode } from '@crossworld/common';

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
  private wireframeMesh: THREE.LineSegments | null = null;
  private checkerPlane: CheckerPlane | null = null;
  private groundPlane: THREE.Plane = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0); // Plane at y=0
  private currentAvatar: IAvatar | null = null;
  private avatarEngine: AvatarEngine | null = null;
  private raycaster: THREE.Raycaster;
  private mouse: THREE.Vector2;
  private lastTime: number = 0;

  // Mode system
  private currentMode: MainMode = 'walk';
  // private voxelCursor: VoxelCursor | null = null;
  // private editMode: EditMode | null = null; // TODO: Use EditMode class instead of legacy edit mode
  private placementMode: PlacementMode | null = null;

  // Legacy edit mode properties (TODO: migrate to EditMode class)
  private isEditMode: boolean = false;
  private previewCube: THREE.LineSegments | null = null;
  private faceHighlightMesh: THREE.Mesh | null = null;
  private currentGridPosition: THREE.Vector3 = new THREE.Vector3();
  private currentCursorCoord: CubeCoord | null = null;

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

  // Wireframe mode
  private wireframeMode: boolean = false;

  // World cube for WASM raycasting
  private worldCube: any | null = null;

  // Raycast method: 'mesh' uses Three.js mesh raycasting, 'wasm' uses WASM aether raycasting
  private raycastMethod: 'mesh' | 'wasm' = 'mesh';

  // Depth voxel select mode: 1 = near side (y=0), 2 = far side (y=-1)
  private depthSelectMode: 1 | 2 = 1;

  // Cursor depth - single source of truth for current cursor depth
  // depth can be 0 to totalDepth (macro+micro, smaller depth = larger voxel size)
  // initialized to macroDepth (3)
  private cursorDepth: number = getMacroDepth();

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

  // Active edit plane (set when clicking voxel face in edit mode)
  private activeEditPlane: THREE.Plane | null = null;
  private activeEditPlaneNormal: THREE.Vector3 | null = null;
  private editPlaneGridHelper: THREE.GridHelper | null = null;

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
    this.setupFaceHighlight();
    this.setupOriginHelpers();
    this.setupCrosshair();
    this.setupCheckerPlane();

    // Initialize cursor and mode system
    // this.voxelCursor = new VoxelCursor(this.scene, this.cursorDepth);
    // this.editMode = new EditMode(this.voxelCursor); // TODO: Use EditMode class instead of legacy edit mode
    this.placementMode = new PlacementMode(this.scene);

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
      // Edit mode is now handled by mousedown/mouseup events for better responsiveness
      if (this.isEditMode) {
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

    // Mouse down handler - track continuous paint and set edit plane
    canvas.addEventListener('mousedown', (event) => {
      if (this.isEditMode) {
        if (event.button === 0) {
          // Left mouse button - detect plane and paint
          this.isLeftMousePressed = true;
          this.lastPaintedVoxel = null;
          this.detectAndSetEditPlane(event, canvas);
          // Also trigger initial paint
          this.handleEditModeClick(event, canvas, true);
        } else if (event.button === 2) {
          // Right mouse button - detect plane and erase
          this.isRightMousePressed = true;
          this.lastPaintedVoxel = null;
          this.detectAndSetEditPlane(event, canvas);
          // Also trigger initial erase
          this.handleEditModeClick(event, canvas, false);
        }
      }
    });

    // Mouse up handler - end continuous paint
    canvas.addEventListener('mouseup', (event) => {
      if (event.button === 0) {
        this.isLeftMousePressed = false;
        this.lastPaintedVoxel = null;
        this.clearActiveEditPlane();
      } else if (event.button === 2) {
        this.isRightMousePressed = false;
        this.lastPaintedVoxel = null;
        this.clearActiveEditPlane();
      }
    });

    // Mouse leave handler - end continuous paint when mouse leaves canvas
    canvas.addEventListener('mouseleave', () => {
      this.isLeftMousePressed = false;
      this.isRightMousePressed = false;
      this.lastPaintedVoxel = null;
      this.clearActiveEditPlane();
    });

    // Right click handler - prevent context menu in edit and placement modes
    canvas.addEventListener('contextmenu', (event) => {
      if (this.isEditMode || this.currentMode === 'placement') {
        event.preventDefault();
      }
    });
  }

  /**
   * Convert a raycast hit to a cursor cube coordinate
   * Properly scales the hit voxel coordinate to the current cursor depth
   * and adjusts for near/far placement
   *
   * @param hitCubeCoord The cube coordinate from raycast (at hit depth)
   * @param hitNormal The surface normal at hit point
   * @param placeFar If true, place on far side (away from camera), otherwise near side
   * @returns Cube coordinate at cursor depth, adjusted for placement side
   *
   * TODO: Fix cursor scaling logic - currently disabled due to positioning issues
   */
  // private hitToCursorCoord(
  //   hitCubeCoord: CubeCoord,
  //   hitNormal: THREE.Vector3,
  //   placeFar: boolean
  // ): CubeCoord {
  //   // Scale the hit coordinate to cursor depth
  //   let cursorCoord = scaleCubeCoord(hitCubeCoord, this.cursorDepth);
  //
  //   // If placing on far side (depth select mode 1), offset in normal direction
  //   if (placeFar) {
  //     const normalOffset = {
  //       x: Math.round(hitNormal.x),
  //       y: Math.round(hitNormal.y),
  //       z: Math.round(hitNormal.z)
  //     };
  //
  //     cursorCoord = {
  //       x: cursorCoord.x + normalOffset.x,
  //       y: cursorCoord.y + normalOffset.y,
  //       z: cursorCoord.z + normalOffset.z,
  //       depth: this.cursorDepth
  //     };
  //   }
  //
  //   return cursorCoord;
  // }

  /**
   * Perform raycast to geometry mesh
   * Returns hit point, normal, and cube coordinate, or null if no hit
   *
   * Supports two methods:
   * - 'mesh': Uses Three.js mesh raycasting (faster, works without WASM cube)
   * - 'wasm': Uses WASM aether raycasting (slower, more accurate for octree)
   */
  private raycastGeometry(): { point: THREE.Vector3; normal: THREE.Vector3; cubeCoord: CubeCoord } | null {
    if (!this.geometryMesh) return null;

    const size = this.getCursorSize();
    let result: MeshRaycastResult | null = null;

    try {
      if (this.raycastMethod === 'mesh') {
        // Use Three.js mesh raycasting
        result = raycastMesh(
          this.geometryMesh,
          this.raycaster,
          false, // far side = false (near side)
          getMacroDepth()
        );
      } else {
        // Use WASM raycasting
        if (!this.worldCube) return null;

        const worldSize = getWorldSize(getMacroDepth());
        const halfWorld = worldSize / 2;

        // Convert ray from world space to normalized [0, 1]
        const normalizedOrigin = new THREE.Vector3(
          (this.raycaster.ray.origin.x + halfWorld) / worldSize,
          (this.raycaster.ray.origin.y + halfWorld) / worldSize,
          (this.raycaster.ray.origin.z + halfWorld) / worldSize
        );

        const dir = this.raycaster.ray.direction;

        // Call WASM raycast
        result = raycastWasm(
          this.worldCube,
          normalizedOrigin.x, normalizedOrigin.y, normalizedOrigin.z,
          dir.x, dir.y, dir.z,
          false, // far side = false (near side)
          getMacroDepth()
        );
      }

      if (result) {
        const worldSize = getWorldSize(getMacroDepth());
        const halfWorld = worldSize / 2;

        // Convert hit position from normalized [0, 1] to world space
        const hitPoint = new THREE.Vector3(
          result.world_x * worldSize - halfWorld,
          result.world_y * worldSize - halfWorld,
          result.world_z * worldSize - halfWorld
        );

        // Normal is already in correct space
        const hitNormal = new THREE.Vector3(result.normal_x, result.normal_y, result.normal_z);

        // Apply epsilon adjustment
        const epsilon = size * 1e-6;
        hitPoint.x += hitNormal.x * epsilon;
        hitPoint.y += hitNormal.y * epsilon;
        hitPoint.z += hitNormal.z * epsilon;

        // Get the cube coordinate from raycast result
        const hitCubeCoord: CubeCoord = {
          x: result.x,
          y: result.y,
          z: result.z,
          depth: result.depth
        };

        return { point: hitPoint, normal: hitNormal, cubeCoord: hitCubeCoord };
      }
    } catch (error) {
      logger.error('renderer', `[Raycast] ${this.raycastMethod} raycast failed:`, error);
    }

    return null;
  }

  /**
   * Set the raycast method
   * @param method - 'mesh' for Three.js mesh raycasting, 'wasm' for WASM aether raycasting
   */
  public setRaycastMethod(method: 'mesh' | 'wasm'): void {
    this.raycastMethod = method;
    logger.log('renderer', `[Raycast] Switched to ${method} raycast method`);
  }

  /**
   * Get the current raycast method
   */
  public getRaycastMethod(): 'mesh' | 'wasm' {
    return this.raycastMethod;
  }

  /**
   * Detect voxel face and set active edit plane (without painting)
   */
  private detectAndSetEditPlane(event: MouseEvent, canvas: HTMLCanvasElement): void {
    // Calculate mouse position
    if (this.mouseMode === 2) {
      // Center of screen in shift mode
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

    // Raycast to geometry mesh using shared method
    const hit = this.raycastGeometry();
    if (hit) {
      this.setActiveEditPlane(hit.point, hit.normal);
    }
  }

  /**
   * Set the active edit plane based on a clicked face
   */
  private setActiveEditPlane(point: THREE.Vector3, normal: THREE.Vector3): void {
    // Get current voxel size
    const size = this.getCursorSize();
    const halfSize = size / 2;

    // Convert hit point to CubeCoord to snap to voxel grid
    const coord = worldToCube(point.x, point.y, point.z, this.cursorDepth);
    const [voxelX, voxelY, voxelZ] = cubeToWorld(coord);

    // Calculate voxel center
    const voxelCenterX = voxelX + halfSize;
    const voxelCenterY = voxelY + halfSize;
    const voxelCenterZ = voxelZ + halfSize;

    // Calculate face center: voxel center + normal * halfSize - unit normal
    const faceCenterX = voxelCenterX + normal.x * halfSize - normal.x;
    const faceCenterY = voxelCenterY + normal.y * halfSize - normal.y;
    const faceCenterZ = voxelCenterZ + normal.z * halfSize - normal.z;

    const faceCenter = new THREE.Vector3(faceCenterX, faceCenterY, faceCenterZ);

    // Store plane normal (use face center for grid alignment)
    this.activeEditPlaneNormal = normal.clone();

    // Create plane from face center and normal
    this.activeEditPlane = new THREE.Plane();
    this.activeEditPlane.setFromNormalAndCoplanarPoint(normal, faceCenter);

    // Remove old grid helper if exists
    if (this.editPlaneGridHelper) {
      this.scene.remove(this.editPlaneGridHelper);
      this.editPlaneGridHelper = null;
    }

    // Create grid helper for visualization
    const worldSize = getWorldSize(getMacroDepth());
    const gridSize = worldSize; // Grid covers world size
    const divisions = worldSize / size; // Grid divisions match voxel size

    // GridHelper is always horizontal, so we need to rotate it based on normal
    this.editPlaneGridHelper = new THREE.GridHelper(gridSize, divisions, 0x00ff00, 0x00ff00);
    this.editPlaneGridHelper.material.transparent = true;
    this.editPlaneGridHelper.material.opacity = 0.3;
    this.editPlaneGridHelper.material.depthTest = true;

    // Rotate grid to match plane normal first
    // Default GridHelper normal is (0, 1, 0) - Y-up
    const defaultNormal = new THREE.Vector3(0, 1, 0);
    const quaternion = new THREE.Quaternion().setFromUnitVectors(defaultNormal, normal);
    this.editPlaneGridHelper.setRotationFromQuaternion(quaternion);

    // Position grid to align with world coordinates like CheckerPlane
    // The grid should be at world origin (0, 0, 0) but translated along normal to reach the face
    // Calculate distance from origin to face plane along normal
    const distanceAlongNormal = faceCenter.dot(normal);

    // Position grid along the normal direction only, keeping it centered in the other axes
    this.editPlaneGridHelper.position.copy(normal.clone().multiplyScalar(distanceAlongNormal));

    this.scene.add(this.editPlaneGridHelper);

    logger.log('renderer', '[Active Edit Plane]', {
      faceCenter: { x: faceCenterX, y: faceCenterY, z: faceCenterZ },
      normal: { x: normal.x, y: normal.y, z: normal.z }
    });
  }

  /**
   * Clear the active edit plane
   */
  private clearActiveEditPlane(): void {
    this.activeEditPlane = null;
    this.activeEditPlaneNormal = null;

    if (this.editPlaneGridHelper) {
      this.scene.remove(this.editPlaneGridHelper);
      this.editPlaneGridHelper = null;
    }
  }

  /**
   * Setup face highlight mesh
   */
  private setupFaceHighlight(): void {
    // Create a plane geometry for the face highlight
    const geometry = new THREE.PlaneGeometry(1, 1);
    const material = new THREE.MeshBasicMaterial({
      color: 0xffffff,
      transparent: true,
      opacity: 0.3,
      side: THREE.DoubleSide,
      depthTest: true,
      depthWrite: false
    });
    this.faceHighlightMesh = new THREE.Mesh(geometry, material);
    this.faceHighlightMesh.visible = false;
    this.faceHighlightMesh.renderOrder = 998; // Below voxel cursor (999)
    this.scene.add(this.faceHighlightMesh);
  }

  /**
   * Update face highlight based on CubeCoord and face normal
   */
  private updateFaceHighlight(point: THREE.Vector3, normal: THREE.Vector3, size: number): void {
    if (!this.faceHighlightMesh) return;

    // Convert world hit point to CubeCoord
    const coord = worldToCube(point.x, point.y, point.z, this.cursorDepth);

    // Convert CubeCoord back to world position (corner of voxel)
    const [voxelX, voxelY, voxelZ] = cubeToWorld(coord);

    // Calculate voxel center in world space
    const halfSize = size / 2;
    const voxelCenterX = voxelX + halfSize;
    const voxelCenterY = voxelY + halfSize;
    const voxelCenterZ = voxelZ + halfSize;

    // Calculate face center: voxel center + (normal * half_size) - unit normal
    // This positions the highlight at the center of the face
    const faceCenterX = voxelCenterX + normal.x * halfSize - normal.x;
    const faceCenterY = voxelCenterY + normal.y * halfSize - normal.y;
    const faceCenterZ = voxelCenterZ + normal.z * halfSize - normal.z;

    // Add small offset along normal to prevent z-fighting
    const offset = 0.01;
    const position = new THREE.Vector3(
      faceCenterX + normal.x * offset,
      faceCenterY + normal.y * offset,
      faceCenterZ + normal.z * offset
    );

    // Orient to face the normal direction
    const defaultNormal = new THREE.Vector3(0, 0, 1);
    const quaternion = new THREE.Quaternion().setFromUnitVectors(defaultNormal, normal);

    // Update face highlight mesh
    this.faceHighlightMesh.position.copy(position);
    this.faceHighlightMesh.scale.set(size, size, 1);
    this.faceHighlightMesh.setRotationFromQuaternion(quaternion);
    this.faceHighlightMesh.visible = true;
  }

  /**
   * Hide face highlight
   */
  private hideFaceHighlight(): void {
    if (this.faceHighlightMesh) {
      this.faceHighlightMesh.visible = false;
    }
  }

  private handleEditModeClick(event: MouseEvent, canvas: HTMLCanvasElement, isLeftClick: boolean): void {
    logger.log('renderer', '[Edit Click]', { isLeftClick, mouseMode: this.mouseMode, cursorDepth: this.cursorDepth });

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

    const size = this.getCursorSize();
    const halfSize = size / 2;

    // First, try to raycast to geometry mesh to detect clicked face
    let voxelX = 0;
    let voxelY = 0;
    let voxelZ = 0;
    let hasGeometryHit = false;

    // Raycast to geometry mesh using shared method
    const hit = this.raycastGeometry();
    if (hit) {
      // Set active edit plane based on hit face normal
      this.setActiveEditPlane(hit.point, hit.normal);

      // Calculate voxel position using SPACE mode (same as cursor)
      // Convert hit point to CubeCoord to snap to voxel grid
      const coord = worldToCube(hit.point.x, hit.point.y, hit.point.z, this.cursorDepth);
      const [hitVoxelX, hitVoxelY, hitVoxelZ] = cubeToWorld(coord);

      // Calculate voxel center of the hit voxel
      const voxelCenterX = hitVoxelX + halfSize;
      const voxelCenterY = hitVoxelY + halfSize;
      const voxelCenterZ = hitVoxelZ + halfSize;

      // Calculate face center
      const faceCenterX = voxelCenterX + hit.normal.x * halfSize - hit.normal.x;
      const faceCenterY = voxelCenterY + hit.normal.y * halfSize - hit.normal.y;
      const faceCenterZ = voxelCenterZ + hit.normal.z * halfSize - hit.normal.z;

      // Move from face center to voxel area center based on depth select mode
      // Mode 1: voxel on positive normal side (outward from face)
      // Mode 2: voxel on negative normal side (inward from face)
      const normalOffset = this.depthSelectMode === 1 ? halfSize : -halfSize;
      const cursorX = faceCenterX + hit.normal.x * normalOffset;
      const cursorY = faceCenterY + hit.normal.y * normalOffset;
      const cursorZ = faceCenterZ + hit.normal.z * normalOffset;

      // Calculate corner position for the cursor voxel
      voxelX = cursorX - halfSize;
      voxelY = cursorY - halfSize;
      voxelZ = cursorZ - halfSize;

      hasGeometryHit = true;
    }

    // If no geometry hit, use plane intersection
    if (!hasGeometryHit) {
      const targetPlane = this.activeEditPlane || this.groundPlane;
      const intersectPoint = new THREE.Vector3();
      const didIntersect = this.raycaster.ray.intersectPlane(targetPlane, intersectPoint);

      logger.log('renderer', '[Raycast]', { didIntersect, intersectPoint: didIntersect ? intersectPoint : null });

      if (!didIntersect) return;

      // Snap all three axes to grid centered on the intersection point
      const voxelCenterX = snapToGrid(intersectPoint.x, size);
      const voxelCenterY = snapToGrid(intersectPoint.y, size);
      const voxelCenterZ = snapToGrid(intersectPoint.z, size);

      // Calculate corner position (world space)
      voxelX = voxelCenterX - halfSize;
      voxelZ = voxelCenterZ - halfSize;

      // For ground plane mode (no active edit plane), use depth select mode
      // For active edit plane, snap Y coordinate to grid like X and Z
      voxelY = targetPlane !== this.groundPlane
        ? voxelCenterY - halfSize
        : (this.depthSelectMode === 1 ? 0 : -size);
    }

    logger.log('renderer', '[Voxel Pos]', { voxelX, voxelY, voxelZ, size });

    // Check if within world bounds (all axes)
    const halfWorld = getWorldSize(getMacroDepth()) / 2;
    const isInBounds =
      voxelX >= -halfWorld && voxelX + size <= halfWorld &&
      voxelY >= -halfWorld && voxelY + size <= halfWorld &&
      voxelZ >= -halfWorld && voxelZ + size <= halfWorld;

    if (isInBounds) {
      logger.log('renderer', '[Voxel Action]', isLeftClick ? 'paint' : 'erase');
      if (isLeftClick) {
        // Left click: use current color/erase mode
        this.paintVoxelWithSize(voxelX, voxelY, voxelZ, size);
      } else {
        // Right click always removes voxel
        this.eraseVoxelWithSize(voxelX, voxelY, voxelZ, size);
      }
    } else {
      logger.log('renderer', '[Out of Bounds]', { voxelX, voxelY, voxelZ, size });
    }
  }

  private onVoxelEdit?: (coord: CubeCoord, colorIndex: number) => void;

  setOnVoxelEdit(callback: (coord: CubeCoord, colorIndex: number) => void): void {
    this.onVoxelEdit = callback;
  }

  setSelectedColorIndex(colorIndex: number): void {
    this.selectedColorIndex = colorIndex;
  }

  async setSelectedModel(modelPath: string): Promise<void> {
    if (this.placementMode) {
      await this.placementMode.setModel(modelPath);
    }
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
    logger.log('renderer', '[Paint Voxel]', { x, y, z, size, selectedColor: this.selectedColorIndex });

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

    logger.log('renderer', '[Paint -> CubeCoord]', { coord, colorValue, hasCallback: !!this.onVoxelEdit });

    // Call onVoxelEdit with CubeCoord
    this.onVoxelEdit?.(coord, colorValue);
  }

  private eraseVoxelWithSize(x: number, y: number, z: number, size: number): void {
    logger.log('renderer', '[Erase Voxel]', { x, y, z, size });

    // Convert world coordinates to cube coordinates
    const coord = worldToCube(x, y, z, this.cursorDepth);

    logger.log('renderer', '[Erase -> CubeCoord]', { coord, hasCallback: !!this.onVoxelEdit });

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
          logger.log('renderer', '[Mouse Mode] Switched to mode 2 (first-person camera rotation)');
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
          this.clearActiveEditPlane();
          logger.log('renderer', '[Mouse Mode] Switched to mode 1 (paint/erase)');
        }
        return;
      }

      // Toggle edit mode with 'e' key (works in both walk and edit modes)
      if (event.key === 'e' || event.key === 'E') {
        this.setEditMode(!this.isEditMode);
        // Reset avatar movement state when toggling edit mode
        this.resetAvatarMovementState();
        logger.log('renderer', `[Edit Mode] Toggled to ${this.isEditMode ? 'ON' : 'OFF'}`);
        return;
      }

      // Cursor depth/scale control with Arrow Up/Down (works in edit and placement modes)
      if (event.code === 'ArrowUp') {
        event.preventDefault();

        // Placement mode: increase model scale
        if (this.currentMode === 'placement' && this.placementMode) {
          const currentScale = this.placementMode.getScale();
          this.placementMode.setScale(currentScale + 1);
          logger.log('renderer', `[Placement Scale] Increased to ${currentScale + 1}`);
        }
        // Edit mode: increase cursor depth
        else if (this.isEditMode) {
          this.cursorDepth = Math.min(getTotalDepth(), this.cursorDepth + 1);
          this.updateCursorSize();
          logger.log('renderer', `[Cursor Depth] Increased to ${this.cursorDepth} (size=${this.getCursorSize()})`);
          // Update cursor position immediately
          if (this.mouseMode === 2) {
            this.updateVoxelCursorAtCenter();
          } else {
            this.updateCursorVisualization();
          }
        }
      }

      if (event.code === 'ArrowDown') {
        event.preventDefault();

        // Placement mode: decrease model scale
        if (this.currentMode === 'placement' && this.placementMode) {
          const currentScale = this.placementMode.getScale();
          this.placementMode.setScale(Math.max(0, currentScale - 1));
          logger.log('renderer', `[Placement Scale] Decreased to ${Math.max(0, currentScale - 1)}`);
        }
        // Edit mode: decrease cursor depth
        else if (this.isEditMode) {
          this.cursorDepth = Math.max(0, this.cursorDepth - 1);
          this.updateCursorSize();
          logger.log('renderer', `[Cursor Depth] Decreased to ${this.cursorDepth} (size=${this.getCursorSize()})`);
          // Update cursor position immediately
          if (this.mouseMode === 2) {
            this.updateVoxelCursorAtCenter();
          } else {
            this.updateCursorVisualization();
          }
        }
      }

      // Edit mode specific controls
      if (!this.isEditMode) return;

      // Toggle depth select mode with Spacebar
      if (event.code === 'Space') {
        event.preventDefault();
        this.depthSelectMode = this.depthSelectMode === 1 ? 2 : 1;
        logger.log('renderer', `[Depth Select] Switched to mode ${this.depthSelectMode} (y=${this.depthSelectMode === 1 ? 0 : -1})`);
        // Update cursor position immediately
        if (this.mouseMode === 2) {
          this.updateVoxelCursorAtCenter();
        } else {
          this.updateCursorVisualization();
        }
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
        this.clearActiveEditPlane();
        logger.log('renderer', '[Mouse Mode] Pointer lock exited (Escape), switched to mode 1');
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
      color: 0xffffff, // White
      linewidth: 2,
      opacity: 0.6, // 60% transparent
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
    const worldSize = getWorldSize(getMacroDepth());
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

  /**
   * Update cursor visualization at current mouse position
   * Called from mouse move and keyboard handlers
   */
  private updateCursorVisualization(mouseX?: number, mouseY?: number): void {
    // Edit mode only
    if (!this.isEditMode || !this.previewCube) {
      this.hideFaceHighlight();
      return;
    }

    // Use provided mouse position or current mouse position
    if (mouseX !== undefined && mouseY !== undefined) {
      this.mouse.x = mouseX;
      this.mouse.y = mouseY;
    }
    // Otherwise use this.mouse.x and this.mouse.y which are already set

    // Update raycaster
    this.raycaster.setFromCamera(this.mouse, this.camera);

    const size = this.getCursorSize();
    const halfSize = size / 2;

    // When draw plane is active, prioritize raycasting to the plane
    // Otherwise raycast to geometry mesh
    let hasPlaneHit = false;
    let planeHitPoint: THREE.Vector3 | null = null;
    let hasGeometryHit = false;
    let geometryHitPoint: THREE.Vector3 | null = null;
    let geometryHitNormal: THREE.Vector3 | null = null;

    if (this.activeEditPlane && this.activeEditPlaneNormal) {
      // Draw plane is active - always raycast to the plane (even during draw mode)
      const intersectPoint = new THREE.Vector3();
      const didIntersect = this.raycaster.ray.intersectPlane(this.activeEditPlane, intersectPoint);

      if (didIntersect) {
        hasPlaneHit = true;
        planeHitPoint = intersectPoint;

        // Show face highlight on the plane using the plane's normal
        this.updateFaceHighlight(intersectPoint, this.activeEditPlaneNormal, size);
      } else {
        this.hideFaceHighlight();
      }
    } else if (this.geometryMesh && !this.isLeftMousePressed && !this.isRightMousePressed) {
      // No draw plane active and not drawing - raycast to geometry mesh using shared method
      const hit = this.raycastGeometry();
      if (hit) {
        this.updateFaceHighlight(hit.point, hit.normal, size);

        // Store hit info for voxel cursor positioning
        hasGeometryHit = true;
        geometryHitPoint = hit.point;
        geometryHitNormal = hit.normal;
      } else {
        this.hideFaceHighlight();
      }
    } else {
      this.hideFaceHighlight();
    }

    // Position voxel cursor
    if (hasPlaneHit && planeHitPoint && this.activeEditPlaneNormal) {
      // Draw plane hit - position cursor on plane with SPACE mode
      // Snap intersection point to grid
      const snappedCenter = new THREE.Vector3(
        snapToGrid(planeHitPoint.x, size),
        snapToGrid(planeHitPoint.y, size),
        snapToGrid(planeHitPoint.z, size)
      );

      // Project snapped point back onto the plane
      const distanceToPlane = this.activeEditPlane!.distanceToPoint(snappedCenter);
      const projectedCenter = snappedCenter.clone().sub(
        this.activeEditPlaneNormal.clone().multiplyScalar(distanceToPlane)
      );

      // Apply SPACE mode offset
      const normalOffset = this.depthSelectMode === 1 ? halfSize : -halfSize;
      const cursorX = projectedCenter.x + this.activeEditPlaneNormal.x * normalOffset;
      const cursorY = projectedCenter.y + this.activeEditPlaneNormal.y * normalOffset;
      const cursorZ = projectedCenter.z + this.activeEditPlaneNormal.z * normalOffset;

      // Calculate corner position for the cursor voxel
      const voxelX = cursorX - halfSize;
      const voxelY = cursorY - halfSize;
      const voxelZ = cursorZ - halfSize;

      // Check if within world bounds
      const halfWorld = getWorldSize(getMacroDepth()) / 2;
      const isInBounds =
        voxelX >= -halfWorld && voxelX + size <= halfWorld &&
        voxelY >= -halfWorld && voxelY + size <= halfWorld &&
        voxelZ >= -halfWorld && voxelZ + size <= halfWorld;

      if (isInBounds) {
        // Store current cursor coordinate
        const newCoord = worldToCube(voxelX, voxelY, voxelZ, this.cursorDepth);

        // Only log when coordinate changes
        if (!this.currentCursorCoord ||
            this.currentCursorCoord.x !== newCoord.x ||
            this.currentCursorCoord.y !== newCoord.y ||
            this.currentCursorCoord.z !== newCoord.z) {
          console.log('[MOUSEMOVE CURSOR] DrawPlane CHANGED:', {
            oldCoord: this.currentCursorCoord,
            newCoord,
            planeHitPoint,
            snappedCenter: { x: snappedCenter.x, y: snappedCenter.y, z: snappedCenter.z },
            projectedCenter: { x: projectedCenter.x, y: projectedCenter.y, z: projectedCenter.z },
            cursorCenter: { x: cursorX, y: cursorY, z: cursorZ },
            voxelCorner: { x: voxelX, y: voxelY, z: voxelZ }
          });
        }

        this.currentCursorCoord = newCoord;

        // Position preview cube at voxel area center
        this.currentGridPosition.set(cursorX, cursorY, cursorZ);
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
        this.previewCube.visible = false;
        this.currentCursorCoord = null;
      }
    } else if (hasGeometryHit && geometryHitPoint && geometryHitNormal) {
      // Geometry hit - position cursor using SPACE mode
      // Convert hit point to CubeCoord to snap to voxel grid
      const coord = worldToCube(geometryHitPoint.x, geometryHitPoint.y, geometryHitPoint.z, this.cursorDepth);
      const [hitVoxelX, hitVoxelY, hitVoxelZ] = cubeToWorld(coord);

      // Calculate voxel center of the hit voxel
      const voxelCenterX = hitVoxelX + halfSize;
      const voxelCenterY = hitVoxelY + halfSize;
      const voxelCenterZ = hitVoxelZ + halfSize;

      // Calculate face center
      const faceCenterX = voxelCenterX + geometryHitNormal.x * halfSize - geometryHitNormal.x;
      const faceCenterY = voxelCenterY + geometryHitNormal.y * halfSize - geometryHitNormal.y;
      const faceCenterZ = voxelCenterZ + geometryHitNormal.z * halfSize - geometryHitNormal.z;

      // Move from face center to voxel area center based on depth select mode
      const normalOffset = this.depthSelectMode === 1 ? halfSize : -halfSize;
      const cursorX = faceCenterX + geometryHitNormal.x * normalOffset;
      const cursorY = faceCenterY + geometryHitNormal.y * normalOffset;
      const cursorZ = faceCenterZ + geometryHitNormal.z * normalOffset;

      // Calculate corner position for the cursor voxel
      const voxelX = cursorX - halfSize;
      const voxelY = cursorY - halfSize;
      const voxelZ = cursorZ - halfSize;

      // Check if within world bounds
      const halfWorld = getWorldSize(getMacroDepth()) / 2;
      const isInBounds =
        voxelX >= -halfWorld && voxelX + size <= halfWorld &&
        voxelY >= -halfWorld && voxelY + size <= halfWorld &&
        voxelZ >= -halfWorld && voxelZ + size <= halfWorld;

      if (isInBounds) {
        // Store current cursor coordinate
        const newCoord = worldToCube(voxelX, voxelY, voxelZ, this.cursorDepth);

        // Only log when coordinate changes
        if (!this.currentCursorCoord ||
            this.currentCursorCoord.x !== newCoord.x ||
            this.currentCursorCoord.y !== newCoord.y ||
            this.currentCursorCoord.z !== newCoord.z) {
          console.log('[MOUSEMOVE CURSOR] GeometryHit CHANGED:', {
            oldCoord: this.currentCursorCoord,
            newCoord,
            hitPoint: geometryHitPoint,
            hitNormal: geometryHitNormal,
            voxelCenter: { x: voxelCenterX, y: voxelCenterY, z: voxelCenterZ },
            faceCenter: { x: faceCenterX, y: faceCenterY, z: faceCenterZ },
            cursorCenter: { x: cursorX, y: cursorY, z: cursorZ },
            voxelCorner: { x: voxelX, y: voxelY, z: voxelZ }
          });
        }

        this.currentCursorCoord = newCoord;

        // Position preview cube at voxel area center
        this.currentGridPosition.set(cursorX, cursorY, cursorZ);
        this.previewCube.position.copy(this.currentGridPosition);
        this.previewCube.visible = true;
      } else {
        this.previewCube.visible = false;
        this.currentCursorCoord = null;
      }
    } else {
      this.previewCube.visible = false;
      this.currentCursorCoord = null;
    }
  }


  private updateVoxelCursorAtCenter(): void {
    if (!this.previewCube) return;

    // Raycast from center of screen
    this.mouse.x = 0;
    this.mouse.y = 0;
    this.raycaster.setFromCamera(this.mouse, this.camera);

    const size = this.getCursorSize();
    const halfSize = size / 2;

    // When draw plane is active, prioritize raycasting to the plane
    // Otherwise raycast to geometry mesh
    let hasPlaneHit = false;
    let planeHitPoint: THREE.Vector3 | null = null;
    let hasGeometryHit = false;
    let geometryHitPoint: THREE.Vector3 | null = null;
    let geometryHitNormal: THREE.Vector3 | null = null;

    if (this.activeEditPlane && this.activeEditPlaneNormal && this.isEditMode) {
      // Draw plane is active - always raycast to the plane (even during draw mode)
      const intersectPoint = new THREE.Vector3();
      const didIntersect = this.raycaster.ray.intersectPlane(this.activeEditPlane, intersectPoint);

      if (didIntersect) {
        hasPlaneHit = true;
        planeHitPoint = intersectPoint;

        // Show face highlight on the plane using the plane's normal
        this.updateFaceHighlight(intersectPoint, this.activeEditPlaneNormal, size);
      } else {
        this.hideFaceHighlight();
      }
    } else if (this.geometryMesh && this.isEditMode && !this.isLeftMousePressed && !this.isRightMousePressed) {
      // No draw plane active and not drawing - raycast to geometry mesh (shift-rotate mode)
      const hit = this.raycastGeometry();
      if (hit) {
        this.updateFaceHighlight(hit.point, hit.normal, size);

        // Store hit info for voxel cursor positioning
        hasGeometryHit = true;
        geometryHitPoint = hit.point;
        geometryHitNormal = hit.normal;
      } else {
        this.hideFaceHighlight();
      }
    } else {
      this.hideFaceHighlight();
    }

    // Position voxel cursor
    if (hasPlaneHit && planeHitPoint && this.activeEditPlaneNormal) {
      // Draw plane hit - position cursor on plane with SPACE mode
      // Snap intersection point to grid
      const snappedCenter = new THREE.Vector3(
        snapToGrid(planeHitPoint.x, size),
        snapToGrid(planeHitPoint.y, size),
        snapToGrid(planeHitPoint.z, size)
      );

      // Project snapped point back onto the plane
      const distanceToPlane = this.activeEditPlane!.distanceToPoint(snappedCenter);
      const projectedCenter = snappedCenter.clone().sub(
        this.activeEditPlaneNormal.clone().multiplyScalar(distanceToPlane)
      );

      // Apply SPACE mode offset
      const normalOffset = this.depthSelectMode === 1 ? halfSize : -halfSize;
      const cursorX = projectedCenter.x + this.activeEditPlaneNormal.x * normalOffset;
      const cursorY = projectedCenter.y + this.activeEditPlaneNormal.y * normalOffset;
      const cursorZ = projectedCenter.z + this.activeEditPlaneNormal.z * normalOffset;

      // Calculate corner position for the cursor voxel
      const voxelX = cursorX - halfSize;
      const voxelY = cursorY - halfSize;
      const voxelZ = cursorZ - halfSize;

      // Check if within world bounds
      const halfWorld = getWorldSize(getMacroDepth()) / 2;
      const isInBounds =
        voxelX >= -halfWorld && voxelX + size <= halfWorld &&
        voxelY >= -halfWorld && voxelY + size <= halfWorld &&
        voxelZ >= -halfWorld && voxelZ + size <= halfWorld;

      if (isInBounds) {
        // Store current cursor coordinate
        this.currentCursorCoord = worldToCube(voxelX, voxelY, voxelZ, this.cursorDepth);

        // Position preview cube at voxel area center
        this.currentGridPosition.set(cursorX, cursorY, cursorZ);
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
    } else if (hasGeometryHit && geometryHitPoint && geometryHitNormal) {
      // Position cursor at the voxel area (not face center)
      const size = this.getCursorSize();
      const halfSize = size / 2;

      // Convert hit point to CubeCoord to snap to voxel grid
      const coord = worldToCube(geometryHitPoint.x, geometryHitPoint.y, geometryHitPoint.z, this.cursorDepth);
      const [voxelX, voxelY, voxelZ] = cubeToWorld(coord);

      // Calculate voxel center
      const voxelCenterX = voxelX + halfSize;
      const voxelCenterY = voxelY + halfSize;
      const voxelCenterZ = voxelZ + halfSize;

      // Calculate face center (same as face highlight)
      const faceCenterX = voxelCenterX + geometryHitNormal.x * halfSize - geometryHitNormal.x;
      const faceCenterY = voxelCenterY + geometryHitNormal.y * halfSize - geometryHitNormal.y;
      const faceCenterZ = voxelCenterZ + geometryHitNormal.z * halfSize - geometryHitNormal.z;

      // Move from face center to voxel area center based on depth select mode
      // Mode 1: voxel on positive normal side (outward from face)
      // Mode 2: voxel on negative normal side (inward from face)
      const normalOffset = this.depthSelectMode === 1 ? halfSize : -halfSize;
      const cursorX = faceCenterX + geometryHitNormal.x * normalOffset;
      const cursorY = faceCenterY + geometryHitNormal.y * normalOffset;
      const cursorZ = faceCenterZ + geometryHitNormal.z * normalOffset;

      // Calculate corner position for the cursor voxel
      const cursorVoxelX = cursorX - halfSize;
      const cursorVoxelY = cursorY - halfSize;
      const cursorVoxelZ = cursorZ - halfSize;

      // Check if within world bounds
      const halfWorld = getWorldSize(getMacroDepth()) / 2;
      const isInBounds =
        cursorVoxelX >= -halfWorld && cursorVoxelX + size <= halfWorld &&
        cursorVoxelY >= -halfWorld && cursorVoxelY + size <= halfWorld &&
        cursorVoxelZ >= -halfWorld && cursorVoxelZ + size <= halfWorld;

      if (isInBounds) {
        // Store cursor coordinate using cursor voxel position
        this.currentCursorCoord = worldToCube(cursorVoxelX, cursorVoxelY, cursorVoxelZ, this.cursorDepth);

        // Position preview cube at voxel area center
        this.currentGridPosition.set(cursorX, cursorY, cursorZ);
        this.previewCube.position.copy(this.currentGridPosition);
        this.previewCube.visible = true;

        // Continuous paint in shift mode: if mouse button is pressed, paint/erase voxel at new position
        if (this.isLeftMousePressed || this.isRightMousePressed) {
          // Check if this is a new voxel position (different from last painted)
          const isNewPosition = !this.lastPaintedVoxel ||
            this.lastPaintedVoxel.x !== cursorVoxelX ||
            this.lastPaintedVoxel.y !== cursorVoxelY ||
            this.lastPaintedVoxel.z !== cursorVoxelZ;

          if (isNewPosition) {
            if (this.isLeftMousePressed) {
              // Left mouse: draw with selected color
              this.paintVoxelWithSize(cursorVoxelX, cursorVoxelY, cursorVoxelZ, size);
            } else if (this.isRightMousePressed) {
              // Right mouse: erase
              this.eraseVoxelWithSize(cursorVoxelX, cursorVoxelY, cursorVoxelZ, size);
            }
            this.lastPaintedVoxel = { x: cursorVoxelX, y: cursorVoxelY, z: cursorVoxelZ };
          }
        }
      } else {
        // Outside bounds - hide cursor
        this.previewCube.visible = false;
        this.currentCursorCoord = null;
      }
    } else {
      // Use active edit plane if available, otherwise ground plane
      const targetPlane = this.activeEditPlane || this.groundPlane;
      const intersectPoint = new THREE.Vector3();
      const didIntersect = this.raycaster.ray.intersectPlane(targetPlane, intersectPoint);

      if (didIntersect) {
        const size = this.getCursorSize();
        const halfSize = size / 2;

        // Snap all three axes to grid centered on the intersection point
        const snappedCenter = new THREE.Vector3(
          snapToGrid(intersectPoint.x, size),
          snapToGrid(intersectPoint.y, size),
          snapToGrid(intersectPoint.z, size)
        );

        // For active edit plane, project back onto plane and apply SPACE mode
        // For ground plane, use ground plane normal (Y-up)
        let voxelX: number, voxelY: number, voxelZ: number;

        if (this.activeEditPlane && this.activeEditPlaneNormal) {
          // Project snapped point back onto the plane
          const distanceToPlane = this.activeEditPlane.distanceToPoint(snappedCenter);
          const projectedCenter = snappedCenter.clone().sub(
            this.activeEditPlaneNormal.clone().multiplyScalar(distanceToPlane)
          );

          // Apply SPACE mode offset
          const normalOffset = this.depthSelectMode === 1 ? halfSize : -halfSize;
          const cursorX = projectedCenter.x + this.activeEditPlaneNormal.x * normalOffset;
          const cursorY = projectedCenter.y + this.activeEditPlaneNormal.y * normalOffset;
          const cursorZ = projectedCenter.z + this.activeEditPlaneNormal.z * normalOffset;

          // Calculate corner position
          voxelX = cursorX - halfSize;
          voxelY = cursorY - halfSize;
          voxelZ = cursorZ - halfSize;
        } else {
          // Ground plane mode: use depth select mode
          voxelX = snappedCenter.x - halfSize;
          voxelZ = snappedCenter.z - halfSize;
          voxelY = this.depthSelectMode === 1 ? 0 : -size;
        }

        // Check if within world bounds (all axes)
        const halfWorld = getWorldSize(getMacroDepth()) / 2;
        const isInBounds =
          voxelX >= -halfWorld && voxelX + size <= halfWorld &&
          voxelY >= -halfWorld && voxelY + size <= halfWorld &&
          voxelZ >= -halfWorld && voxelZ + size <= halfWorld;

        if (isInBounds) {
          // Store current cursor coordinate (using corner position)
          this.currentCursorCoord = worldToCube(voxelX, voxelY, voxelZ, this.cursorDepth);

          // Position preview cube at center of voxel (world space)
          this.currentGridPosition.set(voxelX + halfSize, voxelY + halfSize, voxelZ + halfSize);
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

      // Placement mode: update model cursor position
      if (this.currentMode === 'placement' && this.placementMode) {
        const rect = canvas.getBoundingClientRect();
        const mouseX = ((event.clientX - rect.left) / rect.width) * 2 - 1;
        const mouseY = -((event.clientY - rect.top) / rect.height) * 2 + 1;

        this.mouse.x = mouseX;
        this.mouse.y = mouseY;
        this.raycaster.setFromCamera(this.mouse, this.camera);

        // Raycast to geometry or ground
        const hit = this.raycastGeometry();
        if (hit) {
          // Position model at hit point
          this.placementMode.setPosition(hit.point.x, hit.point.y, hit.point.z);
          this.placementMode.show();

          // Calculate cube coord for placement
          const brush = this.placementMode.getBrush();
          const baseSize = Math.pow(2, brush.size);
          const scaleMultiplier = Math.pow(2, brush.scale);
          const finalSize = baseSize * scaleMultiplier;
          const halfSize = finalSize / 2;

          // Calculate corner position
          const voxelX = hit.point.x - halfSize;
          const voxelY = hit.point.y - halfSize;
          const voxelZ = hit.point.z - halfSize;

          // Convert to cube coordinate
          const coord = worldToCube(voxelX, voxelY, voxelZ, brush.size);
          this.placementMode.setCursorCoord(coord);
        } else {
          this.placementMode.hide();
        }
        return;
      }

      // Edit mode only: update voxel cursor and face highlight
      if (!this.isEditMode || !this.previewCube) {
        this.hideFaceHighlight();
        return;
      }

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

      const size = this.getCursorSize();
      const halfSize = size / 2;

      // When draw plane is active, prioritize raycasting to the plane
      // Otherwise raycast to geometry mesh
      let hasPlaneHit = false;
      let planeHitPoint: THREE.Vector3 | null = null;
      let hasGeometryHit = false;
      let geometryHitPoint: THREE.Vector3 | null = null;
      let geometryHitNormal: THREE.Vector3 | null = null;

      if (this.activeEditPlane && this.activeEditPlaneNormal) {
        // Draw plane is active - always raycast to the plane (even during draw mode)
        const intersectPoint = new THREE.Vector3();
        const didIntersect = this.raycaster.ray.intersectPlane(this.activeEditPlane, intersectPoint);

        if (didIntersect) {
          hasPlaneHit = true;
          planeHitPoint = intersectPoint;

          // Show face highlight on the plane using the plane's normal
          this.updateFaceHighlight(intersectPoint, this.activeEditPlaneNormal, size);
        } else {
          this.hideFaceHighlight();
        }
      } else if (this.geometryMesh && !this.isLeftMousePressed && !this.isRightMousePressed) {
        // No draw plane active and not drawing - raycast to geometry mesh using shared method
        const hit = this.raycastGeometry();
        if (hit) {
          this.updateFaceHighlight(hit.point, hit.normal, size);

          // Store hit info for voxel cursor positioning
          hasGeometryHit = true;
          geometryHitPoint = hit.point;
          geometryHitNormal = hit.normal;
        } else {
          this.hideFaceHighlight();
        }
      } else {
        this.hideFaceHighlight();
      }

      // Position voxel cursor
      if (hasPlaneHit && planeHitPoint && this.activeEditPlaneNormal) {
        // Draw plane hit - position cursor on plane with SPACE mode
        // Snap intersection point to grid
        const snappedCenter = new THREE.Vector3(
          snapToGrid(planeHitPoint.x, size),
          snapToGrid(planeHitPoint.y, size),
          snapToGrid(planeHitPoint.z, size)
        );

        // Project snapped point back onto the plane
        const distanceToPlane = this.activeEditPlane!.distanceToPoint(snappedCenter);
        const projectedCenter = snappedCenter.clone().sub(
          this.activeEditPlaneNormal.clone().multiplyScalar(distanceToPlane)
        );

        // Apply SPACE mode offset
        const normalOffset = this.depthSelectMode === 1 ? halfSize : -halfSize;
        const cursorX = projectedCenter.x + this.activeEditPlaneNormal.x * normalOffset;
        const cursorY = projectedCenter.y + this.activeEditPlaneNormal.y * normalOffset;
        const cursorZ = projectedCenter.z + this.activeEditPlaneNormal.z * normalOffset;

        // Calculate corner position for the cursor voxel
        const voxelX = cursorX - halfSize;
        const voxelY = cursorY - halfSize;
        const voxelZ = cursorZ - halfSize;

        // Check if within world bounds
        const halfWorld = getWorldSize(getMacroDepth()) / 2;
        const isInBounds =
          voxelX >= -halfWorld && voxelX + size <= halfWorld &&
          voxelY >= -halfWorld && voxelY + size <= halfWorld &&
          voxelZ >= -halfWorld && voxelZ + size <= halfWorld;

        if (isInBounds) {
          // Store current cursor coordinate
          const newCoord = worldToCube(voxelX, voxelY, voxelZ, this.cursorDepth);

          // Only log when coordinate changes
          if (!this.currentCursorCoord ||
              this.currentCursorCoord.x !== newCoord.x ||
              this.currentCursorCoord.y !== newCoord.y ||
              this.currentCursorCoord.z !== newCoord.z) {
            console.log('[MOUSEMOVE CURSOR] DrawPlane CHANGED:', {
              oldCoord: this.currentCursorCoord,
              newCoord,
              planeHitPoint,
              snappedCenter: { x: snappedCenter.x, y: snappedCenter.y, z: snappedCenter.z },
              projectedCenter: { x: projectedCenter.x, y: projectedCenter.y, z: projectedCenter.z },
              cursorCenter: { x: cursorX, y: cursorY, z: cursorZ },
              voxelCorner: { x: voxelX, y: voxelY, z: voxelZ }
            });
          }

          this.currentCursorCoord = newCoord;

          // Position preview cube at voxel area center
          this.currentGridPosition.set(cursorX, cursorY, cursorZ);
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
          this.previewCube.visible = false;
          this.currentCursorCoord = null;
        }
      } else if (hasGeometryHit && geometryHitPoint && geometryHitNormal) {
        // Position cursor at the voxel area (not face center)
        const size = this.getCursorSize();
        const halfSize = size / 2;

        // Convert adjusted hit point to CubeCoord to snap to voxel grid
        const coord = worldToCube(geometryHitPoint.x, geometryHitPoint.y, geometryHitPoint.z, this.cursorDepth);
        const [voxelX, voxelY, voxelZ] = cubeToWorld(coord);

        // Calculate voxel center
        const voxelCenterX = voxelX + halfSize;
        const voxelCenterY = voxelY + halfSize;
        const voxelCenterZ = voxelZ + halfSize;

        // Calculate face center (same as face highlight)
        const faceCenterX = voxelCenterX + geometryHitNormal.x * halfSize - geometryHitNormal.x;
        const faceCenterY = voxelCenterY + geometryHitNormal.y * halfSize - geometryHitNormal.y;
        const faceCenterZ = voxelCenterZ + geometryHitNormal.z * halfSize - geometryHitNormal.z;

        // Move from face center to voxel area center based on depth select mode
        // Mode 1: voxel on positive normal side (outward from face)
        // Mode 2: voxel on negative normal side (inward from face)
        const normalOffset = this.depthSelectMode === 1 ? halfSize : -halfSize;
        const cursorX = faceCenterX + geometryHitNormal.x * normalOffset;
        const cursorY = faceCenterY + geometryHitNormal.y * normalOffset;
        const cursorZ = faceCenterZ + geometryHitNormal.z * normalOffset;

        // Calculate corner position for the cursor voxel
        const cursorVoxelX = cursorX - halfSize;
        const cursorVoxelY = cursorY - halfSize;
        const cursorVoxelZ = cursorZ - halfSize;

        // Check if within world bounds
        const halfWorld = getWorldSize(getMacroDepth()) / 2;
        const isInBounds =
          cursorVoxelX >= -halfWorld && cursorVoxelX + size <= halfWorld &&
          cursorVoxelY >= -halfWorld && cursorVoxelY + size <= halfWorld &&
          cursorVoxelZ >= -halfWorld && cursorVoxelZ + size <= halfWorld;

        if (isInBounds) {
          // Store cursor coordinate using cursor voxel position
          const newCoord = worldToCube(cursorVoxelX, cursorVoxelY, cursorVoxelZ, this.cursorDepth);

          // Only log when coordinate changes
          if (!this.currentCursorCoord ||
              this.currentCursorCoord.x !== newCoord.x ||
              this.currentCursorCoord.y !== newCoord.y ||
              this.currentCursorCoord.z !== newCoord.z) {
            console.log('[MOUSEMOVE CURSOR] GeometryHit CHANGED:', {
              oldCoord: this.currentCursorCoord,
              newCoord,
              hitPoint: geometryHitPoint,
              hitNormal: geometryHitNormal,
              cursorCenter: { x: cursorX, y: cursorY, z: cursorZ },
              voxelCorner: { x: cursorVoxelX, y: cursorVoxelY, z: cursorVoxelZ }
            });
          }

          this.currentCursorCoord = newCoord;

          // Position preview cube at voxel area center
          this.currentGridPosition.set(cursorX, cursorY, cursorZ);
          this.previewCube.position.copy(this.currentGridPosition);
          this.previewCube.visible = true;

          // Continuous paint: if mouse button is pressed, paint/erase voxel at new position
          if (this.isLeftMousePressed || this.isRightMousePressed) {
            // Check if this is a new voxel position (different from last painted)
            const isNewPosition = !this.lastPaintedVoxel ||
              this.lastPaintedVoxel.x !== cursorVoxelX ||
              this.lastPaintedVoxel.y !== cursorVoxelY ||
              this.lastPaintedVoxel.z !== cursorVoxelZ;

            if (isNewPosition) {
              if (this.isLeftMousePressed) {
                // Left mouse: draw with selected color
                this.paintVoxelWithSize(cursorVoxelX, cursorVoxelY, cursorVoxelZ, size);
              } else if (this.isRightMousePressed) {
                // Right mouse: erase
                this.eraseVoxelWithSize(cursorVoxelX, cursorVoxelY, cursorVoxelZ, size);
              }
              this.lastPaintedVoxel = { x: cursorVoxelX, y: cursorVoxelY, z: cursorVoxelZ };
            }
          }
        } else {
          this.previewCube.visible = false;
          this.currentCursorCoord = null;
        }
      } else {
        // Use active edit plane if available, otherwise ground plane
        const targetPlane = this.activeEditPlane || this.groundPlane;
        const intersectPoint = new THREE.Vector3();
        const didIntersect = this.raycaster.ray.intersectPlane(targetPlane, intersectPoint);

        if (didIntersect) {
          const size = this.getCursorSize();
          const halfSize = size / 2;

          // If we have an active edit plane with a normal, apply SPACE mode
          if (this.activeEditPlane && this.activeEditPlaneNormal) {
            // Snap intersection point to grid
            const voxelCenterX = snapToGrid(intersectPoint.x, size);
            const voxelCenterY = snapToGrid(intersectPoint.y, size);
            const voxelCenterZ = snapToGrid(intersectPoint.z, size);

            // Calculate face position on the plane
            const faceCenterX = voxelCenterX;
            const faceCenterY = voxelCenterY;
            const faceCenterZ = voxelCenterZ;

            // Apply SPACE mode offset
            const normalOffset = this.depthSelectMode === 1 ? halfSize : -halfSize;
            const cursorX = faceCenterX + this.activeEditPlaneNormal.x * normalOffset;
            const cursorY = faceCenterY + this.activeEditPlaneNormal.y * normalOffset;
            const cursorZ = faceCenterZ + this.activeEditPlaneNormal.z * normalOffset;

            // Calculate corner position for the cursor voxel
            const voxelX = cursorX - halfSize;
            const voxelY = cursorY - halfSize;
            const voxelZ = cursorZ - halfSize;

            // Check if within world bounds
            const halfWorld = getWorldSize(getMacroDepth()) / 2;
            const isInBounds =
              voxelX >= -halfWorld && voxelX + size <= halfWorld &&
              voxelY >= -halfWorld && voxelY + size <= halfWorld &&
              voxelZ >= -halfWorld && voxelZ + size <= halfWorld;

            if (isInBounds) {
              // Store current cursor coordinate
              this.currentCursorCoord = worldToCube(voxelX, voxelY, voxelZ, this.cursorDepth);

              // Position preview cube at voxel area center
              this.currentGridPosition.set(cursorX, cursorY, cursorZ);
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
            // Ground plane mode (no active edit plane normal)
            // Snap X and Z axes to grid centered on the intersection point
            const voxelCenterX = snapToGrid(intersectPoint.x, size);
            const voxelCenterZ = snapToGrid(intersectPoint.z, size);

            // Calculate corner position (world space)
            const voxelX = voxelCenterX - halfSize;
            const voxelZ = voxelCenterZ - halfSize;
            // Y position based on depth select mode (not snapped to grid for ground plane)
            const voxelY = this.depthSelectMode === 1 ? 0 : -size;

            // Check if within world bounds (all axes)
            const halfWorld = getWorldSize(getMacroDepth()) / 2;
            const isInBounds =
              voxelX >= -halfWorld && voxelX + size <= halfWorld &&
              voxelY >= -halfWorld && voxelY + size <= halfWorld &&
              voxelZ >= -halfWorld && voxelZ + size <= halfWorld;

            if (isInBounds) {
              // Store cursor coordinate
              const newCoord = worldToCube(voxelX, voxelY, voxelZ, this.cursorDepth);

              // Only log when coordinate changes
              if (!this.currentCursorCoord ||
                  this.currentCursorCoord.x !== newCoord.x ||
                  this.currentCursorCoord.y !== newCoord.y ||
                  this.currentCursorCoord.z !== newCoord.z) {
                console.log('[MOUSEMOVE CURSOR] GroundPlane CHANGED:', {
                  oldCoord: this.currentCursorCoord,
                  newCoord,
                  intersectPoint,
                  voxelCenter: { x: voxelCenterX, z: voxelCenterZ },
                  voxelCorner: { x: voxelX, y: voxelY, z: voxelZ },
                  depthSelectMode: this.depthSelectMode
                });
              }

              this.currentCursorCoord = newCoord;

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
          }
        } else {
          this.previewCube.visible = false;
          this.currentCursorCoord = null;
        }
      }
    });
  }

  updateGeometry(vertices: Float32Array, indices: Uint32Array, normals: Float32Array, colors?: Float32Array): void {
    // Clean up old geometry mesh
    if (this.geometryMesh) {
      this.scene.remove(this.geometryMesh);
      this.geometryMesh.geometry.dispose();
      if (this.geometryMesh.material instanceof THREE.Material) {
        this.geometryMesh.material.dispose();
      }
    }

    // Clean up old wireframe mesh
    if (this.wireframeMesh) {
      this.scene.remove(this.wireframeMesh);
      this.wireframeMesh.geometry.dispose();
      if (this.wireframeMesh.material instanceof THREE.Material) {
        this.wireframeMesh.material.dispose();
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

    // Create solid mesh (always visible)
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

    // Create wireframe overlay mesh
    const wireframeGeometry = new THREE.WireframeGeometry(geometry);
    const wireframeMaterial = new THREE.LineBasicMaterial({
      color: 0x000000,
      linewidth: 1,
      depthTest: true,
      depthWrite: false
    });

    this.wireframeMesh = new THREE.LineSegments(wireframeGeometry, wireframeMaterial);
    this.wireframeMesh.renderOrder = 1; // Render wireframe on top
    this.wireframeMesh.visible = this.wireframeMode;
    this.scene.add(this.wireframeMesh);
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
      logger.error('renderer', 'Avatar engine not initialized');
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

    logger.log('renderer', `Created voxel avatar for ${userNpub}`);
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
      logger.log('renderer', `[Scene] After avatar creation, currentUserPubkey: ${this.currentUserPubkey}`);
      if (this.currentUserPubkey) {
        this.fetchAndApplyProfilePicture(this.currentUserPubkey, this.currentAvatar);
      } else {
        logger.log('renderer', `[Scene] No currentUserPubkey set, skipping profile fetch`);
      }

      logger.log('renderer', `Created voxel avatar from .vox file: ${voxUrl}`);
    } catch (error) {
      logger.error('renderer', 'Failed to load .vox avatar:', error);
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

    logger.log('renderer', 'Created CSM avatar');
  }

  /**
   * Set edit mode to show/hide grid helpers
   */
  setMainMode(mode: MainMode): void {
    const previousMode = this.currentMode;
    this.currentMode = mode;

    // Hide cursors from previous mode
    if (previousMode === 'edit' && this.previewCube) {
      this.previewCube.visible = false;
    }
    if (previousMode === 'placement' && this.placementMode) {
      this.placementMode.hide();
    }

    // Update legacy isEditMode flag for backward compatibility
    this.isEditMode = (mode === 'edit');

    // Reset mouse mode when switching modes
    if (this.mouseMode === 2) {
      this.mouseMode = 1;
      document.exitPointerLock();
      if (this.crosshair) {
        this.crosshair.style.display = 'none';
      }
    }

    // Reset input state
    this.isLeftMousePressed = false;
    this.isRightMousePressed = false;
    this.lastPaintedVoxel = null;
    if (mode !== 'edit') {
      this.clearActiveEditPlane();
    }
  }

  setEditMode(isEditMode: boolean): void {
    // Map to MainMode for backward compatibility
    this.setMainMode(isEditMode ? 'edit' : 'walk');

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
      this.clearActiveEditPlane(); // Clear active edit plane
      this.hideFaceHighlight(); // Hide face highlight
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
   * Load world cube from CSM for WASM raycasting
   */
  async loadWorldCube(csmText: string): Promise<void> {
    try {
      this.worldCube = await loadCubeFromCsm(csmText);
      if (this.worldCube) {
        logger.log('renderer', '[Raycast] World cube loaded for WASM raycasting');
      } else {
        logger.error('renderer', '[Raycast] Failed to load world cube from CSM');
      }
    } catch (error) {
      logger.error('renderer', '[Raycast] Failed to load world cube:', error);
    }
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
    logger.log('renderer', `[Scene] setCurrentUserPubkey called: ${pubkey}`);
    this.currentUserPubkey = pubkey;

    // If we have an avatar already and now have a pubkey, fetch profile retroactively
    if (pubkey && this.currentAvatar) {
      logger.log('renderer', `[Scene] Avatar exists, fetching profile retroactively`);
      this.fetchAndApplyProfilePicture(pubkey, this.currentAvatar);
    }
  }

  /**
   * Refresh profile picture for current avatar (call after avatar is loaded)
   */
  refreshCurrentAvatarProfile(): void {
    logger.log('renderer', `[Scene] refreshCurrentAvatarProfile called, pubkey: ${this.currentUserPubkey}, hasAvatar: ${!!this.currentAvatar}`);
    if (this.currentUserPubkey && this.currentAvatar) {
      logger.log('renderer', `[Scene] Refreshing profile for current avatar`);
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
        logger.log('renderer', `Removed remote avatar for ${pubkey}`);
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
        logger.log('renderer', `[Scene] Model changed for ${state.npub}:`, {
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
          logger.log('renderer', `Recreating remote avatar for ${state.npub} due to model change`);
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

    logger.log('renderer', `[Scene] Creating remote avatar for ${npub}:`, { avatarType, avatarId, avatarUrl, avatarDataLength: avatarData?.length });

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
        logger.log('renderer', `[Scene] Loading VOX model from avatarId: ${avatarId}`);
        // Load from .vox file using model config
        import('../utils/modelConfig').then(({ getModelUrl }) => {
          const voxUrl = getModelUrl(avatarId, 'vox');

          if (!voxUrl) {
            logger.warn('renderer', `No model found for avatarId: ${avatarId}, using generated`);
            const geometryData = this.avatarEngine!.generate_avatar(npub);
            voxelAvatar.applyGeometry(geometryData);
            return;
          }

          import('../utils/voxLoader').then(({ loadVoxFromUrl }) => {
            // Pass undefined to preserve original colors
            loadVoxFromUrl(voxUrl, undefined).then((geometryData) => {
              voxelAvatar.applyGeometry(geometryData);
            }).catch(error => {
              logger.error('renderer', 'Failed to load .vox avatar for remote user:', error);
              // Fallback to generated
              const geometryData = this.avatarEngine!.generate_avatar(npub);
              voxelAvatar.applyGeometry(geometryData);
            });
          }).catch(err => logger.error('renderer', err));
        }).catch(err => logger.error('renderer', err));
      } else {
        // Use procedurally generated model
        logger.log('renderer', `[Scene] No avatarId for ${npub}, using procedurally generated avatar`);
        const geometryData = this.avatarEngine.generate_avatar(npub);
        voxelAvatar.applyGeometry(geometryData);
      }

      // Add to scene
      this.scene.add(voxelAvatar.getObject3D());
      this.remoteAvatars.set(pubkey, voxelAvatar);
      this.remoteAvatarConfigs.set(pubkey, { avatarType, avatarId, avatarData });

      // Fetch and apply profile picture
      this.fetchAndApplyProfilePicture(pubkey, voxelAvatar, npub);

      logger.log('renderer', `Created remote voxel avatar for ${npub}`);
    } else if (avatarType === 'csm') {
      // Create CSM avatar
      if (avatarData) {
        logger.log('renderer', `Creating remote CSM avatar for ${npub}`);

        // Parse CSM code to mesh
        import('../utils/cubeWasm').then(({ parseCsmToMesh }) => {
          parseCsmToMesh(avatarData).then((result) => {
            if ('error' in result) {
              logger.error('renderer', `Failed to parse CSM for remote avatar ${npub}:`, result.error);
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

            logger.log('renderer', `Created remote CSM avatar for ${npub}`);
          }).catch(error => {
            logger.error('renderer', `Failed to load CSM avatar for remote user ${npub}:`, error);
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
        }).catch(err => logger.error('renderer', err));
      } else {
        logger.warn('renderer', `No avatarData provided for remote CSM avatar ${npub}, using generated`);
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

      logger.log('renderer', `Created remote GLB avatar for ${npub}`);
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
    logger.log('renderer', `[Scene] fetchAndApplyProfilePicture called for pubkey: ${pubkey}, npub: ${npub}`);
    try {
      const profile = await profileCache.getProfile(pubkey, DEFAULT_RELAYS);
      logger.log('renderer', `[Scene] Profile fetched:`, profile);

      // Set display name for initials fallback
      const displayName = profile?.display_name || profile?.name || npub?.slice(0, 12) || '';
      logger.log('renderer', `[Scene] Setting display name: ${displayName}`);
      avatar.setDisplayName(displayName);

      // Set profile picture if available
      if (profile?.picture) {
        logger.log('renderer', `[Scene] Setting profile picture: ${profile.picture}`);
        await avatar.setProfilePicture(profile.picture);
      } else {
        logger.log('renderer', `[Scene] No profile picture available`);
      }
    } catch (error) {
      logger.warn('renderer', `[Scene] Failed to fetch profile picture for ${pubkey}:`, error);
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

    const worldSize = getWorldSize(getMacroDepth());

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
      timeOfDay: this.sunSystem?.getTimeOfDay(),
      placementModel: this.placementMode?.getBrush().modelPath,
      placementScale: this.placementMode?.getScale()
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
   * Set wireframe mode for the ground geometry mesh
   */
  setWireframe(enabled: boolean): void {
    this.wireframeMode = enabled;
    if (this.wireframeMesh) {
      this.wireframeMesh.visible = enabled;
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

    // Clear active edit plane
    this.clearActiveEditPlane();

    // Clean up face highlight
    if (this.faceHighlightMesh) {
      this.scene.remove(this.faceHighlightMesh);
      this.faceHighlightMesh.geometry.dispose();
      if (this.faceHighlightMesh.material instanceof THREE.Material) {
        this.faceHighlightMesh.material.dispose();
      }
      this.faceHighlightMesh = null;
    }

    logger.log('renderer', '[SceneManager] Disposed');
  }
}
