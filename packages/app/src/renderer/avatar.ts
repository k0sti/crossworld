import * as logger from '../utils/logger';
import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { Transform } from './transform';
import { BaseAvatar } from './base-avatar';

export interface AvatarConfig {
  modelUrl?: string;
  scale?: number;
}

/**
 * GLB-based avatar with animation support
 */
export class Avatar extends BaseAvatar {
  private model: THREE.Object3D | null = null;
  private config: AvatarConfig;
  private mixer: THREE.AnimationMixer | null = null;
  private animations: THREE.AnimationClip[] = [];
  private currentAction: THREE.AnimationAction | null = null;

  constructor(initialTransform?: Transform, config: AvatarConfig = {}, scene?: THREE.Scene) {
    super(initialTransform, scene);
    this.config = config;

    // Load GLB model
    this.loadModel();
  }

  // ========== BaseAvatar hooks ==========

  protected getModel(): THREE.Object3D | null {
    return this.model;
  }

  protected getRotationOffset(): number {
    return 0; // GLB models face forward by default
  }

  protected onStartMoving(): void {
    this.playAnimation('Walk', true);
  }

  protected onStopMoving(): void {
    this.playAnimation('Idle', true);
  }

  // ========== GLB-specific implementation ==========

  private loadModel() {
    const loader = new GLTFLoader();
    const modelUrl = this.config.modelUrl || `${import.meta.env.BASE_URL}models/avatar.glb`;

    logger.log('renderer', 'Loading avatar model from:', modelUrl);

    // Try to load model, fallback to simple geometry if not found
    loader.load(
      modelUrl,
      (gltf) => {
        this.model = gltf.scene;
        this.model.traverse((child) => {
          if ((child as THREE.Mesh).isMesh) {
            child.castShadow = true;
            child.receiveShadow = true;
          }
        });

        // Scale model to reasonable size
        const scale = this.config.scale || 1.0;
        this.model.scale.set(scale, scale, scale);

        // Position model with feet at y=0 (ground level)
        const box = new THREE.Box3().setFromObject(this.model);
        const size = box.getSize(new THREE.Vector3());
        const center = box.getCenter(new THREE.Vector3());

        // Center horizontally (x,z) but keep bottom at y=0
        this.model.position.x = -center.x;
        this.model.position.z = -center.z;
        this.model.position.y = -box.min.y; // Lift model so bottom is at y=0

        this.group.add(this.model);

        // Setup animations
        if (gltf.animations && gltf.animations.length > 0) {
          this.animations = gltf.animations;
          this.mixer = new THREE.AnimationMixer(this.model);

          logger.log('renderer', `Avatar loaded with ${this.animations.length} animations:`,
            this.animations.map(a => a.name));

          // Try to find and play idle animation by default
          this.playAnimation('Idle', true);
        }

        logger.log('renderer', 'Avatar model loaded successfully');
        logger.log('renderer', 'Model size:', size);
        logger.log('renderer', 'Bounding box:', box.min, box.max);
      },
      (progress) => {
        const percent = (progress.loaded / progress.total * 100).toFixed(0);
        logger.log('renderer', `Loading avatar: ${percent}%`);
      },
      (error) => {
        logger.warn('renderer', 'Failed to load avatar model, using fallback geometry:', error);
        this.createFallbackModel();
      }
    );
  }

  private createFallbackModel() {
    // Fallback: simple arrow shape pointing forward
    const geometry = new THREE.ConeGeometry(0.3, 1.0, 4);
    const material = new THREE.MeshPhongMaterial({
      color: 0x4488ff,
      emissive: 0x112244,
      flatShading: true
    });
    this.model = new THREE.Mesh(geometry, material);
    this.model.castShadow = true;
    this.model.receiveShadow = true;

    // Rotate to point forward (along +Z axis)
    // Cone default points +Y (up), rotate +90° around X to point +Z (forward)
    this.model.rotation.x = Math.PI / 2;
    this.model.position.y = 0.5;

    this.group.add(this.model);
  }

  private playAnimation(name: string, loop: boolean = true) {
    if (!this.mixer || this.animations.length === 0) return;

    // Stop current animation
    if (this.currentAction) {
      this.currentAction.fadeOut(0.2);
    }

    // Find animation by name (case-insensitive, partial match)
    const clip = this.animations.find(anim =>
      anim.name.toLowerCase().includes(name.toLowerCase())
    );

    if (clip) {
      this.currentAction = this.mixer.clipAction(clip);
      this.currentAction.reset();
      this.currentAction.setLoop(loop ? THREE.LoopRepeat : THREE.LoopOnce, loop ? Infinity : 1);
      this.currentAction.fadeIn(0.2);
      this.currentAction.play();
      logger.log('renderer', `Playing animation: ${clip.name}`);
    } else {
      logger.warn('renderer', `Animation "${name}" not found. Available:`, this.animations.map(a => a.name));
    }
  }

  // Override update to add animation mixer update
  update(deltaTime_s: number): void {
    // Update animation mixer
    if (this.mixer) {
      this.mixer.update(deltaTime_s);
    }

    // Call base class update for movement/teleport
    super.update(deltaTime_s);
  }

  dispose(): void {
    // Dispose animation mixer
    if (this.mixer) {
      this.mixer.stopAllAction();
    }

    // Dispose model geometry and materials
    if (this.model) {
      this.model.traverse((child) => {
        if ((child as THREE.Mesh).isMesh) {
          const mesh = child as THREE.Mesh;
          if (mesh.geometry) {
            mesh.geometry.dispose();
          }
          if (mesh.material) {
            if (Array.isArray(mesh.material)) {
              mesh.material.forEach((m) => m.dispose());
            } else {
              mesh.material.dispose();
            }
          }
        }
      });
    }

    // Dispose profile icon
    if (this.profileIcon) {
      this.profileIcon.dispose();
    }
  }
}
