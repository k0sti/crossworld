# Rendering Pipeline

## Overview

Crossworld uses a hybrid rendering approach combining GPU rendering via Three.js with CPU raytracing capabilities. The primary rendering path uses WebGL through Three.js for real-time performance.

**Key Components**:
- **Three.js Scene Management** - Camera, lights, objects
- **WebGL Rendering** - GPU-accelerated graphics
- **Shader System** - Custom materials for voxels
- **Post-Processing** - Effects pipeline
- **CPU Raytracer** - Experimental alternative renderer

## Three.js Integration

### Scene Structure

```
Scene
├── Camera (PerspectiveCamera)
├── Lighting
│   ├── DirectionalLight (Sun)
│   ├── AmbientLight
│   └── HemisphereLight
├── Voxel Terrain (Mesh)
│   ├── BufferGeometry (from WASM)
│   └── MeshStandardMaterial
├── Avatars (Group)
│   ├── Avatar Meshes
│   └── Name Labels
└── Sky/Environment
    └── Sky sphere or gradient
```

### Initialization

**Scene Setup** (`packages/app/src/renderer/scene.ts`):
```typescript
// Create scene
const scene = new THREE.Scene()

// Create camera
const camera = new THREE.PerspectiveCamera(
  75,                    // FOV
  window.innerWidth / window.innerHeight,  // Aspect
  0.1,                  // Near plane
  1000                  // Far plane
)

// Create renderer
const renderer = new THREE.WebGLRenderer({
  antialias: true,
  alpha: true
})
renderer.setSize(window.innerWidth, window.innerHeight)
renderer.setPixelRatio(window.devicePixelRatio)
renderer.shadowMap.enabled = true
renderer.shadowMap.type = THREE.PCFSoftShadowMap
```

### Render Loop

```typescript
function animate() {
  requestAnimationFrame(animate)

  // Update physics (if enabled)
  if (physicsBridge) {
    physicsBridge.step(deltaTime)
  }

  // Update avatars
  avatars.forEach(avatar => avatar.update(deltaTime))

  // Update camera controls
  cameraController.update(deltaTime)

  // Render scene
  renderer.render(scene, camera)
}
```

## Voxel Rendering

### Mesh Generation Pipeline

```
WASM Octree
   ↓
Face Culling (remove interior faces)
   ↓
Greedy Meshing (combine adjacent faces)
   ↓
Generate GeometryData
   ├── vertices: Float32Array
   ├── indices: Uint32Array
   ├── normals: Float32Array
   └── colors: Float32Array
   ↓
Transfer to TypeScript
   ↓
Create THREE.BufferGeometry
   ↓
Apply Material
   ↓
Add to Scene
```

### Geometry Creation

**From WASM GeometryData**:
```typescript
function createVoxelMesh(geometryData: GeometryData): THREE.Mesh {
  const geometry = new THREE.BufferGeometry()

  // Set vertex positions
  geometry.setAttribute(
    'position',
    new THREE.Float32BufferAttribute(geometryData.vertices, 3)
  )

  // Set normals for lighting
  geometry.setAttribute(
    'normal',
    new THREE.Float32BufferAttribute(geometryData.normals, 3)
  )

  // Set vertex colors
  geometry.setAttribute(
    'color',
    new THREE.Float32BufferAttribute(geometryData.colors, 3)
  )

  // Set indices for faces
  geometry.setIndex(
    new THREE.Uint32BufferAttribute(geometryData.indices, 1)
  )

  // Create material
  const material = new THREE.MeshStandardMaterial({
    vertexColors: true,
    flatShading: false,
    roughness: 0.8,
    metalness: 0.2
  })

  return new THREE.Mesh(geometry, material)
}
```

### Materials

**Voxel Terrain Material**:
```typescript
new THREE.MeshStandardMaterial({
  vertexColors: true,      // Use per-vertex colors from WASM
  flatShading: false,      // Smooth shading
  roughness: 0.8,          // Not very shiny
  metalness: 0.2,          // Slightly metallic
  side: THREE.DoubleSide   // Render both sides (optional)
})
```

**Voxel Avatar Material**:
```typescript
new THREE.MeshPhongMaterial({
  vertexColors: true,      // Use colors from palette
  flatShading: true,       // Blocky voxel aesthetic
  shininess: 30,           // Slight specular
  side: THREE.DoubleSide
})
```

### Mesh Updates

**Dynamic Voxel Editing**:
```typescript
// User modifies voxel
modifyVoxel(x, y, z, color)
  ↓
// WASM regenerates mesh for modified chunk
const newGeometry = wasmWorld.getChunkGeometry(chunkId)
  ↓
// Update Three.js mesh
mesh.geometry.dispose()  // Free old geometry
mesh.geometry = createGeometry(newGeometry)
  ↓
// Render automatically on next frame
```

## Lighting System

### Directional Light (Sun)

**Configuration**:
```typescript
const sunLight = new THREE.DirectionalLight(0xffffff, 1.0)
sunLight.position.set(50, 100, 50)
sunLight.castShadow = true

// Shadow map configuration
sunLight.shadow.mapSize.width = 2048
sunLight.shadow.mapSize.height = 2048
sunLight.shadow.camera.near = 0.5
sunLight.shadow.camera.far = 500
sunLight.shadow.camera.left = -100
sunLight.shadow.camera.right = 100
sunLight.shadow.camera.top = 100
sunLight.shadow.camera.bottom = -100
```

### Ambient Light

```typescript
const ambientLight = new THREE.AmbientLight(0x404040, 0.5)
scene.add(ambientLight)
```

### Hemisphere Light

```typescript
const hemisphereLight = new THREE.HemisphereLight(
  0x87CEEB,  // Sky color
  0x8B4513,  // Ground color
  0.6        // Intensity
)
scene.add(hemisphereLight)
```

### Shadow System

**Enable Shadows**:
```typescript
renderer.shadowMap.enabled = true
renderer.shadowMap.type = THREE.PCFSoftShadowMap

// For each object that casts shadows
mesh.castShadow = true

// For each object that receives shadows
ground.receiveShadow = true
```

## Camera System

### Perspective Camera

**Configuration**:
```typescript
const camera = new THREE.PerspectiveCamera(
  75,    // Vertical FOV in degrees
  aspect,  // Aspect ratio
  0.1,   // Near clipping plane
  1000   // Far clipping plane
)
```

### Camera Controls

**Third-Person Camera** (follows avatar):
```typescript
class CameraController {
  update(avatar: THREE.Object3D, deltaTime: number) {
    // Orbit around avatar
    const distance = 10
    const height = 5
    const targetPos = avatar.position.clone()

    this.camera.position.lerp(
      new THREE.Vector3(
        targetPos.x + distance * Math.cos(this.angle),
        targetPos.y + height,
        targetPos.z + distance * Math.sin(this.angle)
      ),
      0.1  // Smoothing factor
    )

    this.camera.lookAt(targetPos)
  }
}
```

**First-Person Camera** (avatar's viewpoint):
```typescript
// Camera at avatar head height
camera.position.copy(avatar.position)
camera.position.y += 1.6  // Eye height

// Camera looks in avatar's direction
camera.rotation.y = avatar.rotation.y
```

## Avatar Rendering

### GLB Avatar Loading

```typescript
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader'

const loader = new GLTFLoader()
loader.load(
  avatarUrl,
  (gltf) => {
    const avatar = gltf.scene
    avatar.traverse((child) => {
      if (child instanceof THREE.Mesh) {
        child.castShadow = true
        child.receiveShadow = true
      }
    })
    scene.add(avatar)
  }
)
```

### Voxel Avatar Rendering

```typescript
// Create from WASM-generated geometry
const avatarMesh = new THREE.Mesh(
  createGeometry(wasmAvatarData),
  new THREE.MeshPhongMaterial({
    vertexColors: true,
    flatShading: true
  })
)

avatarMesh.castShadow = true
avatarMesh.receiveShadow = true
scene.add(avatarMesh)
```

### Animation System

**Skeletal Animation** (GLB avatars):
```typescript
const mixer = new THREE.AnimationMixer(avatar)
const action = mixer.clipAction(gltf.animations[0])
action.play()

// Update in render loop
mixer.update(deltaTime)
```

## Post-Processing

### Effects Pipeline

```typescript
import { EffectComposer } from 'three/examples/jsm/postprocessing/EffectComposer'
import { RenderPass } from 'three/examples/jsm/postprocessing/RenderPass'
import { UnrealBloomPass } from 'three/examples/jsm/postprocessing/UnrealBloomPass'

// Create composer
const composer = new EffectComposer(renderer)

// Add render pass
const renderPass = new RenderPass(scene, camera)
composer.addPass(renderPass)

// Add bloom effect
const bloomPass = new UnrealBloomPass(
  new THREE.Vector2(window.innerWidth, window.innerHeight),
  1.5,  // Strength
  0.4,  // Radius
  0.85  // Threshold
)
composer.addPass(bloomPass)

// Render with effects
composer.render()
```

### Available Effects

- **Bloom** - Glow effect for bright areas
- **SSAO** - Screen-space ambient occlusion
- **Depth of Field** - Blur distant objects
- **Color Correction** - Tone mapping, gamma correction
- **Antialiasing** - FXAA, SMAA, TAA

## CPU Raytracer

### Experimental Renderer

**Location**: `crates/renderer/`

**Purpose**: Alternative rendering path for:
- Reference implementation
- Offline rendering
- High-quality screenshots
- Testing voxel visibility

**Status**: Stub implementation, not used in production

**Components**:
- `cpu_tracer.rs` - Software raytracing
- `gpu_tracer.rs` - GPU compute shader (future)
- `gl_tracer.rs` - OpenGL interop (future)

### CPU Tracer Architecture

```
Ray Generation
   ↓
For Each Pixel:
   ↓
   Ray → Octree Raycast
   ↓
   If Hit:
      ├── Material Lookup
      ├── Lighting Calculation
      ├── Shadow Ray
      └── Color Output
   ↓
   If Miss:
      └── Sky Color
```

**Not Currently Integrated**: The CPU raytracer is a standalone component not connected to the main rendering pipeline.

## Performance Optimization

### Geometry Optimization

**Face Culling**:
- Remove faces between adjacent solid voxels
- Only render exposed faces
- Reduces triangle count by 50-80%

**Greedy Meshing**:
- Combine adjacent coplanar faces
- Reduces draw calls
- Improves GPU performance

**Level of Detail (Future)**:
- Multiple mesh resolutions
- Switch based on distance
- Lower detail for distant chunks

### Rendering Optimization

**Frustum Culling**:
- Don't render objects outside camera view
- Automatic in Three.js

**Occlusion Culling** (Future):
- Don't render objects behind other objects
- Requires additional implementation

**Instancing** (Future):
- Render multiple identical objects efficiently
- Useful for vegetation, particles

### Draw Call Reduction

**Batching**:
- Combine multiple meshes into one
- Reduces CPU overhead
- Trade-off: harder to update individual parts

**Texture Atlasing**:
- Combine multiple textures into one
- Reduces texture switches
- Currently not used (vertex colors instead)

## Debug Rendering

### Wireframe Mode

```typescript
material.wireframe = true  // Show mesh structure
```

### Helper Objects

```typescript
// Axis helper (X=red, Y=green, Z=blue)
const axesHelper = new THREE.AxesHelper(5)
scene.add(axesHelper)

// Grid helper
const gridHelper = new THREE.GridHelper(100, 100)
scene.add(gridHelper)

// Bounding box helper
const box = new THREE.Box3Helper(mesh.geometry.boundingBox)
scene.add(box)
```

### Stats Display

```typescript
import Stats from 'three/examples/jsm/libs/stats.module'

const stats = new Stats()
document.body.appendChild(stats.dom)

// In render loop
stats.update()
```

## Related Documentation

- [overview.md](overview.md) - System architecture
- [voxel-system.md](voxel-system.md) - Voxel mesh generation
- [../features/avatar-system.md](../features/avatar-system.md) - Avatar rendering
- [../reference/materials.md](../reference/materials.md) - Material definitions
