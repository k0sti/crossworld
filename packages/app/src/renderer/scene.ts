import * as THREE from 'three';
import { Avatar } from './avatar';
import { VoxelAvatar } from './voxel-avatar';
import type { IAvatar } from './base-avatar';
import type { AvatarState } from '../services/avatar-state';
import { Transform } from './transform';
import type { TeleportAnimationType } from './teleport-animation';
import { CameraController } from './camera-controller';
import { GamepadController } from './gamepad-controller';
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
import {
  getMacroDepth,
  getBorderDepth,
  cursorDepthToAbsolute,
  getMinCursorDepth,
  getMaxCursorDepth
} from '../config/depth-config';
import { CheckerPlane } from './checker-plane';
import { loadCubeFromCsm, raycastWasm } from '../utils/cubeWasm';
import { raycastMesh, type MeshRaycastResult } from '../utils/meshRaycast';
import { raycastWorld, calculateAvatarPlacement } from '../utils/worldRaycast';
import { SunSystem } from './sun-system';
import { PostProcessing } from './post-processing';
import { profileCache } from '../services/profile-cache';
import { DEFAULT_RELAYS } from '../config';
import * as logger from '../utils/logger';
import { MaterialsLoader } from './materials-loader';
import { createTexturedVoxelMaterial, updateShaderLighting } from './textured-voxel-material';
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
  private texturedMesh: THREE.Mesh | null = null;
  private solidColorMesh: THREE.Mesh | null = null;
  private wireframeMesh: THREE.LineSegments | null = null;
  private checkerPlane: CheckerPlane | null = null;
  private groundPlane: THREE.Plane = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0); // Plane at y=0
  private currentAvatar: IAvatar | null = null;
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
  // Cursor depth is now relative to base depth (macro + border)
  // cursorDepth = 0 means unit cubes (1x1x1 world units) at base depth
  // cursorDepth < 0 means larger voxels (e.g., -1 = 2x2x2 units)
  // cursorDepth > 0 means smaller subdivisions (up to micro_depth)
  // initialized to 0 (unit cubes)
  private cursorDepth: number = 0;

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

  // Gamepad controller for avatar movement
  private gamepadController: GamepadController | null = null;

  // Event listener references for cleanup
  private boundKeyDown?: (event: KeyboardEvent) => void;
  private boundKeyUp?: (event: KeyboardEvent) => void;
  private boundPointerLockChange?: () => void;

  // Materials and textures
  private materialsLoader: MaterialsLoader = new MaterialsLoader();
  private texturesLoaded: boolean = false;
  private texturesEnabled: boolean = true;
  private avatarTexturesEnabled: boolean = true;
  private texturesLoadingPromise: Promise<void> | null = null;

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
    // Request WebGL 2.0 context for texture array support
    const gl = canvas.getContext('webgl2');
    if (!gl) {
      logger.warn('renderer', 'WebGL 2.0 not available, falling back to WebGL 1.0');
    } else {
      logger.log('renderer', 'Using WebGL 2.0 context');
    }

    this.renderer = new THREE.WebGLRenderer({
      canvas,
      antialias: true,
      alpha: true,
      context: gl || undefined
    });
    this.renderer.setSize(window.innerWidth, window.innerHeight);
    this.renderer.setPixelRatio(window.devicePixelRatio);
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.0;

    // Set fixed camera position for isometric-like view (centered at origin)
    // For 128-unit world (depth 7), position camera to see the whole world
    this.camera.position.set(10, 8, 10);
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

    // Initialize gamepad controller for avatar movement
    this.gamepadController = new GamepadController();
    this.setupOriginHelpers();
    this.setupCrosshair();
    this.setupCheckerPlane();

    // Load materials and textures asynchronously
    logger.log('renderer', '[Scene] Starting material and texture loading...');
    this.loadMaterialsAndTextures().catch(error => {
      logger.error('renderer', 'Failed to load materials/textures:', error);
    });

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
    // Create checker plane (2^(macroDepth+borderDepth) × 2^(macroDepth+borderDepth) centered at origin)
    const checkerSize = 1 << (getMacroDepth() + getBorderDepth());
    this.checkerPlane = new CheckerPlane(checkerSize, checkerSize, 0.02);
    const checkerMesh = this.checkerPlane.getMesh();
    this.scene.add(checkerMesh);
    this.worldGridHelpers.push(checkerMesh);
  }

  /**
   * Load materials.json and textures for textured voxels
   * Uses promise caching to prevent multiple concurrent loads
   */
  private async loadMaterialsAndTextures(): Promise<void> {
    // If already loaded, return immediately
    if (this.texturesLoaded) {
      return;
    }

    // If currently loading, wait for existing promise
    if (this.texturesLoadingPromise) {
      logger.log('renderer', 'Textures already loading, waiting for completion...');
      return this.texturesLoadingPromise;
    }

    // Start loading and cache the promise
    this.texturesLoadingPromise = (async () => {
      try {
        logger.log('renderer', 'Loading materials.json...');
        await this.materialsLoader.loadMaterialsJson();

        logger.log('renderer', 'Loading textures...');
        await this.materialsLoader.loadTextures();

        this.texturesLoaded = true;
        logger.log('renderer', 'Materials and textures loaded successfully');

        // Update geometry mesh with textures if it already exists
        if (this.geometryMesh) {
          logger.log('renderer', 'Updating existing geometry mesh with textures');
          // Trigger a re-render with textures by updating the material
          this.updateGeometryMaterial();
        }
      } catch (error) {
        logger.error('renderer', 'Failed to load materials and textures:', error);
        this.texturesLoaded = false;
        this.texturesLoadingPromise = null; // Allow retry
        throw error;
      }
    })();

    return this.texturesLoadingPromise;
  }

  /**
   * Update the geometry mesh material to use textures
   */
  private updateGeometryMaterial(): void {
    if (!this.geometryMesh || !this.texturesLoaded) return;

    const oldMaterial = this.geometryMesh.material;

    // Create new textured material
    const textureArray = this.materialsLoader.getTextureArray();
    const newMaterial = createTexturedVoxelMaterial(textureArray, this.texturesEnabled);

    // Update shader lighting based on current scene lights
    updateShaderLighting(newMaterial, this.scene);

    // Replace material
    this.geometryMesh.material = newMaterial;

    // Dispose old material
    if (oldMaterial instanceof THREE.Material) {
      oldMaterial.dispose();
    }

    logger.log('renderer', 'Updated geometry material with textures');
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
      this.calculateMousePosition(event, canvas);

      // Update raycaster
      this.raycaster.setFromCamera(this.mouse, this.camera);

      // Check modifiers: CTRL for teleport, SHIFT for run
      const useTeleport = event.ctrlKey;
      const useRun = event.shiftKey && !useTeleport;

      let targetX: number;
      let targetZ: number;

      // For teleport, use world raycast to find voxel face
      if (useTeleport && this.geometryMesh) {
        const worldHit = raycastWorld(this.geometryMesh, this.raycaster);

        if (worldHit) {
          // Calculate avatar placement on the voxel face
          const placement = calculateAvatarPlacement(worldHit);
          targetX = placement.x;
          targetZ = placement.z;

          this.currentAvatar.teleportTo(targetX, targetZ, this.teleportAnimationType);
          this.currentMoveStyle = `teleport:${this.teleportAnimationType}`;
          // Publish TARGET position with move style
          this.publishPlayerPositionAt(targetX, targetZ, this.currentMoveStyle);
        }
      } else {
        // For walk/run, use ground plane raycast
        const intersectPoint = new THREE.Vector3();
        const didIntersect = this.raycaster.ray.intersectPlane(this.groundPlane, intersectPoint);

        if (didIntersect) {
          // Use exact raycast coordinates or snap to grid based on flag
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

          // Move avatar
          if (useRun) {
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

        const worldSize = getWorldSize(getMacroDepth(), getBorderDepth());
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
        const worldSize = getWorldSize(getMacroDepth(), getBorderDepth());
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
    this.calculateMousePosition(event, canvas);

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
    const coord = worldToCube(point.x, point.y, point.z, this.getAbsoluteCursorDepth());
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
    const worldSize = getWorldSize(getMacroDepth(), getBorderDepth());
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
    const coord = worldToCube(point.x, point.y, point.z, this.getAbsoluteCursorDepth());

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
    this.calculateMousePosition(event, canvas);

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
      const coord = worldToCube(hit.point.x, hit.point.y, hit.point.z, this.getAbsoluteCursorDepth());
      const [hitVoxelX, hitVoxelY, hitVoxelZ] = cubeToWorld(coord);

      // Calculate voxel center of the hit voxel
      const hitVoxelCorner = new THREE.Vector3(hitVoxelX, hitVoxelY, hitVoxelZ);
      const voxelCenter = this.calculateVoxelCenter(hitVoxelCorner, halfSize);

      // Calculate face center
      const faceCenter = this.calculateFaceCenter(voxelCenter, hit.normal, halfSize);

      // Move from face center to voxel area center based on depth select mode
      // Mode 1: voxel on positive normal side (outward from face)
      // Mode 2: voxel on negative normal side (inward from face)
      const normalOffset = this.depthSelectMode === 1 ? halfSize : -halfSize;
      const cursorCenter = faceCenter.clone().addScaledVector(hit.normal, normalOffset);

      // Calculate corner position for the cursor voxel (as Vector3)
      const voxelCorner = cursorCenter.clone().subScalar(halfSize);
      voxelX = voxelCorner.x;
      voxelY = voxelCorner.y;
      voxelZ = voxelCorner.z;

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

    const voxelCorner = new THREE.Vector3(voxelX, voxelY, voxelZ);
    logger.log('renderer', '[Voxel Pos]', { voxelCorner, size });

    // Check if within world bounds (all axes)
    if (this.isVoxelInBounds(voxelCorner, size)) {
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
   * Get the current cursor depth (relative to base depth)
   */
  getCursorDepth(): number {
    return this.cursorDepth;
  }

  /**
   * Get the absolute octree depth for the current cursor
   * Converts relative cursor depth to absolute octree depth
   */
  private getAbsoluteCursorDepth(): number {
    return cursorDepthToAbsolute(this.cursorDepth);
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
    const coord = worldToCube(x, y, z, this.getAbsoluteCursorDepth());

    logger.log('renderer', '[Paint -> CubeCoord]', { coord, colorValue, hasCallback: !!this.onVoxelEdit });

    // Call onVoxelEdit with CubeCoord
    this.onVoxelEdit?.(coord, colorValue);
  }

  private eraseVoxelWithSize(x: number, y: number, z: number, size: number): void {
    logger.log('renderer', '[Erase Voxel]', { x, y, z, size });

    // Convert world coordinates to cube coordinates
    const coord = worldToCube(x, y, z, this.getAbsoluteCursorDepth());

    logger.log('renderer', '[Erase -> CubeCoord]', { coord, hasCallback: !!this.onVoxelEdit });

    // Call onVoxelEdit with CubeCoord
    this.onVoxelEdit?.(coord, 0);
  }

  private setupKeyboardListener(canvas: HTMLCanvasElement): void {
    this.boundKeyDown = (event: KeyboardEvent) => {
      // WASD movement removed - use gamepad left stick instead

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
          this.resetMouseMode();
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
        this.adjustCursorDepth(1);
      }

      if (event.code === 'ArrowDown') {
        event.preventDefault();
        this.adjustCursorDepth(-1);
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

      // WASD movement removed - use gamepad left stick instead
    };

    this.boundPointerLockChange = () => {
      if (!document.pointerLockElement && this.mouseMode === 2) {
        // Pointer lock was exited externally (e.g., Escape key), sync mode back to 1
        this.resetMouseMode();
        logger.log('renderer', '[Mouse Mode] Pointer lock exited (Escape), switched to mode 1');
      }
    };

    // Register event listeners
    window.addEventListener('keydown', this.boundKeyDown);
    window.addEventListener('keyup', this.boundKeyUp);
    document.addEventListener('pointerlockchange', this.boundPointerLockChange);
  }

  private resetAvatarMovementState(): void {
    // WASD movement removed - gamepad state is managed by GamepadController
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
    const worldSize = getWorldSize(getMacroDepth(), getBorderDepth());
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
    return getVoxelSizeFromCubeCoord(this.getAbsoluteCursorDepth());
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
      this.currentCursorCoord = worldToCube(voxelX, voxelY, voxelZ, this.getAbsoluteCursorDepth());

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
    if (!this.isEditMode) {
      this.hideFaceHighlight();
      return;
    }

    // Use provided mouse position or current mouse position
    if (mouseX !== undefined && mouseY !== undefined) {
      this.mouse.x = mouseX;
      this.mouse.y = mouseY;
    }

    // Update raycaster and perform unified cursor update
    this.raycaster.setFromCamera(this.mouse, this.camera);
    this.raycastAndUpdateCursor();
  }


  private updateVoxelCursorAtCenter(): void {
    // Raycast from center of screen
    this.mouse.x = 0;
    this.mouse.y = 0;
    this.raycaster.setFromCamera(this.mouse, this.camera);
    this.raycastAndUpdateCursor();
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
        const cursorCenter = projectedCenter.clone().addScaledVector(this.activeEditPlaneNormal, normalOffset);

        // Calculate corner position for the cursor voxel
        const voxelCorner = cursorCenter.clone().subScalar(halfSize);

        // Check if within world bounds
        if (this.isVoxelInBounds(voxelCorner, size)) {
          // Store current cursor coordinate
          const newCoord = worldToCube(voxelCorner.x, voxelCorner.y, voxelCorner.z, this.getAbsoluteCursorDepth());

          this.currentCursorCoord = newCoord;

          // Position preview cube at voxel area center
          this.currentGridPosition.copy(cursorCenter);
          this.previewCube.position.copy(this.currentGridPosition);
          this.previewCube.visible = true;

          // Continuous paint: if mouse button is pressed, paint/erase voxel at new position
          this.handleContinuousPaint(voxelCorner, size);
        } else {
          this.previewCube.visible = false;
          this.currentCursorCoord = null;
        }
      } else if (hasGeometryHit && geometryHitPoint && geometryHitNormal) {
        // Position cursor at the voxel area (not face center)
        const size = this.getCursorSize();
        const halfSize = size / 2;

        // Convert adjusted hit point to CubeCoord to snap to voxel grid
        const coord = worldToCube(geometryHitPoint.x, geometryHitPoint.y, geometryHitPoint.z, this.getAbsoluteCursorDepth());
        const [voxelX, voxelY, voxelZ] = cubeToWorld(coord);

        // Calculate voxel center
        const voxelCorner = new THREE.Vector3(voxelX, voxelY, voxelZ);
        const voxelCenter = this.calculateVoxelCenter(voxelCorner, halfSize);

        // Calculate face center (same as face highlight)
        const faceCenter = this.calculateFaceCenter(voxelCenter, geometryHitNormal, halfSize);

        // Move from face center to voxel area center based on depth select mode
        // Mode 1: voxel on positive normal side (outward from face)
        // Mode 2: voxel on negative normal side (inward from face)
        const normalOffset = this.depthSelectMode === 1 ? halfSize : -halfSize;
        const cursorCenter = faceCenter.clone().addScaledVector(geometryHitNormal, normalOffset);

        // Calculate corner position for the cursor voxel
        const cursorVoxelCorner = cursorCenter.clone().subScalar(halfSize);

        // Check if within world bounds
        if (this.isVoxelInBounds(cursorVoxelCorner, size)) {
          // Store cursor coordinate using cursor voxel position
          const newCoord = worldToCube(cursorVoxelCorner.x, cursorVoxelCorner.y, cursorVoxelCorner.z, this.getAbsoluteCursorDepth());

          // Only log when coordinate changes
          if (!this.currentCursorCoord ||
              this.currentCursorCoord.x !== newCoord.x ||
              this.currentCursorCoord.y !== newCoord.y ||
              this.currentCursorCoord.z !== newCoord.z) {
          }

          this.currentCursorCoord = newCoord;

          // Position preview cube at voxel area center
          this.currentGridPosition.copy(cursorCenter);
          this.previewCube.position.copy(this.currentGridPosition);
          this.previewCube.visible = true;

          // Continuous paint: if mouse button is pressed, paint/erase voxel at new position
          this.handleContinuousPaint(cursorVoxelCorner, size);
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
            const faceCenter = new THREE.Vector3(
              snapToGrid(intersectPoint.x, size),
              snapToGrid(intersectPoint.y, size),
              snapToGrid(intersectPoint.z, size)
            );

            // Apply SPACE mode offset
            const normalOffset = this.depthSelectMode === 1 ? halfSize : -halfSize;
            const cursorCenter = faceCenter.clone().addScaledVector(this.activeEditPlaneNormal, normalOffset);

            // Calculate corner position for the cursor voxel
            const voxelCorner = cursorCenter.clone().subScalar(halfSize);

            // Check if within world bounds
            if (this.isVoxelInBounds(voxelCorner, size)) {
              // Store current cursor coordinate
              this.currentCursorCoord = worldToCube(voxelCorner.x, voxelCorner.y, voxelCorner.z, this.getAbsoluteCursorDepth());

              // Position preview cube at voxel area center
              this.currentGridPosition.copy(cursorCenter);
              this.previewCube.position.copy(this.currentGridPosition);
              this.previewCube.visible = true;

              // Continuous paint: if mouse button is pressed, paint/erase voxel at new position
              this.handleContinuousPaint(voxelCorner, size);
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
            // Y position based on depth select mode (not snapped to grid for ground plane)
            const voxelY = this.depthSelectMode === 1 ? 0 : -size;
            const voxelCorner = new THREE.Vector3(
              voxelCenterX - halfSize,
              voxelY,
              voxelCenterZ - halfSize
            );

            // Check if within world bounds (all axes)
            if (this.isVoxelInBounds(voxelCorner, size)) {
              // Store cursor coordinate
              const newCoord = worldToCube(voxelCorner.x, voxelCorner.y, voxelCorner.z, this.getAbsoluteCursorDepth());

              // Only log when coordinate changes
              if (!this.currentCursorCoord ||
                  this.currentCursorCoord.x !== newCoord.x ||
                  this.currentCursorCoord.y !== newCoord.y ||
                  this.currentCursorCoord.z !== newCoord.z) {
              }

              this.currentCursorCoord = newCoord;

              // Position preview cube at center of voxel (world space)
              this.currentGridPosition.set(voxelCenterX, voxelY + halfSize, voxelCenterZ);
              this.previewCube.position.copy(this.currentGridPosition);
              this.previewCube.visible = true;

              // Continuous paint: if mouse button is pressed, paint/erase voxel at new position
              this.handleContinuousPaint(voxelCorner, size);
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

  updateGeometry(vertices: Float32Array, indices: Uint32Array, normals: Float32Array, colors?: Float32Array, uvs?: Float32Array, materialIds?: Uint8Array): void {
    logger.log('renderer', '=== updateGeometry called ===');
    logger.log('renderer', `Vertices: ${vertices.length}, Indices: ${indices.length}, Normals: ${normals.length}`);
    logger.log('renderer', `Colors: ${colors?.length || 0}, UVs: ${uvs?.length || 0}, MaterialIds: ${materialIds?.length || 0}`);
    logger.log('renderer', `Textures loaded: ${this.texturesLoaded}`);

    // Clean up old geometry mesh (legacy - kept for backwards compatibility)
    if (this.geometryMesh) {
      this.scene.remove(this.geometryMesh);
      this.geometryMesh.geometry.dispose();
      if (this.geometryMesh.material instanceof THREE.Material) {
        this.geometryMesh.material.dispose();
      }
      this.geometryMesh = null;
    }

    // Clean up old textured mesh
    if (this.texturedMesh) {
      this.scene.remove(this.texturedMesh);
      this.texturedMesh.geometry.dispose();
      if (this.texturedMesh.material instanceof THREE.Material) {
        this.texturedMesh.material.dispose();
      }
      this.texturedMesh = null;
    }

    // Clean up old solid color mesh
    if (this.solidColorMesh) {
      this.scene.remove(this.solidColorMesh);
      this.solidColorMesh.geometry.dispose();
      if (this.solidColorMesh.material instanceof THREE.Material) {
        this.solidColorMesh.material.dispose();
      }
      this.solidColorMesh = null;
    }

    // Clean up old wireframe mesh
    if (this.wireframeMesh) {
      this.scene.remove(this.wireframeMesh);
      this.wireframeMesh.geometry.dispose();
      if (this.wireframeMesh.material instanceof THREE.Material) {
        this.wireframeMesh.material.dispose();
      }
      this.wireframeMesh = null;
    }

    // Split indices into textured (2-127) and solid color (0-1, 128-255) materials
    const texturedIndices: number[] = [];
    const solidIndices: number[] = [];
    const materialIdStats = new Map<number, number>(); // Track material ID distribution

    if (materialIds && materialIds.length > 0) {
      // Process each triangle (3 indices at a time)
      for (let i = 0; i < indices.length; i += 3) {
        const idx0 = indices[i];
        const idx1 = indices[i + 1];
        const idx2 = indices[i + 2];

        // Check material ID of first vertex of the triangle
        const matId = materialIds[idx0];

        // Track material ID usage
        materialIdStats.set(matId, (materialIdStats.get(matId) || 0) + 1);

        if (matId >= 2 && matId <= 127) {
          // Textured material
          texturedIndices.push(idx0, idx1, idx2);
        } else {
          // Solid color material (0-1, 128-255)
          solidIndices.push(idx0, idx1, idx2);
        }
      }

      logger.log('renderer', `Material ID distribution: ${JSON.stringify(Array.from(materialIdStats.entries()).slice(0, 10))}`);
    } else {
      // No material IDs, treat all as solid color
      solidIndices.push(...indices);
      logger.log('renderer', 'No materialIds provided, treating all as solid color');
    }

    logger.log('renderer', `Split results: ${texturedIndices.length / 3} textured triangles, ${solidIndices.length / 3} solid triangles`);

    // Create textured mesh if there are textured triangles
    if (texturedIndices.length > 0 && this.texturesLoaded) {
      const texturedGeometry = new THREE.BufferGeometry();
      texturedGeometry.setAttribute('position', new THREE.BufferAttribute(vertices, 3));
      texturedGeometry.setAttribute('normal', new THREE.BufferAttribute(normals, 3));

      if (colors && colors.length > 0) {
        texturedGeometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));
      }

      if (uvs && uvs.length > 0) {
        texturedGeometry.setAttribute('uv', new THREE.BufferAttribute(uvs, 2));
      }

      if (materialIds && materialIds.length > 0) {
        texturedGeometry.setAttribute('materialId', new THREE.BufferAttribute(materialIds, 1));
      }

      texturedGeometry.setIndex(new THREE.BufferAttribute(new Uint32Array(texturedIndices), 1));

      const textureArray = this.materialsLoader.getTextureArray();
      logger.log('renderer', `Texture array length: ${textureArray.length}`);

      const texturedMaterial = createTexturedVoxelMaterial(textureArray, this.texturesEnabled, this.renderer);
      updateShaderLighting(texturedMaterial as THREE.ShaderMaterial, this.scene);

      // Debug shader uniforms
      if (texturedMaterial instanceof THREE.ShaderMaterial) {
        logger.log('renderer', `Shader uniforms: enableTextures=${texturedMaterial.uniforms.enableTextures?.value}`);
        logger.log('renderer', `Texture uniforms: texture2=${!!texturedMaterial.uniforms.texture2}, texture3=${!!texturedMaterial.uniforms.texture3}`);
      }

      this.texturedMesh = new THREE.Mesh(texturedGeometry, texturedMaterial);
      this.texturedMesh.castShadow = true;
      this.texturedMesh.receiveShadow = true;
      this.texturedMesh.renderOrder = 0;
      this.scene.add(this.texturedMesh);

      logger.log('renderer', `Created textured mesh with ${texturedIndices.length / 3} triangles`);
    }

    // Create solid color mesh if there are solid color triangles
    if (solidIndices.length > 0) {
      const solidGeometry = new THREE.BufferGeometry();
      solidGeometry.setAttribute('position', new THREE.BufferAttribute(vertices, 3));
      solidGeometry.setAttribute('normal', new THREE.BufferAttribute(normals, 3));

      if (colors && colors.length > 0) {
        solidGeometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));
      }

      solidGeometry.setIndex(new THREE.BufferAttribute(new Uint32Array(solidIndices), 1));

      const solidMaterial = new THREE.MeshPhongMaterial({
        vertexColors: colors && colors.length > 0,
        color: colors && colors.length > 0 ? 0xffffff : 0x44aa44,
        specular: 0x333333,
        shininess: 15,
        wireframe: false,
        side: THREE.FrontSide,
        flatShading: false
      });

      this.solidColorMesh = new THREE.Mesh(solidGeometry, solidMaterial);
      this.solidColorMesh.castShadow = true;
      this.solidColorMesh.receiveShadow = true;
      this.solidColorMesh.renderOrder = 0;
      this.scene.add(this.solidColorMesh);

      logger.log('renderer', `Created solid color mesh with ${solidIndices.length / 3} triangles`);
    }

    // For backwards compatibility, set geometryMesh to the primary mesh (prefer textured if available)
    this.geometryMesh = this.texturedMesh || this.solidColorMesh;

    // Update raycast mesh for avatar ground detection
    if (this.currentAvatar && this.geometryMesh) {
      this.currentAvatar.setRaycastMesh(this.geometryMesh);
    }

    // Create wireframe overlay mesh from the combined geometry
    if (this.geometryMesh) {
      const wireframeGeometry = new THREE.WireframeGeometry(this.geometryMesh.geometry);
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

    // Gamepad movement for avatar (left stick controls direction)
    if (this.currentAvatar && this.gamepadController && !this.isEditMode) {
      // Update gamepad state
      this.gamepadController.update();

      // Get movement input from left stick
      const moveInput = this.gamepadController.getMoveDirection();
      const hasMovement = moveInput.length() > 0;

      if (hasMovement) {
        // Get camera's forward direction (projected on XZ plane)
        const cameraDirection = new THREE.Vector3();
        this.camera.getWorldDirection(cameraDirection);
        const forward = new THREE.Vector3(cameraDirection.x, 0, cameraDirection.z).normalize();

        // Get right direction (perpendicular to forward)
        const right = new THREE.Vector3(-forward.z, 0, forward.x).normalize();

        // Calculate movement direction based on gamepad stick input
        // X axis: left/right, Y axis: forward/backward
        const moveDirection = new THREE.Vector3(0, 0, 0);
        moveDirection.add(forward.multiplyScalar(moveInput.y)); // Forward/backward
        moveDirection.add(right.multiplyScalar(moveInput.x)); // Left/right

        // Normalize and scale by speed
        if (moveDirection.length() > 0) {
          moveDirection.normalize();

          // Check if running (RT trigger pressed)
          const isRunning = this.gamepadController.isRunPressed();
          const baseSpeed = 5.0; // units per second
          const speed = isRunning ? baseSpeed * 2.0 : baseSpeed;
          const distance = speed * deltaTime_s;

          // Get current avatar position
          const currentPos = this.currentAvatar.getPosition();

          // Calculate new position
          const newX = currentPos.x + moveDirection.x * distance;
          const newZ = currentPos.z + moveDirection.z * distance;

          // Check if within world bounds
          if (isWithinWorldBounds(newX, newZ, 0)) {
            // Move avatar to new position
            this.currentAvatar.setRunSpeed(isRunning);
            this.currentAvatar.setTargetPosition(newX, newZ);
            this.currentMoveStyle = isRunning ? 'run' : 'walk';

            // Publish position update
            this.publishPlayerPositionAt(newX, newZ, this.currentMoveStyle);
          }
        }
      }

      // Handle jump button (A button)
      if (this.gamepadController.wasJumpPressed()) {
        // TODO: Implement jump functionality
        console.log('[Scene] Jump button pressed!');
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

  createAvatar(modelUrl?: string, scale?: number, transform?: Transform, renderScaleDepth?: number): void {
    // Remove existing avatar
    if (this.currentAvatar) {
      this.scene.remove(this.currentAvatar.getObject3D());
      this.currentAvatar.dispose();
    }

    // Create new GLB avatar
    this.currentAvatar = new Avatar(transform, { modelUrl, scale, renderScaleDepth }, this.scene);
    this.scene.add(this.currentAvatar.getObject3D());

    // Set raycast mesh for ground detection
    if (this.geometryMesh) {
      this.currentAvatar.setRaycastMesh(this.geometryMesh);
    }

    // Fetch and apply profile picture for current user
    this.applyCurrentUserProfile(this.currentAvatar);
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
   * Set callback for position updates
   */
  setPositionUpdateCallback(callback: (x: number, y: number, z: number, quaternion: [number, number, number, number], moveStyle?: string) => void): void {
    this.onPositionUpdate = callback;
  }

  /**
   * Create a voxel avatar for a user
   * NOTE: Procedural avatar generation removed. Use .vox files instead.
   */
  createVoxelAvatar(_userNpub: string, _scale: number = 1.0, _transform?: Transform): void {
    logger.warn('renderer', 'Procedural avatar generation removed. Use .vox files instead.');
  }

  /**
   * Create a voxel avatar from a .vox file
   */
  async createVoxelAvatarFromVoxFile(voxUrl: string, userNpub: string | undefined = undefined, scale: number = 1.0, transform?: Transform, renderScaleDepth?: number, textureName?: string): Promise<void> {
    // Import the loadVoxFromUrl function
    const { loadVoxFromUrl } = await import('../utils/voxLoader');

    try {
      // Wait for textures to load if texture is requested
      if (textureName && textureName !== '0' && !this.texturesLoaded) {
        logger.log('renderer', `[Avatar] Waiting for textures to load before creating avatar with texture '${textureName}'...`);
        await this.loadMaterialsAndTextures();
      }

      // Load .vox file and get geometry (pass undefined for original colors)
      const geometryData = await loadVoxFromUrl(voxUrl, userNpub ?? undefined);

      // Remove existing avatar
      if (this.currentAvatar) {
        this.scene.remove(this.currentAvatar.getObject3D());
        this.currentAvatar.dispose();
      }

      // Get material ID if specified
      let materialId: number | undefined = undefined;
      if (textureName && textureName !== '0') {
        logger.log('renderer', `[Avatar] Loading texture: ${textureName}`);
        const materials = this.materialsLoader['materialsData'];

        if (materials) {
          const material = materials.materials.find(m => m.id === textureName);

          if (material) {
            materialId = material.index;
            logger.log('renderer', `[Avatar] Using material ID ${materialId} for texture '${textureName}'`);
          } else {
            logger.warn('renderer', `[Avatar] Texture '${textureName}' not found in materials`);
          }
        } else {
          logger.warn('renderer', `[Avatar] Materials data not loaded yet - this should not happen after await`);
        }
      } else {
        logger.log('renderer', `[Avatar] No texture specified (using vertex colors only)`);
      }

      // Create new voxel avatar with shared texture system
      const voxelAvatar = new VoxelAvatar({
        userNpub: userNpub ?? '',
        scale,
        renderScaleDepth,
        materialId,
        textures: this.materialsLoader.getTextureArray(),
        enableTextures: this.avatarTexturesEnabled,
        renderer: this.renderer,
        scene: this.scene,
      }, transform, this.scene);

      // Apply geometry from .vox file
      voxelAvatar.applyGeometry(geometryData);

      // Add to scene
      this.scene.add(voxelAvatar.getObject3D());
      this.currentAvatar = voxelAvatar;

      // Fetch and apply profile picture for current user
      logger.log('renderer', `[Scene] After avatar creation, currentUserPubkey: ${this.currentUserPubkey}`);
      this.applyCurrentUserProfile(this.currentAvatar);
      if (!this.currentUserPubkey) {
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
  async createCsmAvatar(meshData: { vertices: number[]; indices: number[]; normals: number[]; colors: number[] }, userNpub: string | undefined = undefined, scale: number = 1.0, transform?: Transform, renderScaleDepth?: number, textureName?: string): Promise<void> {
    // Wait for textures to load if texture is requested
    if (textureName && textureName !== '0' && !this.texturesLoaded) {
      logger.log('renderer', `[CSM Avatar] Waiting for textures to load before creating avatar with texture '${textureName}'...`);
      await this.loadMaterialsAndTextures();
    }

    // Remove existing avatar
    if (this.currentAvatar) {
      this.scene.remove(this.currentAvatar.getObject3D());
      this.currentAvatar.dispose();
    }

    // Get material ID if specified
    let materialId: number | undefined = undefined;
    if (textureName && textureName !== '0') {
      const materials = this.materialsLoader['materialsData'];
      if (materials) {
        const material = materials.materials.find(m => m.id === textureName);
        if (material) {
          materialId = material.index;
          logger.log('renderer', `Using material ID ${materialId} for texture '${textureName}' on CSM avatar`);
        } else {
          logger.warn('renderer', `Texture '${textureName}' not found for CSM avatar`);
        }
      } else {
        logger.warn('renderer', `[CSM Avatar] Materials data not loaded yet - this should not happen after await`);
      }
    }

    // Create new voxel avatar (CSM avatars use VoxelAvatar class) with shared texture system
    const voxelAvatar = new VoxelAvatar({
      userNpub: userNpub ?? '',
      scale,
      renderScaleDepth,
      materialId,
      textures: this.materialsLoader.getTextureArray(),
      enableTextures: this.avatarTexturesEnabled,
      renderer: this.renderer,
      scene: this.scene,
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
    this.applyCurrentUserProfile(this.currentAvatar);

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
        // createRemoteAvatar is async now, but we don't want to block the update loop
        this.createRemoteAvatar(pubkey, state).catch(error => {
          logger.error('renderer', `Failed to create remote avatar for ${state.npub}:`, error);
        });
      } else {
        // Update position for existing avatar
        this.updateRemoteAvatarPosition(pubkey, state);
      }
    });
  }

  /**
   * Create a remote avatar for another user
   */
  private async createRemoteAvatar(pubkey: string, state: AvatarState): Promise<void> {

    const { position, avatarType, avatarId, avatarUrl, avatarData, avatarTexture, npub } = state;

    logger.log('renderer', `[Scene] Creating remote avatar for ${npub}:`, { avatarType, avatarId, avatarUrl, avatarDataLength: avatarData?.length, avatarTexture });

    // Wait for textures to load if texture is requested
    if (avatarTexture && avatarTexture !== '0' && !this.texturesLoaded) {
      logger.log('renderer', `[Remote Avatar] Waiting for textures to load before creating avatar with texture '${avatarTexture}' for ${npub}...`);
      await this.loadMaterialsAndTextures();
    }

    // Create transform from position data
    const transform = Transform.fromEventData(position);

    if (avatarType === 'vox') {
      // Get material ID if specified
      let materialId: number | undefined = undefined;
      if (avatarTexture && avatarTexture !== '0') {
        logger.log('renderer', `[Remote Avatar] Loading texture: ${avatarTexture} for ${npub}`);
        // Look up material ID
        const materials = this.materialsLoader['materialsData'];
        if (materials) {
          const material = materials.materials.find(m => m.id === avatarTexture);
          if (material) {
            materialId = material.index;
            logger.log('renderer', `[Remote Avatar] Using material ID ${materialId} for texture '${avatarTexture}' for ${npub}`);
          } else {
            logger.warn('renderer', `[Remote Avatar] Material '${avatarTexture}' not found in materials.json for ${npub}`);
          }
        } else {
          logger.warn('renderer', `[Remote Avatar] Materials not loaded yet for ${npub} - this should not happen after await`);
        }
      } else {
        logger.log('renderer', `[Remote Avatar] No texture specified for ${npub} (using vertex colors only)`);
      }

      // Create voxel avatar with shared texture system
      const voxelAvatar = new VoxelAvatar({
        userNpub: npub,
        scale: 1.0,
        materialId,
        textures: this.materialsLoader.getTextureArray(),
        enableTextures: this.avatarTexturesEnabled,
        renderer: this.renderer,
        scene: this.scene,
        // renderScaleDepth defaults to 0.0 in VoxelAvatar
      }, transform, this.scene);

      // Generate or load geometry (use undefined for npub to preserve original colors)
      if (avatarId && avatarId !== 'generated') {
        logger.log('renderer', `[Scene] Loading VOX model from avatarId: ${avatarId}`);
        // Load from .vox file using model config
        import('../utils/modelConfig').then(({ getModelUrl }) => {
          const voxUrl = getModelUrl(avatarId, 'vox');

          if (!voxUrl) {
            logger.warn('renderer', `No model found for avatarId: ${avatarId}, procedural generation removed`);
            return;
          }

          import('../utils/voxLoader').then(({ loadVoxFromUrl }) => {
            // Pass undefined to preserve original colors
            loadVoxFromUrl(voxUrl, undefined).then((geometryData) => {
              voxelAvatar.applyGeometry(geometryData);
            }).catch(error => {
              logger.error('renderer', 'Failed to load .vox avatar for remote user:', error);
            });
          }).catch(err => logger.error('renderer', err));
        }).catch(err => logger.error('renderer', err));
      } else {
        // Procedural avatar generation removed
        logger.warn('renderer', `[Scene] No avatarId for ${npub}, procedural generation removed`);
      }

      // Add to scene
      this.scene.add(voxelAvatar.getObject3D());
      this.remoteAvatars.set(pubkey, voxelAvatar);
      this.remoteAvatarConfigs.set(pubkey, { avatarType, avatarId, avatarData });

      // Fetch and apply profile picture
      this.fetchAndApplyProfilePicture(pubkey, voxelAvatar, npub);

      logger.log('renderer', `Created remote voxel avatar for ${npub}`);
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

    const worldSize = getWorldSize(getMacroDepth(), getBorderDepth());

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
   * Set texture rendering enabled/disabled for the ground geometry mesh
   */
  setTextures(enabled: boolean): void {
    this.texturesEnabled = enabled;

    // Update the material uniform if textured mesh exists with shader material
    if (this.texturedMesh && this.texturedMesh.material instanceof THREE.ShaderMaterial) {
      const material = this.texturedMesh.material as THREE.ShaderMaterial;
      if (material.uniforms.enableTextures) {
        material.uniforms.enableTextures.value = enabled;
      }
    }

    logger.log('renderer', `Textures ${enabled ? 'enabled' : 'disabled'}`);
  }

  /**
   * Set texture rendering enabled/disabled for all avatars
   */
  setAvatarTextures(enabled: boolean): void {
    this.avatarTexturesEnabled = enabled;

    // Update current avatar material uniform
    if (this.currentAvatar) {
      const avatarObj = this.currentAvatar.getObject3D();
      avatarObj.traverse((child) => {
        if (child instanceof THREE.Mesh && child.material instanceof THREE.ShaderMaterial) {
          const material = child.material as THREE.ShaderMaterial;
          if (material.uniforms.enableTextures) {
            material.uniforms.enableTextures.value = enabled;
          }
        }
      });
    }

    // Update all remote avatars material uniforms
    for (const avatar of this.remoteAvatars.values()) {
      const avatarObj = avatar.getObject3D();
      avatarObj.traverse((child) => {
        if (child instanceof THREE.Mesh && child.material instanceof THREE.ShaderMaterial) {
          const material = child.material as THREE.ShaderMaterial;
          if (material.uniforms.enableTextures) {
            material.uniforms.enableTextures.value = enabled;
          }
        }
      });
    }

    logger.log('renderer', `Avatar textures ${enabled ? 'enabled' : 'disabled'}`);
  }

  // ============================================================================
  // Helper Methods - Extracted to reduce code duplication
  // ============================================================================

  /**
   * Calculate and update mouse position in normalized device coordinates
   * In mouse rotate mode (mode 2), uses center crosshair position instead of mouse pointer
   */
  private calculateMousePosition(event: MouseEvent, canvas: HTMLCanvasElement): void {
    if (this.mouseMode === 2) {
      this.mouse.x = 0;
      this.mouse.y = 0;
    } else {
      const rect = canvas.getBoundingClientRect();
      this.mouse.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
      this.mouse.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
    }
  }

  /**
   * Check if a voxel at the given position and size is within world bounds
   */
  private isVoxelInBounds(voxelCorner: THREE.Vector3, size: number): boolean {
    const halfWorld = getWorldSize(getMacroDepth(), getBorderDepth()) / 2;
    return voxelCorner.x >= -halfWorld && voxelCorner.x + size <= halfWorld &&
           voxelCorner.y >= -halfWorld && voxelCorner.y + size <= halfWorld &&
           voxelCorner.z >= -halfWorld && voxelCorner.z + size <= halfWorld;
  }

  /**
   * Handle continuous paint/erase at cursor position
   * Only paints/erases if the position has changed since last operation
   */
  private handleContinuousPaint(voxelCorner: THREE.Vector3, size: number): void {
    if (!this.isLeftMousePressed && !this.isRightMousePressed) return;

    const isNewPosition = !this.lastPaintedVoxel ||
      this.lastPaintedVoxel.x !== voxelCorner.x ||
      this.lastPaintedVoxel.y !== voxelCorner.y ||
      this.lastPaintedVoxel.z !== voxelCorner.z;

    if (isNewPosition) {
      if (this.isLeftMousePressed) {
        this.paintVoxelWithSize(voxelCorner.x, voxelCorner.y, voxelCorner.z, size);
      } else if (this.isRightMousePressed) {
        this.eraseVoxelWithSize(voxelCorner.x, voxelCorner.y, voxelCorner.z, size);
      }
      this.lastPaintedVoxel = { x: voxelCorner.x, y: voxelCorner.y, z: voxelCorner.z };
    }
  }

  /**
   * Unified function that performs raycast and updates both face highlight and voxel cursor
   * Returns true if cursor was positioned, false otherwise
   */
  private raycastAndUpdateCursor(): boolean {
    if (!this.previewCube) return false;

    const size = this.getCursorSize();
    const halfSize = size / 2;
    const absoluteDepth = this.getAbsoluteCursorDepth();
    const normalOffset = this.depthSelectMode === 1 ? halfSize : -halfSize;

    let cursorPositioned = false;

    // Try draw plane first (if active)
    if (this.activeEditPlane && this.activeEditPlaneNormal) {
      const planeHit = new THREE.Vector3();
      if (this.raycaster.ray.intersectPlane(this.activeEditPlane, planeHit)) {
        // Show face highlight only when not pressing mouse buttons
        if (!this.isLeftMousePressed && !this.isRightMousePressed) {
          this.updateFaceHighlight(planeHit, this.activeEditPlaneNormal, size);
        }

        // Position cursor: snap to grid, project to plane, apply offset
        const snapped = snapToGrid(planeHit, size) as { x: number; y: number; z: number };
        const snappedVec = new THREE.Vector3(snapped.x, snapped.y, snapped.z);
        const distanceToPlane = this.activeEditPlane.distanceToPoint(snappedVec);
        const projectedCenter = snappedVec.clone().sub(
          this.activeEditPlaneNormal.clone().multiplyScalar(distanceToPlane)
        );
        const cursorCenter = projectedCenter.clone().addScaledVector(this.activeEditPlaneNormal, normalOffset);
        const voxelCorner = cursorCenter.clone().subScalar(halfSize);

        if (this.isVoxelInBounds(voxelCorner, size)) {
          const newCoord = worldToCube(voxelCorner.x, voxelCorner.y, voxelCorner.z, absoluteDepth);

          // Only log when coordinate changes
          if (!this.currentCursorCoord ||
              this.currentCursorCoord.x !== newCoord.x ||
              this.currentCursorCoord.y !== newCoord.y ||
              this.currentCursorCoord.z !== newCoord.z) {
            console.log('[CURSOR] DrawPlane:', { oldCoord: this.currentCursorCoord, newCoord });
          }

          this.currentCursorCoord = newCoord;
          this.currentGridPosition.copy(cursorCenter);
          this.previewCube.position.copy(this.currentGridPosition);
          this.previewCube.visible = true;
          this.handleContinuousPaint(voxelCorner, size);
          cursorPositioned = true;
        }
      }
    }

    // Try geometry raycast (if no plane hit and geometry exists)
    if (!cursorPositioned && this.geometryMesh) {
      const hit = this.raycastGeometry();
      if (hit) {
        // Show face highlight only when not pressing mouse buttons
        if (!this.isLeftMousePressed && !this.isRightMousePressed) {
          this.updateFaceHighlight(hit.point, hit.normal, size);
        }

        // Position cursor: snap to voxel grid, calculate face center, apply offset
        const coord = worldToCube(hit.point.x, hit.point.y, hit.point.z, absoluteDepth);
        const [voxelX, voxelY, voxelZ] = cubeToWorld(coord);
        const voxelCorner = new THREE.Vector3(voxelX, voxelY, voxelZ);
        const voxelCenter = this.calculateVoxelCenter(voxelCorner, halfSize);
        const faceCenter = this.calculateFaceCenter(voxelCenter, hit.normal, halfSize);
        const cursorCenter = faceCenter.clone().addScaledVector(hit.normal, normalOffset);
        const cursorVoxelCorner = cursorCenter.clone().subScalar(halfSize);

        if (this.isVoxelInBounds(cursorVoxelCorner, size)) {
          const newCoord = worldToCube(cursorVoxelCorner.x, cursorVoxelCorner.y, cursorVoxelCorner.z, absoluteDepth);

          // Only log when coordinate changes
          if (!this.currentCursorCoord ||
              this.currentCursorCoord.x !== newCoord.x ||
              this.currentCursorCoord.y !== newCoord.y ||
              this.currentCursorCoord.z !== newCoord.z) {
            console.log('[CURSOR] GeometryHit:', { oldCoord: this.currentCursorCoord, newCoord });
          }

          this.currentCursorCoord = newCoord;
          this.currentGridPosition.copy(cursorCenter);
          this.previewCube.position.copy(this.currentGridPosition);
          this.previewCube.visible = true;
          this.handleContinuousPaint(cursorVoxelCorner, size);
          cursorPositioned = true;
        }
      }
    }

    // Fallback to plane intersection (ground or active edit plane)
    if (!cursorPositioned) {
      const targetPlane = this.activeEditPlane || this.groundPlane;
      const intersectPoint = new THREE.Vector3();
      if (this.raycaster.ray.intersectPlane(targetPlane, intersectPoint)) {
        if (this.activeEditPlane && this.activeEditPlaneNormal) {
          // Active edit plane with normal - apply SPACE mode
          const snapped = snapToGrid(intersectPoint, size) as { x: number; y: number; z: number };
          const faceCenter = new THREE.Vector3(snapped.x, snapped.y, snapped.z);
          const cursorCenter = faceCenter.clone().addScaledVector(this.activeEditPlaneNormal, normalOffset);
          const voxelCorner = cursorCenter.clone().subScalar(halfSize);

          if (this.isVoxelInBounds(voxelCorner, size)) {
            this.currentCursorCoord = worldToCube(voxelCorner.x, voxelCorner.y, voxelCorner.z, absoluteDepth);
            this.currentGridPosition.copy(cursorCenter);
            this.previewCube.position.copy(this.currentGridPosition);
            this.previewCube.visible = true;
            this.handleContinuousPaint(voxelCorner, size);
            cursorPositioned = true;
          }
        } else {
          // Ground plane mode - snap X and Z
          const voxelCenterX = snapToGrid(intersectPoint.x, size) as number;
          const voxelCenterZ = snapToGrid(intersectPoint.z, size) as number;
          const voxelY = this.depthSelectMode === 1 ? 0 : -size;
          const voxelCorner = new THREE.Vector3(voxelCenterX - halfSize, voxelY, voxelCenterZ - halfSize);

          if (this.isVoxelInBounds(voxelCorner, size)) {
            this.currentCursorCoord = worldToCube(voxelCorner.x, voxelCorner.y, voxelCorner.z, absoluteDepth);
            this.currentGridPosition.set(voxelCenterX, voxelY + halfSize, voxelCenterZ);
            this.previewCube.position.copy(this.currentGridPosition);
            this.previewCube.visible = true;
            this.handleContinuousPaint(voxelCorner, size);
            cursorPositioned = true;
          }
        }
      }
    }

    // Hide cursor and face highlight if no valid position found
    if (!cursorPositioned) {
      this.previewCube.visible = false;
      this.currentCursorCoord = null;
      this.hideFaceHighlight();
    }

    return cursorPositioned;
  }

  /**
   * Calculate face center position from voxel center, normal, and half size
   */
  private calculateFaceCenter(
    voxelCenter: THREE.Vector3,
    normal: THREE.Vector3,
    halfSize: number
  ): THREE.Vector3 {
    // Face center = voxelCenter + normal * (halfSize - 1)
    return voxelCenter.clone().addScaledVector(normal, halfSize - 1);
  }

  /**
   * Calculate voxel center from corner position
   */
  private calculateVoxelCenter(voxelCorner: THREE.Vector3, halfSize: number): THREE.Vector3 {
    return voxelCorner.clone().addScalar(halfSize);
  }

  /**
   * Adjust cursor depth by delta and update cursor visualization
   * Used for arrow up/down keys
   */
  private adjustCursorDepth(delta: 1 | -1): void {
    if (this.currentMode === 'placement' && this.placementMode) {
      const currentScale = this.placementMode.getScale();
      this.placementMode.setScale(delta > 0 ? currentScale + 1 : Math.max(0, currentScale - 1));
    } else if (this.isEditMode) {
      this.cursorDepth = delta > 0
        ? Math.min(getMaxCursorDepth(), this.cursorDepth + 1)
        : Math.max(getMinCursorDepth(), this.cursorDepth - 1);
      this.updateCursorSize();
      logger.log('renderer', `[Cursor Depth] Changed to ${this.cursorDepth}, absolute depth: ${this.getAbsoluteCursorDepth()}, size: ${this.getCursorSize()}`);

      if (this.mouseMode === 2) {
        this.updateVoxelCursorAtCenter();
      } else {
        this.updateCursorVisualization();
      }
    }
  }

  /**
   * Reset mouse mode and clear edit state
   * Exits pointer lock, clears paint state, and clears active edit plane
   */
  private resetMouseMode(): void {
    if (this.mouseMode === 2) {
      this.mouseMode = 1;
      document.exitPointerLock();
      if (this.crosshair) {
        this.crosshair.style.display = 'none';
      }
    }
    this.isLeftMousePressed = false;
    this.isRightMousePressed = false;
    this.lastPaintedVoxel = null;
    this.clearActiveEditPlane();
  }

  /**
   * Apply current user's profile picture to an avatar
   */
  private applyCurrentUserProfile(avatar: IAvatar): void {
    if (this.currentUserPubkey) {
      this.fetchAndApplyProfilePicture(this.currentUserPubkey, avatar);
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

    // Clean up gamepad controller
    if (this.gamepadController) {
      this.gamepadController.dispose();
      this.gamepadController = null;
    }

    logger.log('renderer', '[SceneManager] Disposed');
  }
}
