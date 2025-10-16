import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';

export interface AvatarConfig {
  modelUrl?: string;
  scale?: number;
}

export class Avatar {
  private group: THREE.Group;
  private model: THREE.Object3D | null = null;
  private position: THREE.Vector3;
  private targetPosition: THREE.Vector3;
  private isMoving: boolean = false;
  private moveSpeed: number = 3.0; // units per second
  private config: AvatarConfig;
  private mixer: THREE.AnimationMixer | null = null;
  private animations: THREE.AnimationClip[] = [];
  private currentAction: THREE.AnimationAction | null = null;

  constructor(initialX: number = 4, initialZ: number = 4, config: AvatarConfig = {}) {
    this.position = new THREE.Vector3(initialX, 0, initialZ);
    this.targetPosition = this.position.clone();
    this.config = config;

    this.group = new THREE.Group();
    this.group.position.copy(this.position);

    // Load GLB model
    this.loadModel();
  }

  private loadModel() {
    const loader = new GLTFLoader();
    const modelUrl = this.config.modelUrl || `${import.meta.env.BASE_URL}models/avatar.glb`;

    console.log('Loading avatar model from:', modelUrl);

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

          console.log(`Avatar loaded with ${this.animations.length} animations:`,
            this.animations.map(a => a.name));

          // Try to find and play idle animation by default
          this.playAnimation('Idle', true);
        }

        console.log('Avatar model loaded successfully');
        console.log('Model size:', size);
        console.log('Bounding box:', box.min, box.max);
      },
      (progress) => {
        const percent = (progress.loaded / progress.total * 100).toFixed(0);
        console.log(`Loading avatar: ${percent}%`);
      },
      (error) => {
        console.warn('Failed to load avatar model, using fallback geometry:', error);
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
      console.log(`Playing animation: ${clip.name}`);
    } else {
      console.warn(`Animation "${name}" not found. Available:`, this.animations.map(a => a.name));
    }
  }

  setTargetPosition(x: number, z: number) {
    // Keep y at 0 (ground level)
    this.targetPosition.set(x, 0, z);

    const wasMoving = this.isMoving;
    this.isMoving = true;

    // Start walking animation if not already moving
    if (!wasMoving) {
      this.playAnimation('Walk', true);
    }

    // Rotate to face target
    const direction = new THREE.Vector3()
      .subVectors(this.targetPosition, this.position)
      .normalize();

    if (direction.length() > 0.01) {
      const angle = Math.atan2(direction.x, direction.z);
      this.group.rotation.y = angle;
    }
  }

  update(deltaTime_s: number) {
    // Update animation mixer
    if (this.mixer) {
      this.mixer.update(deltaTime_s);
    }

    if (!this.isMoving) return;

    const distance = this.position.distanceTo(this.targetPosition);

    if (distance < 0.1) {
      // Reached target
      this.position.copy(this.targetPosition);
      this.group.position.copy(this.position);
      this.isMoving = false;

      // Switch to idle animation
      this.playAnimation('Idle', true);
      return;
    }

    // Move towards target
    const moveDistance = this.moveSpeed * deltaTime_s;
    const direction = new THREE.Vector3()
      .subVectors(this.targetPosition, this.position)
      .normalize();

    this.position.addScaledVector(direction, Math.min(moveDistance, distance));
    this.group.position.copy(this.position);
  }

  getObject3D(): THREE.Group {
    return this.group;
  }

  getPosition(): THREE.Vector3 {
    return this.position.clone();
  }

  isCurrentlyMoving(): boolean {
    return this.isMoving;
  }
}
