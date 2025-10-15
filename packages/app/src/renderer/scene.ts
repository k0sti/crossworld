import * as THREE from 'three';
import { Avatar } from './avatar';
import { VoxelAvatar } from './voxel-avatar';
import type { AvatarEngine } from '@workspace/wasm';

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
  private cameraOffset: THREE.Vector3;
  private lastTime: number = 0;

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
    this.cameraOffset = new THREE.Vector3(4, 6, 4);
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
    this.lastTime = performance.now();
  }

  private setupMouseListener(canvas: HTMLCanvasElement): void {
    canvas.addEventListener('click', (event) => {
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
      this.voxelAvatar.update(deltaTime_s);
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

  createAvatar(modelUrl?: string, scale?: number): void {
    if (this.avatar) {
      this.scene.remove(this.avatar.getObject3D());
    }
    this.avatar = new Avatar(4, 4, { modelUrl, scale });
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
   * Create a voxel avatar for a user
   */
  createVoxelAvatar(userNpub: string, scale: number = 1.0): void {
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
    this.voxelAvatar = new VoxelAvatar({ userNpub, scale }, 4, 4);

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
  async createVoxelAvatarFromVoxFile(voxUrl: string, userNpub: string, scale: number = 1.0): Promise<void> {
    // Import the loadVoxFromUrl function
    const { loadVoxFromUrl } = await import('../utils/voxLoader');

    try {
      // Load .vox file and get geometry
      const geometryData = await loadVoxFromUrl(voxUrl, userNpub);

      // Remove existing voxel avatar
      if (this.voxelAvatar) {
        this.scene.remove(this.voxelAvatar.getObject3D());
        this.voxelAvatar.dispose();
      }

      // Create new voxel avatar
      this.voxelAvatar = new VoxelAvatar({ userNpub, scale }, 4, 4);

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
}
