import * as THREE from 'three';

export class SceneManager {
  private scene: THREE.Scene;
  private camera: THREE.PerspectiveCamera;
  private renderer: THREE.WebGLRenderer;
  private geometryMesh: THREE.Mesh | null = null;

  constructor() {
    this.scene = new THREE.Scene();
    this.camera = new THREE.PerspectiveCamera(
      75,
      window.innerWidth / window.innerHeight,
      0.1,
      1000
    );
    this.renderer = new THREE.WebGLRenderer();
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
}
