# Avatar System Implementation Plan

## Key Insight

This system combines the visual appeal of **voxel art** with the flexibility of **skeletal animation**:

1. **Design once**: Create a single voxel character in MagicaVoxel
2. **Convert to polygons**: Rust converts voxels to optimized polygon mesh
3. **Animate with skeleton**: TypeScript binds the mesh to a skeleton with animations
4. **Customize per user**: Apply unique colors based on user's npub hash

**Why this works**: The voxel model becomes a **skinned mesh** where vertices are influenced by bone transformations. The blocky voxel aesthetic is preserved through flat shading and limited deformation.

**Memory efficient**: All users share the same skeleton and animation clips. Only the vertex colors differ per user.

## Architecture Overview

The avatar system uses a hybrid Rust/TypeScript architecture where **Rust handles all voxel data** and **TypeScript handles all animations**:

- **Rust (WASM)**:
  - Loads and processes voxel model data from MagicaVoxel (.vox format)
  - Generates optimized polygon mesh from voxels (greedy meshing)
  - Applies per-user color customization based on npub hash
  - Exports geometry via existing `GeometryData` struct (vertices, indices, normals, colors)

- **TypeScript (Three.js)**:
  - Loads skeleton and animations from glTF file
  - Creates SkinnedMesh from Rust-generated geometry
  - Binds geometry to skeleton for deformation
  - Manages AnimationMixer for playback (idle, walk, etc.)
  - Handles avatar movement and state transitions

- **Interface**: `GeometryData` struct (crates/world/src/lib.rs:34-71) passes pre-computed mesh data from Rust to TypeScript

### Design Approach

**Single base voxel model + Single skeleton + Multiple animations:**

1. **Base Model**: One voxel character designed in MagicaVoxel (16×32×16 voxels in T-pose)
2. **Skeleton**: One rigged skeleton with animations exported from Blender or Mixamo
3. **Rendering**: Voxel model converted to polygon mesh by Rust, then animated by TypeScript
4. **Customization**: Each user gets unique colors applied to the same base geometry
5. **Sharing**: All avatars share the same skeleton and animation clips (memory efficient)

## Data Flow

```
┌─────────────────────────────────────────────────────────────┐
│ Asset Creation (One-time)                                   │
├─────────────────────────────────────────────────────────────┤
│ MagicaVoxel → base_avatar.vox (16×32×16 voxels, T-pose)   │
│ Blender/Mixamo → avatar_skeleton.glb (skeleton + anims)    │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Runtime: Per-User Avatar Generation                         │
└─────────────────────────────────────────────────────────────┘
                            ↓
        ┌───────────────────────────────────┐
        │ RUST WASM (crates/world/)         │
        ├───────────────────────────────────┤
        │ 1. Load embedded .vox file        │
        │ 2. Parse voxel data structure     │
        │ 3. Generate mesh (greedy meshing) │
        │ 4. Customize colors (user hash)   │
        │ 5. Export GeometryData            │
        │    - vertices: Vec<f32>           │
        │    - indices: Vec<u32>            │
        │    - normals: Vec<f32>            │
        │    - colors: Vec<f32>             │
        └───────────────────────────────────┘
                            ↓
        ┌───────────────────────────────────┐
        │ TYPESCRIPT (packages/app/src/)    │
        ├───────────────────────────────────┤
        │ 1. Receive GeometryData from Rust │
        │ 2. Create BufferGeometry          │
        │ 3. Load skeleton from .glb        │
        │ 4. Create SkinnedMesh             │
        │ 5. Bind geometry to skeleton      │
        │ 6. Setup AnimationMixer           │
        │ 7. Play animations (idle, walk)   │
        └───────────────────────────────────┘
                            ↓
                    Three.js Renderer
                            ↓
                      User sees avatar
```

**Key Point**: Rust generates **static geometry** (the voxel mesh), TypeScript **animates** it using skeleton bones. The voxel model is converted to a polygon model that can be deformed by bones.

## Phase 1: Rust Voxel Processing

### 1.1 Voxel Data Structure

Create `crates/world/src/geometry/voxel_avatar.rs`:

```rust
use glam::{Vec3, Vec4};

/// Represents a single voxel in 3D space
#[derive(Clone, Copy, Debug)]
pub struct Voxel {
    pub position: Vec3,
    pub color: Vec4, // RGBA
    pub is_solid: bool,
}

/// Voxel model loaded from embedded data
pub struct VoxelModel {
    pub size: (u32, u32, u32), // width, height, depth
    pub voxels: Vec<Option<Voxel>>,
}

impl VoxelModel {
    /// Load voxel data from embedded .vox file
    pub fn from_vox_data(data: &[u8]) -> Result<Self, String> {
        // Parse .vox format (MagicaVoxel)
        // Use dot_vox crate or custom parser
        todo!("Implement .vox parser")
    }

    /// Get voxel at specific coordinates
    pub fn get(&self, x: u32, y: u32, z: u32) -> Option<&Voxel> {
        let idx = self.coord_to_index(x, y, z);
        self.voxels.get(idx)?.as_ref()
    }

    fn coord_to_index(&self, x: u32, y: u32, z: u32) -> usize {
        (z * self.size.0 * self.size.1 + y * self.size.0 + x) as usize
    }
}
```

### 1.2 Greedy Meshing Algorithm

Convert voxels to optimized mesh:

```rust
pub struct VoxelMesher {
    model: VoxelModel,
}

impl VoxelMesher {
    /// Convert voxels to mesh using greedy meshing
    pub fn generate_mesh(&self) -> MeshData {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut normals = Vec::new();
        let mut colors = Vec::new();

        // Greedy meshing: combine adjacent voxels into larger quads
        for axis in 0..3 {
            self.mesh_axis(axis, &mut vertices, &mut indices, &mut normals, &mut colors);
        }

        MeshData {
            vertices,
            indices,
            normals,
            colors,
        }
    }

    fn mesh_axis(
        &self,
        axis: u32,
        vertices: &mut Vec<f32>,
        indices: &mut Vec<u32>,
        normals: &mut Vec<f32>,
        colors: &mut Vec<f32>,
    ) {
        // Implementation of greedy meshing for one axis
        // Iterate through slices perpendicular to axis
        // Merge adjacent voxels of same color into quads
        todo!("Implement greedy meshing per axis")
    }
}

struct MeshData {
    vertices: Vec<f32>,
    indices: Vec<u32>,
    normals: Vec<f32>,
    colors: Vec<f32>,
}
```

### 1.3 Skeleton and Skinning Data

Define skeleton structure and vertex weights:

```rust
#[derive(Clone, Debug)]
pub struct Bone {
    pub name: String,
    pub parent: Option<usize>,
    pub position: Vec3,
    pub rotation: glam::Quat,
}

pub struct Skeleton {
    pub bones: Vec<Bone>,
}

impl Skeleton {
    /// Create humanoid skeleton with standard bone hierarchy
    pub fn humanoid() -> Self {
        Self {
            bones: vec![
                Bone { name: "root".into(), parent: None, position: Vec3::ZERO, rotation: glam::Quat::IDENTITY },
                Bone { name: "spine".into(), parent: Some(0), position: Vec3::new(0.0, 1.0, 0.0), rotation: glam::Quat::IDENTITY },
                Bone { name: "head".into(), parent: Some(1), position: Vec3::new(0.0, 2.0, 0.0), rotation: glam::Quat::IDENTITY },
                // Add more bones: arms, legs, etc.
            ],
        }
    }
}

/// Vertex weight for skinning
#[derive(Clone, Copy, Debug)]
pub struct VertexWeight {
    pub bone_indices: [u8; 4], // Up to 4 bones per vertex
    pub weights: [f32; 4], // Weights must sum to 1.0
}

pub struct SkinnedMesh {
    pub mesh: MeshData,
    pub skeleton: Skeleton,
    pub vertex_weights: Vec<VertexWeight>,
}

impl SkinnedMesh {
    /// Assign bone weights based on vertex proximity to bones
    pub fn auto_weight(mesh: MeshData, skeleton: &Skeleton) -> Self {
        let mut vertex_weights = Vec::with_capacity(mesh.vertices.len() / 3);

        // For each vertex, find closest bones and assign weights
        for i in (0..mesh.vertices.len()).step_by(3) {
            let vertex = Vec3::new(
                mesh.vertices[i],
                mesh.vertices[i + 1],
                mesh.vertices[i + 2],
            );

            let weight = Self::compute_weight(&vertex, skeleton);
            vertex_weights.push(weight);
        }

        Self {
            mesh,
            skeleton: skeleton.clone(),
            vertex_weights,
        }
    }

    fn compute_weight(vertex: &Vec3, skeleton: &Skeleton) -> VertexWeight {
        // Find 4 closest bones and compute weights based on distance
        todo!("Implement weight computation")
    }
}
```

### 1.4 WASM Interface

Expose avatar generation to JavaScript:

```rust
// In crates/world/src/lib.rs

#[wasm_bindgen]
pub struct AvatarEngine {
    base_model: VoxelModel,
    skeleton: Skeleton,
}

#[wasm_bindgen]
impl AvatarEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Load embedded voxel model data
        let vox_data = include_bytes!("../../assets/base_avatar.vox");
        let base_model = VoxelModel::from_vox_data(vox_data)
            .expect("Failed to load base avatar model");

        let skeleton = Skeleton::humanoid();

        Self {
            base_model,
            skeleton,
        }
    }

    /// Generate avatar mesh with user-specific customization
    #[wasm_bindgen]
    pub fn generate_avatar(&self, user_hash: String) -> GeometryData {
        // Convert voxels to mesh
        let mesher = VoxelMesher { model: self.base_model.clone() };
        let mut mesh = mesher.generate_mesh();

        // Apply user-specific colors
        self.apply_user_colors(&mut mesh.colors, &user_hash);

        GeometryData::new(
            mesh.vertices,
            mesh.indices,
            mesh.normals,
            mesh.colors,
        )
    }

    /// Get skeleton data as JSON
    #[wasm_bindgen]
    pub fn get_skeleton_data(&self) -> String {
        // Serialize skeleton to JSON for TypeScript
        serde_json::to_string(&self.skeleton).unwrap()
    }

    fn apply_user_colors(&self, colors: &mut Vec<f32>, user_hash: &str) {
        // Generate deterministic color palette from user hash
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        user_hash.hash(&mut hasher);
        let seed = hasher.finish();

        // Generate 3-5 colors and apply to color regions
        let palette = self.generate_palette(seed);

        // Modify colors based on palette
        // Keep voxel aesthetic with limited colors
        for i in (0..colors.len()).step_by(4) {
            let region = (i / 4) % palette.len();
            let palette_color = palette[region];

            colors[i] = palette_color.x;
            colors[i + 1] = palette_color.y;
            colors[i + 2] = palette_color.z;
            // Keep original alpha
        }
    }

    fn generate_palette(&self, seed: u64) -> Vec<Vec3> {
        // Generate pleasing color palette from seed
        // Use HSV color space for better control
        vec![
            Vec3::new(1.0, 0.5, 0.3), // Example color
            Vec3::new(0.3, 0.7, 1.0),
            Vec3::new(0.8, 0.2, 0.6),
        ]
    }
}
```

## Phase 2: TypeScript Animation System

### 2.1 Avatar Manager

Create `packages/app/src/services/AvatarManager.ts`:

```typescript
import * as THREE from 'three';
import { GeometryData } from '@crossworld/world';

interface AnimationClip {
  name: string;
  duration: number;
  tracks: THREE.KeyframeTrack[];
}

export class AvatarManager {
  private avatarEngine: any; // WASM AvatarEngine
  private skeletonData: any;
  private animationClips: Map<string, THREE.AnimationClip>;

  constructor(avatarEngine: any) {
    this.avatarEngine = avatarEngine;
    this.animationClips = new Map();
  }

  async init() {
    // Load skeleton data from Rust
    const skeletonJson = this.avatarEngine.get_skeleton_data();
    this.skeletonData = JSON.parse(skeletonJson);

    // Load animation data (could be from files or embedded)
    await this.loadAnimations();
  }

  private async loadAnimations() {
    // Load animation clips from JSON or glTF files
    // For now, create procedural animations
    this.animationClips.set('idle', this.createIdleAnimation());
    this.animationClips.set('walk', this.createWalkAnimation());
  }

  private createIdleAnimation(): THREE.AnimationClip {
    const tracks: THREE.KeyframeTrack[] = [];

    // Create subtle breathing animation
    const times = [0, 1, 2];
    const values = [0, 0.05, 0]; // Small up/down movement

    const spineTrack = new THREE.VectorKeyframeTrack(
      '.bones[spine].position',
      times,
      [0, values[0], 0, 0, values[1], 0, 0, values[2], 0]
    );

    tracks.push(spineTrack);

    return new THREE.AnimationClip('idle', 2, tracks);
  }

  private createWalkAnimation(): THREE.AnimationClip {
    // Create walk cycle animation
    const tracks: THREE.KeyframeTrack[] = [];

    // Leg movements, arm swings, etc.
    // TODO: Implement full walk cycle

    return new THREE.AnimationClip('walk', 1, tracks);
  }

  createAvatar(userHash: string): Avatar {
    // Generate geometry from Rust
    const geometryData: GeometryData = this.avatarEngine.generate_avatar(userHash);

    // Create Three.js geometry
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.Float32BufferAttribute(geometryData.vertices, 3));
    geometry.setAttribute('normal', new THREE.Float32BufferAttribute(geometryData.normals, 3));
    geometry.setAttribute('color', new THREE.Float32BufferAttribute(geometryData.colors, 4));
    geometry.setIndex(Array.from(geometryData.indices));

    // Create material with vertex colors
    const material = new THREE.MeshStandardMaterial({
      vertexColors: true,
      flatShading: true, // Maintain voxel aesthetic
    });

    const mesh = new THREE.Mesh(geometry, material);

    // Create skeleton from skeleton data
    const bones = this.createBonesFromData();
    const skeleton = new THREE.Skeleton(bones);

    // Attach skeleton to mesh
    mesh.bind(skeleton);

    // Create animation mixer
    const mixer = new THREE.AnimationMixer(mesh);

    return new Avatar(mesh, mixer, this.animationClips);
  }

  private createBonesFromData(): THREE.Bone[] {
    const bones: THREE.Bone[] = [];

    // Create Three.js bones from skeleton data
    for (const boneData of this.skeletonData.bones) {
      const bone = new THREE.Bone();
      bone.name = boneData.name;
      bone.position.set(
        boneData.position.x,
        boneData.position.y,
        boneData.position.z
      );
      bone.quaternion.set(
        boneData.rotation.x,
        boneData.rotation.y,
        boneData.rotation.z,
        boneData.rotation.w
      );
      bones.push(bone);
    }

    // Set up parent-child relationships
    for (let i = 0; i < this.skeletonData.bones.length; i++) {
      const parentIndex = this.skeletonData.bones[i].parent;
      if (parentIndex !== null) {
        bones[parentIndex].add(bones[i]);
      }
    }

    return bones;
  }
}

export class Avatar {
  public mesh: THREE.Mesh;
  private mixer: THREE.AnimationMixer;
  private currentAction: THREE.AnimationAction | null = null;
  private animations: Map<string, THREE.AnimationClip>;

  constructor(
    mesh: THREE.Mesh,
    mixer: THREE.AnimationMixer,
    animations: Map<string, THREE.AnimationClip>
  ) {
    this.mesh = mesh;
    this.mixer = mixer;
    this.animations = animations;
  }

  playAnimation(name: string, fadeTime_ms: number = 200) {
    const clip = this.animations.get(name);
    if (!clip) {
      console.warn(`Animation ${name} not found`);
      return;
    }

    const action = this.mixer.clipAction(clip);

    if (this.currentAction && this.currentAction !== action) {
      this.currentAction.fadeOut(fadeTime_ms / 1000);
    }

    action.reset().fadeIn(fadeTime_ms / 1000).play();
    this.currentAction = action;
  }

  update(deltaTime_ms: number) {
    this.mixer.update(deltaTime_ms / 1000);
  }

  dispose() {
    this.mesh.geometry.dispose();
    if (Array.isArray(this.mesh.material)) {
      this.mesh.material.forEach(m => m.dispose());
    } else {
      this.mesh.material.dispose();
    }
  }
}
```

### 2.2 Integration with Scene

Update `packages/app/src/renderer/scene.ts` to use avatars:

```typescript
import { AvatarManager } from '../services/AvatarManager';

export class Scene {
  private avatarManager: AvatarManager | null = null;
  private avatars: Map<string, Avatar> = new Map();

  async initAvatarSystem(avatarEngine: any) {
    this.avatarManager = new AvatarManager(avatarEngine);
    await this.avatarManager.init();
  }

  addPlayer(userId: string, position: THREE.Vector3) {
    if (!this.avatarManager) return;

    const avatar = this.avatarManager.createAvatar(userId);
    avatar.mesh.position.copy(position);

    this.scene.add(avatar.mesh);
    this.avatars.set(userId, avatar);

    // Start idle animation
    avatar.playAnimation('idle');
  }

  updatePlayerAnimation(userId: string, animationName: string) {
    const avatar = this.avatars.get(userId);
    if (avatar) {
      avatar.playAnimation(animationName);
    }
  }

  update(deltaTime_ms: number) {
    // Update all avatar animations
    for (const avatar of this.avatars.values()) {
      avatar.update(deltaTime_ms);
    }
  }
}
```

## Phase 3: Asset Creation Pipeline

### 3.1 Create Base Voxel Model

1. **Design in MagicaVoxel**:
   - Create humanoid character in T-pose
   - Size: 16x32x8 voxels (width x height x depth)
   - Use limited color palette (8-16 colors)
   - Keep limbs separable for rigging

2. **Export**:
   - Save as `.vox` file
   - Place in `crates/world/assets/base_avatar.vox`

3. **Alternative: Use Existing Model**:
   - Download from OpenGameArt, itch.io
   - Modify to fit project aesthetic

### 3.2 Animation Creation

**Option A: Procedural (Recommended for MVP)**
- Create animations programmatically in TypeScript
- Simple walk cycle, idle animation
- Fast iteration, no external tools needed

**Option B: Mixamo (Future Enhancement)**
- Export voxel model as `.obj`
- Upload to Mixamo for auto-rigging
- Download with animations as `.fbx`
- Convert to `.gltf` and extract animation data

**Option C: Blender (Full Control)**
- Import voxel model
- Manual rigging with armature
- Create custom animations
- Export as `.gltf` with animations

## Phase 4: Optimization

### 4.1 Instancing

For rendering many avatars efficiently:

```typescript
// Use InstancedMesh for avatars with same geometry
const instancedAvatars = new THREE.InstancedMesh(
  sharedGeometry,
  sharedMaterial,
  maxAvatarCount
);

// Update instance transforms
const matrix = new THREE.Matrix4();
matrix.setPosition(position);
instancedAvatars.setMatrixAt(index, matrix);
instancedAvatars.instanceMatrix.needsUpdate = true;
```

### 4.2 LOD (Level of Detail)

```typescript
const lod = new THREE.LOD();
lod.addLevel(highDetailAvatar.mesh, 0);
lod.addLevel(mediumDetailAvatar.mesh, 50);
lod.addLevel(lowDetailAvatar.mesh, 100);
```

### 4.3 Animation Sharing

Share animation clips between avatars to reduce memory:

```typescript
// All avatars share the same animation clips
// Only create unique mixers per avatar
```

## Implementation Checklist

### Milestone 1: Basic Voxel Rendering
- [ ] Create `voxel_avatar.rs` module
- [ ] Implement `VoxelModel` struct with embedded data support
- [ ] Implement greedy meshing algorithm
- [ ] Add `AvatarEngine` to WASM bindings
- [ ] Create test voxel model in MagicaVoxel
- [ ] Test rendering in Three.js

### Milestone 2: Skeleton System
- [ ] Define `Skeleton` and `Bone` structures
- [ ] Implement skeleton serialization to JSON
- [ ] Create TypeScript skeleton loader
- [ ] Add bone hierarchy to Three.js
- [ ] Verify bone positions match voxel model

### Milestone 3: Basic Animation
- [ ] Implement procedural idle animation
- [ ] Implement procedural walk animation
- [ ] Create `AvatarManager` class
- [ ] Test animation blending
- [ ] Add animation state machine

### Milestone 4: User Customization
- [ ] Implement hash-based color generation
- [ ] Apply user colors to voxel model
- [ ] Test with multiple users
- [ ] Ensure deterministic results

### Milestone 5: Performance Optimization
- [ ] Implement avatar instancing
- [ ] Add LOD system
- [ ] Profile rendering performance
- [ ] Optimize mesh generation
- [ ] Add animation culling for off-screen avatars

### Milestone 6: Advanced Features
- [ ] Add more animations (run, jump, etc.)
- [ ] Implement animation transitions
- [ ] Add accessories system
- [ ] Create animation blending
- [ ] Add facial expressions (if applicable)

## Technical Considerations

### Voxel Aesthetic Preservation

To maintain blocky voxel look during animation:
- Use flat shading in material
- Consider vertex snapping in shader (optional)
- Limit bone rotation angles to avoid distortion
- Use step interpolation for more rigid movement

### Memory Management

- **Rust**: Minimize allocations in hot paths
- **TypeScript**: Dispose geometries and materials properly
- **Shared Resources**: One base model, shared animations
- **Lazy Loading**: Load animations on-demand

### Coordinate Systems

- **MagicaVoxel**: Y-up, right-handed
- **Three.js**: Y-up, right-handed
- **Ensure consistency**: May need coordinate transformations

### File Formats

- **Voxel Data**: `.vox` (MagicaVoxel format)
- **Geometry**: Custom binary format via WASM
- **Animations**: JSON or glTF format
- **Skeleton**: JSON serialization

## Future Enhancements

1. **Animation Blending**: Smooth transitions between animations
2. **IK (Inverse Kinematics)**: For foot placement, look-at
3. **Ragdoll Physics**: For death/fall animations
4. **Customization Options**: Hats, accessories, skins
5. **Emotes**: User-triggered animations
6. **Facial Animations**: Expressions, lip-sync
7. **LOD Animations**: Simpler animations for distant avatars

## Project Integration

### Existing Code to Leverage

1. **GeometryData Structure** (`crates/world/src/lib.rs:34-71`)
   - Already defined and working
   - Used for terrain mesh generation
   - Perfect for avatar mesh data

2. **Existing Avatar Class** (`packages/app/src/renderer/avatar.ts`)
   - Already handles GLB loading and animations (lines 33-92)
   - AnimationMixer setup (lines 68-77)
   - Movement and state management (lines 139-191)
   - Can be refactored to accept Rust-generated geometry

3. **Voxel Reference Code** (`ref/world-proto/crates/geometry-engine/src/geometry/voxel.rs`)
   - BlockType enum with colors (lines 5-43)
   - VoxelChunk with greedy meshing (lines 45-253)
   - Face visibility checking (lines 181-195)
   - Can be adapted for avatar voxels

### Files to Create

```
crates/world/src/
├── avatar/
│   ├── mod.rs              # Module exports
│   ├── voxel_model.rs      # VoxelModel, Voxel, Palette structs
│   ├── mesher.rs           # Greedy meshing algorithm
│   └── manager.rs          # AvatarManager (user caching)
└── lib.rs                  # Add WASM bindings for avatar functions

packages/app/src/
├── renderer/
│   └── voxel-avatar.ts     # New VoxelAvatar class (or refactor avatar.ts)
└── services/
    └── avatar-service.ts   # High-level avatar management
```

## Development Workflow

### Using Rust MCP Tool for Documentation

During implementation, use the Rust documentation MCP server to look up crate documentation. This is especially useful for:

**Voxel File Parsing:**
```bash
# Look up dot_vox crate documentation
# The MCP tool will fetch docs from docs.rs
# Use for: parsing .vox file format, understanding VoxelData structures
```

**WASM Bindings:**
```bash
# Look up wasm-bindgen documentation
# Use for: exposing Rust functions to JavaScript, handling types
# Example: How to pass Vec<f32> from Rust to JavaScript
```

**Vector Math:**
```bash
# Look up glam crate documentation
# Use for: Vec3, Quat operations for skeleton bones
# Especially useful for coordinate transformations
```

**Serialization:**
```bash
# Look up serde and serde_json documentation
# Use for: serializing skeleton data to JSON for TypeScript
```

### Building and Testing

When modifying Rust avatar code:

```bash
# Build WASM module
bun run build:wasm

# For development (faster, unoptimized)
bun run build:wasm:dev

# Start dev server (auto-rebuilds on changes)
bun run dev
```

After building, verify the WASM package name:
```bash
# Check that packages/wasm/package.json has:
# "name": "@workspace/wasm"
```

## References

### Tools
- [MagicaVoxel](https://ephtracy.github.io/) - Free voxel editor
- [Blender](https://www.blender.org/) - Open-source 3D modeling
- [Mixamo](https://www.mixamo.com/) - Auto-rigging and animations

### Documentation
- [Three.js Animation System](https://threejs.org/docs/#manual/en/introduction/Animation-system)
- [Three.js SkinnedMesh](https://threejs.org/docs/#api/en/objects/SkinnedMesh)
- [Greedy Meshing Algorithm](https://0fps.net/2012/06/30/meshing-in-a-minecraft-game/)
- [wasm-bindgen Guide](https://rustwasm.github.io/wasm-bindgen/)

### Rust Crates (Use Rust MCP tool for docs)
- `dot_vox` - VOX file format parser
- `wasm-bindgen` - Rust/WASM/JS bindings
- `glam` - Vector math library
- `serde` - Serialization framework
