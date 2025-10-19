import * as THREE from 'three';
import { Avatar } from './avatar';
import { VoxelAvatar } from './voxel-avatar';
import type { AvatarEngine } from '@workspace/wasm';
import type { AvatarState } from '../services/avatar-state';

export class SceneManager {
  private scene: THREE.Scene;
  private camera: THREE.PerspectiveCamera;
  private renderer: THREE.WebGLRenderer;
  private geometryMesh: THREE.Mesh | null = null;
  private avatar: Avatar | null = null;
  private voxelAvatar: VoxelAvatar | null = null;
  private avatarEngine: AvatarEngine | null = null;
  private raycaster: THREE.Raycaster;
  private mouse: THREE.Vector2;
  private lastTime: number = 0;
  private isEditMode: boolean = false;
  private gridHelper: THREE.GridHelper | null = null;
  private previewCube: THREE.LineSegments | null = null;
  private currentGridPosition: THREE.Vector3 = new THREE.Vector3();
  private onPositionUpdate?: (x: number, y: number, z: number, quaternion: [number, number, number, number]) => void;

  // Remote avatars for other users
  private remoteAvatars: Map<string, VoxelAvatar | Avatar> = new Map();
  private currentUserPubkey: string | null = null;

  // Position update tracking for player avatar
  private lastPublishedPosition: THREE.Vector3 | null = null;
  private lastPublishTime: number = 0;
  private readonly PUBLISH_INTERVAL_MS = 500; // 500ms
  private readonly MIN_POSITION_CHANGE = 0.1; // Minimum movement to trigger update

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

    // Set fixed camera position for isometric-like view
    this.camera.position.set(8, 6, 8);
    this.camera.lookAt(4, 0, 4); // Look at center of 8x8 grid

    this.scene.background = new THREE.Color(0x87ceeb); // Sky blue
    this.scene.fog = new THREE.Fog(0x87ceeb, 10, 50);

    this.setupLights();
    this.setupMouseListener(canvas);
    this.setupMouseMoveListener(canvas);
    this.setupEditModeHelpers();
    this.lastTime = performance.now();
  }

  private setupMouseListener(canvas: HTMLCanvasElement): void {
    canvas.addEventListener('click', (event) => {
      // Don't move avatar in edit mode
      if (this.isEditMode) return;

      // Need either avatar and geometry mesh to handle clicks
      if ((!this.avatar && !this.voxelAvatar) || !this.geometryMesh) return;

      // Calculate mouse position in normalized device coordinates (-1 to +1)
      const rect = canvas.getBoundingClientRect();
      this.mouse.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
      this.mouse.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;

      // Update raycaster
      this.raycaster.setFromCamera(this.mouse, this.camera);

      // Check intersection with ground plane
      const intersects = this.raycaster.intersectObject(this.geometryMesh);

      if (intersects.length > 0) {
        const point = intersects[0].point;
        // Clamp to ground bounds (0-8 range)
        const clampedX = Math.max(0.5, Math.min(7.5, point.x));
        const clampedZ = Math.max(0.5, Math.min(7.5, point.z));

        // Move whichever avatar exists
        if (this.avatar) {
          this.avatar.setTargetPosition(clampedX, clampedZ);
        }
        if (this.voxelAvatar) {
          this.voxelAvatar.setTargetPosition(clampedX, clampedZ);
        }
      }
    });
  }

  private setupLights(): void {
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.6);
    this.scene.add(ambientLight);

    const directionalLight = new THREE.DirectionalLight(0xffffff, 0.8);
    directionalLight.position.set(10, 15, 10);
    directionalLight.castShadow = true;
    directionalLight.shadow.camera.near = 0.1;
    directionalLight.shadow.camera.far = 50;
    directionalLight.shadow.camera.left = -10;
    directionalLight.shadow.camera.right = 10;
    directionalLight.shadow.camera.top = 10;
    directionalLight.shadow.camera.bottom = -10;
    this.scene.add(directionalLight);

    const hemisphereLight = new THREE.HemisphereLight(0x87ceeb, 0x080820, 0.5);
    this.scene.add(hemisphereLight);
  }

  private setupEditModeHelpers(): void {
    // Create grid helper (8x8 grid to match ground)
    this.gridHelper = new THREE.GridHelper(8, 8, 0xffffff, 0xffffff);
    this.gridHelper.position.set(4, 0.01, 4); // Slightly above ground
    this.gridHelper.material = new THREE.LineBasicMaterial({
      color: 0xffffff,
      opacity: 0.3,
      transparent: true
    });
    this.gridHelper.visible = false;
    this.scene.add(this.gridHelper);

    // Create preview cube (1x1x1 cube wireframe)
    const cubeGeometry = new THREE.BoxGeometry(1, 1, 1);
    const edges = new THREE.EdgesGeometry(cubeGeometry);
    const lineMaterial = new THREE.LineBasicMaterial({
      color: 0x00ff00,
      linewidth: 2,
      opacity: 0.7,
      transparent: true
    });
    this.previewCube = new THREE.LineSegments(edges, lineMaterial);
    this.previewCube.visible = false;
    this.scene.add(this.previewCube);
  }

  private setupMouseMoveListener(canvas: HTMLCanvasElement): void {
    canvas.addEventListener('mousemove', (event) => {
      if (!this.isEditMode || !this.geometryMesh || !this.previewCube) return;

      // Calculate mouse position in normalized device coordinates
      const rect = canvas.getBoundingClientRect();
      this.mouse.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
      this.mouse.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;

      // Update raycaster
      this.raycaster.setFromCamera(this.mouse, this.camera);

      // Check intersection with ground plane
      const intersects = this.raycaster.intersectObject(this.geometryMesh);

      if (intersects.length > 0) {
        const point = intersects[0].point;

        // Snap to grid (1 unit grid)
        const snappedX = Math.floor(point.x) + 0.5;
        const snappedZ = Math.floor(point.z) + 0.5;

        // Clamp to ground bounds (0-8 range)
        const clampedX = Math.max(0.5, Math.min(7.5, snappedX));
        const clampedZ = Math.max(0.5, Math.min(7.5, snappedZ));

        // Position preview cube at ground level
        this.currentGridPosition.set(clampedX, 0.5, clampedZ);
        this.previewCube.position.copy(this.currentGridPosition);
        this.previewCube.visible = true;
      } else {
        this.previewCube.visible = false;
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
      specular: 0x111111,
      shininess: 30,
      wireframe: false,
      side: THREE.DoubleSide
    });

    this.geometryMesh = new THREE.Mesh(geometry, material);
    this.geometryMesh.castShadow = true;
    this.geometryMesh.receiveShadow = true;
    this.scene.add(this.geometryMesh);
  }

  render(): void {
    const currentTime = performance.now();
    const deltaTime_s = (currentTime - this.lastTime) / 1000;
    this.lastTime = currentTime;

    // Update avatar
    if (this.avatar) {
      this.avatar.update(deltaTime_s);
    }

    // Update voxel avatar
    if (this.voxelAvatar) {
      const wasMoving = this.voxelAvatar.isCurrentlyMoving();
      this.voxelAvatar.update(deltaTime_s);
      const isMoving = this.voxelAvatar.isCurrentlyMoving();

      // Check if movement state changed or if currently moving
      if (wasMoving && !isMoving) {
        // Just stopped moving - publish final position
        this.publishPlayerPosition();
      } else if (!wasMoving && isMoving) {
        // Just started moving - publish initial position
        this.publishPlayerPosition();
      } else if (isMoving) {
        // Currently moving - publish periodically
        this.checkAndPublishPlayerPosition();
      }
    }

    // Update all remote avatars
    for (const avatar of this.remoteAvatars.values()) {
      avatar.update(deltaTime_s);
    }

    this.renderer.render(this.scene, this.camera);
  }

  handleResize(): void {
    const width = window.innerWidth;
    const height = window.innerHeight;

    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(width, height);
  }

  getCamera(): THREE.PerspectiveCamera {
    return this.camera;
  }

  getScene(): THREE.Scene {
    return this.scene;
  }

  createAvatar(modelUrl?: string, scale?: number, position?: { x: number; z: number }): void {
    if (this.avatar) {
      this.scene.remove(this.avatar.getObject3D());
    }
    const { x = 4, z = 4 } = position || {};
    this.avatar = new Avatar(x, z, { modelUrl, scale });
    this.scene.add(this.avatar.getObject3D());
  }

  removeAvatar(): void {
    if (this.avatar) {
      this.scene.remove(this.avatar.getObject3D());
      this.avatar = null;

      // Reset camera to default position
      this.camera.position.set(8, 6, 8);
      this.camera.lookAt(4, 0, 4);
    }
  }

  hasAvatar(): boolean {
    return this.avatar !== null || this.voxelAvatar !== null;
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
  setPositionUpdateCallback(callback: (x: number, y: number, z: number, quaternion: [number, number, number, number]) => void): void {
    this.onPositionUpdate = callback;
  }

  /**
   * Create a voxel avatar for a user
   */
  createVoxelAvatar(userNpub: string, scale: number = 1.0, position?: { x: number; z: number }): void {
    if (!this.avatarEngine) {
      console.error('Avatar engine not initialized');
      return;
    }

    // Remove existing voxel avatar
    if (this.voxelAvatar) {
      this.scene.remove(this.voxelAvatar.getObject3D());
      this.voxelAvatar.dispose();
    }

    // Create new voxel avatar
    const { x = 4, z = 4 } = position || {};
    this.voxelAvatar = new VoxelAvatar({
      userNpub: userNpub || '',
      scale,
    }, x, z);

    // Generate geometry from Rust
    const geometryData = this.avatarEngine.generate_avatar(userNpub);

    // Apply geometry to avatar
    this.voxelAvatar.applyGeometry(geometryData);

    // Add to scene
    this.scene.add(this.voxelAvatar.getObject3D());

    console.log(`Created voxel avatar for ${userNpub}`);
  }

  /**
   * Create a voxel avatar from a .vox file
   */
  async createVoxelAvatarFromVoxFile(voxUrl: string, userNpub: string | undefined = undefined, scale: number = 1.0, position?: { x: number; z: number }): Promise<void> {
    // Import the loadVoxFromUrl function
    const { loadVoxFromUrl } = await import('../utils/voxLoader');

    try {
      // Load .vox file and get geometry (pass undefined for original colors)
      const geometryData = await loadVoxFromUrl(voxUrl, userNpub ?? undefined);

      // Remove existing voxel avatar
      if (this.voxelAvatar) {
        this.scene.remove(this.voxelAvatar.getObject3D());
        this.voxelAvatar.dispose();
      }

      // Create new voxel avatar
      const { x = 4, z = 4 } = position || {};
      this.voxelAvatar = new VoxelAvatar({
        userNpub: userNpub ?? '',
        scale,
      }, x, z);

      // Apply geometry from .vox file
      this.voxelAvatar.applyGeometry(geometryData);

      // Add to scene
      this.scene.add(this.voxelAvatar.getObject3D());

      console.log(`Created voxel avatar from .vox file: ${voxUrl}`);
    } catch (error) {
      console.error('Failed to load .vox avatar:', error);
      throw error;
    }
  }

  /**
   * Remove voxel avatar from scene
   */
  removeVoxelAvatar(): void {
    if (this.voxelAvatar) {
      this.scene.remove(this.voxelAvatar.getObject3D());
      this.voxelAvatar.dispose();
      this.voxelAvatar = null;

      // Reset camera to default position
      this.camera.position.set(8, 6, 8);
      this.camera.lookAt(4, 0, 4);
    }
  }

  /**
   * Get the current voxel avatar
   */
  getVoxelAvatar(): VoxelAvatar | null {
    return this.voxelAvatar;
  }

  /**
   * Set edit mode to show/hide grid helpers
   */
  setEditMode(isEditMode: boolean): void {
    this.isEditMode = isEditMode;

    if (this.gridHelper) {
      this.gridHelper.visible = isEditMode;
    }

    if (this.previewCube && !isEditMode) {
      this.previewCube.visible = false;
    }
  }

  /**
   * Set the current user's pubkey (to exclude from remote avatars)
   */
  setCurrentUserPubkey(pubkey: string | null): void {
    this.currentUserPubkey = pubkey;
  }

  /**
   * Check if enough time/distance has passed to publish position update
   */
  private checkAndPublishPlayerPosition(): void {
    if (!this.voxelAvatar || !this.onPositionUpdate) return;

    const now = Date.now();
    const timeSinceLastPublish = now - this.lastPublishTime;

    // Check if enough time has passed
    if (timeSinceLastPublish < this.PUBLISH_INTERVAL_MS) {
      return;
    }

    const currentPosition = this.voxelAvatar.getPosition();

    // Check if position changed significantly
    if (this.lastPublishedPosition) {
      const distanceMoved = currentPosition.distanceTo(this.lastPublishedPosition);
      if (distanceMoved < this.MIN_POSITION_CHANGE) {
        return;
      }
    }

    this.publishPlayerPosition();
  }

  /**
   * Publish current player position
   */
  private publishPlayerPosition(): void {
    if (!this.voxelAvatar || !this.onPositionUpdate) return;

    const position = this.voxelAvatar.getPosition();
    const group = this.voxelAvatar.getObject3D();
    const quaternion = group.quaternion;

    const quaternionArray: [number, number, number, number] = [
      quaternion.x,
      quaternion.y,
      quaternion.z,
      quaternion.w
    ];

    this.onPositionUpdate(
      position.x,
      position.y,
      position.z,
      quaternionArray
    );

    // Update tracking
    this.lastPublishedPosition = position.clone();
    this.lastPublishTime = Date.now();
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
      activePubkeys.add(pubkey);
    });

    // Remove avatars for users that are no longer active
    for (const [pubkey, avatar] of this.remoteAvatars.entries()) {
      if (!activePubkeys.has(pubkey)) {
        this.scene.remove(avatar.getObject3D());
        if (avatar instanceof VoxelAvatar) {
          avatar.dispose();
        }
        this.remoteAvatars.delete(pubkey);
        console.log(`Removed remote avatar for ${pubkey}`);
      }
    }

    // Create or update avatars for active users
    states.forEach((state, pubkey) => {
      // Skip current user
      if (pubkey === this.currentUserPubkey) return;

      const existing = this.remoteAvatars.get(pubkey);

      // Check if we need to create a new avatar
      if (!existing) {
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

    const { position, avatarType, avatarModel, avatarUrl, npub } = state;

    if (avatarType === 'voxel') {
      // Create voxel avatar
      const voxelAvatar = new VoxelAvatar({
        userNpub: npub,
        scale: 1.0,
      }, position.x, position.z);

      // Generate or load geometry
      if (avatarModel && avatarModel !== 'generated') {
        // Load from .vox file
        const voxFilename = avatarModel === 'boy'
          ? 'chr_peasant_guy_blackhair.vox'
          : 'chr_peasant_girl_orangehair.vox';
        const voxUrl = `${import.meta.env.BASE_URL}assets/models/vox/${voxFilename}`;

        import('../utils/voxLoader').then(({ loadVoxFromUrl }) => {
          loadVoxFromUrl(voxUrl, npub).then((geometryData) => {
            voxelAvatar.applyGeometry(geometryData);
          }).catch(error => {
            console.error('Failed to load .vox avatar for remote user:', error);
            // Fallback to generated
            const geometryData = this.avatarEngine!.generate_avatar(npub);
            voxelAvatar.applyGeometry(geometryData);
          });
        }).catch(console.error);
      } else {
        // Use procedurally generated model
        const geometryData = this.avatarEngine.generate_avatar(npub);
        voxelAvatar.applyGeometry(geometryData);
      }

      // Add to scene
      this.scene.add(voxelAvatar.getObject3D());
      this.remoteAvatars.set(pubkey, voxelAvatar);
      console.log(`Created remote voxel avatar for ${npub}`);
    } else {
      // Create GLB avatar
      const glbAvatar = new Avatar(position.x, position.z, {
        modelUrl: avatarUrl,
        scale: 1.0,
      });
      this.scene.add(glbAvatar.getObject3D());
      this.remoteAvatars.set(pubkey, glbAvatar);
      console.log(`Created remote GLB avatar for ${npub}`);
    }
  }

  /**
   * Update remote avatar position
   */
  private updateRemoteAvatarPosition(pubkey: string, state: AvatarState): void {
    const avatar = this.remoteAvatars.get(pubkey);
    if (!avatar) return;

    const { position } = state;

    // Update target position
    avatar.setTargetPosition(position.x, position.z);

    // Update rotation if quaternion is available
    if (position.quaternion && avatar.getObject3D()) {
      const [qx, qy, qz, qw] = position.quaternion;
      const quaternion = new THREE.Quaternion(qx, qy, qz, qw);
      avatar.getObject3D().quaternion.copy(quaternion);
    }
  }
}
