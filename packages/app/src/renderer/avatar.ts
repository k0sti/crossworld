import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { Transform } from './transform';

export interface AvatarConfig {
  modelUrl?: string;
  scale?: number;
}

export class Avatar {
  private group: THREE.Group;
  private model: THREE.Object3D | null = null;
  private transform: Transform;
  private targetTransform: Transform;
  private isMoving: boolean = false;
  private moveSpeed: number = 3.0; // units per second
  private config: AvatarConfig;
  private mixer: THREE.AnimationMixer | null = null;
  private animations: THREE.AnimationClip[] = [];
  private currentAction: THREE.AnimationAction | null = null;

  constructor(initialTransform?: Transform, config: AvatarConfig = {}) {
    // Use provided transform or create default at (4, 0, 4)
    this.transform = initialTransform ? Transform.fromTransform(initialTransform) : new Transform(4, 0, 4);
    this.targetTransform = this.transform.clone();
    this.config = config;

    this.group = new THREE.Group();
    this.transform.applyToObject3D(this.group);

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
    this.targetTransform.setXZ(x, z);

    const wasMoving = this.isMoving;
    this.isMoving = true;

    // Start walking animation if not already moving
    if (!wasMoving) {
      this.playAnimation('Walk', true);
    }

    // Calculate direction and rotate to face it
    const dx = x - this.transform.getX();
    const dz = z - this.transform.getZ();
    const distance = Math.sqrt(dx * dx + dz * dz);

    if (distance > 0.01) {
      const angle = Math.atan2(dx, dz);
      this.transform.setAngle(angle);
      this.targetTransform.setAngle(angle);
      this.group.quaternion.copy(this.transform.getRotation());
    }
  }

  update(deltaTime_s: number) {
    // Update animation mixer
    if (this.mixer) {
      this.mixer.update(deltaTime_s);
    }

    if (!this.isMoving) return;

    const distance = this.transform.distanceTo2D(this.targetTransform);

    if (distance < 0.1) {
      // Reached target
      this.transform.setXZ(this.targetTransform.getX(), this.targetTransform.getZ());
      this.group.position.set(this.transform.getX(), this.transform.getY(), this.transform.getZ());
      this.isMoving = false;

      // Switch to idle animation
      this.playAnimation('Idle', true);
      return;
    }

    // Move towards target
    const moveDistance = this.moveSpeed * deltaTime_s;
    const dx = this.targetTransform.getX() - this.transform.getX();
    const dz = this.targetTransform.getZ() - this.transform.getZ();
    const direction = new THREE.Vector2(dx, dz).normalize();

    const actualMove = Math.min(moveDistance, distance);
    const newX = this.transform.getX() + direction.x * actualMove;
    const newZ = this.transform.getZ() + direction.y * actualMove;

    this.transform.setXZ(newX, newZ);
    this.group.position.set(this.transform.getX(), this.transform.getY(), this.transform.getZ());
  }

  getObject3D(): THREE.Group {
    return this.group;
  }

  getTransform(): Transform {
    return this.transform.clone();
  }

  getPosition(): THREE.Vector3 {
    return this.transform.getPosition();
  }

  isCurrentlyMoving(): boolean {
    return this.isMoving;
  }
}
