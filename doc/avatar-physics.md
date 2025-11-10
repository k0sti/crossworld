# Avatar Physics Integration Plan

## Executive Summary

This document outlines the integration plan for incorporating the Rapier3D-based physics system (from `crates/physics`) with the existing avatar system. The goal is to enable realistic physics-based movement, collision detection, and interaction with the voxel world while maintaining smooth, responsive character control.

**Current State:**
- ✅ Physics system with Rapier3D integration complete (`crates/physics`)
- ✅ Avatar system with GLB and voxel rendering complete (`packages/app/src/renderer`)
- ❌ No physics integration - avatars use simple raycast-based ground following
- ❌ No collision detection with voxel world or other objects

**Target State:**
- ✅ Physics-based avatar movement with collision detection
- ✅ Integration with octacube voxel collision system
- ✅ Character controller with smooth, responsive controls
- ✅ Support for jumping, gravity, and terrain navigation

---

## 1. Current System Analysis

### 1.1 Physics System (Rapier3D)

**Location:** `crates/physics/`

**Key Components:**
- **PhysicsWorld**: Wraps Rapier pipeline, manages gravity and simulation
- **CubeObject**: High-level rigid body wrapper with three types:
  - Dynamic: Affected by forces/gravity
  - Kinematic: Programmatically controlled, affects others
  - Static: Immovable terrain/obstacles
- **VoxelColliderBuilder**: Generates collision geometry from octree voxels
  - Traverses octree structure
  - Detects exposed voxel faces
  - Creates compound collider from face rectangles

**API Capabilities:**
```rust
// Creation
CubeObject::new_kinematic(world, position)

// Collider attachment
attach_collider(world, collider)
set_cube(cube: Rc<Cube<i32>>)

// State queries
position(world) -> Vec3
rotation(world) -> Quat
velocity(world) -> Vec3

// Control
set_position(world, Vec3)
set_velocity(world, Vec3)
apply_force(world, Vec3)
apply_impulse(world, Vec3)
```

**Strengths:**
- Complete physics pipeline with Rapier3D
- Sophisticated voxel collision generation
- Supports kinematic bodies (ideal for character control)
- WASM-compatible

**Gaps for Avatar Integration:**
- No character controller implementation
- No ground detection/snapping
- No slope handling or step climbing
- No velocity smoothing for responsive controls

### 1.2 Avatar System

**Location:** `packages/app/src/renderer/`

**Key Components:**
- **BaseAvatar**: Abstract class with shared movement logic
  - Physics-based velocity (acceleration: 40 u/s², damping: 0.9)
  - Smooth rotation (15 rad/s turn speed)
  - Ground raycasting for height following
  - Teleportation support
- **Avatar (GLB)**: 3D model loader with skeletal animations
- **VoxelAvatar**: Voxel-based character rendering

**Current Movement System:**
```typescript
// Click-to-move with velocity-based physics
setTargetPosition(x: number, z: number)
  → Acceleration toward target
  → Velocity damping
  → Position update
  → Raycast for ground height

// Update loop (every frame)
update(deltaTime_s: number)
  → Smooth rotation
  → Velocity physics
  → Ground raycasting
  → Transform updates
```

**Input Methods:**
- Mouse: Click-to-move, CTRL+click teleport
- Gamepad: Left stick movement, RT sprint, A jump

**Strengths:**
- Smooth, responsive movement feel
- Multiple avatar types (GLB, voxel)
- Animation system for GLB avatars
- Network synchronization (Nostr protocol)

**Gaps:**
- Simple raycast-based ground following
- No collision with world geometry
- No obstacle avoidance
- Jump input ignored (no physics)
- Can walk through walls/objects

---

## 2. Integration Architecture

### 2.1 Hybrid Approach: Kinematic Character Controller

**Recommended Approach:**
Use kinematic rigid bodies for avatar physics, allowing programmatic control while enabling collision detection.

**Why Kinematic?**
- ✅ Direct velocity control for responsive feel
- ✅ Prevents physics glitches (sliding, tipping over)
- ✅ Enables collision detection without full dynamics
- ✅ Compatible with existing movement logic
- ✅ Standard approach for character controllers (Unreal, Unity, etc.)

### 2.2 Architecture Layers

```
┌─────────────────────────────────────────┐
│        Input Layer (TypeScript)          │
│  Mouse, Keyboard, Gamepad → Commands    │
└─────────────┬───────────────────────────┘
              │
┌─────────────▼───────────────────────────┐
│   Avatar Movement Logic (TypeScript)     │
│  BaseAvatar: Velocity, rotation, intent  │
└─────────────┬───────────────────────────┘
              │
┌─────────────▼───────────────────────────┐
│   Physics Bridge Layer (WASM/JS)         │
│  Convert movement intent → physics calls │
└─────────────┬───────────────────────────┘
              │
┌─────────────▼───────────────────────────┐
│    Physics System (Rust/Rapier)          │
│  CubeObject kinematic body + collider    │
│  CollisionWorld: voxel + object geometry │
└─────────────┬───────────────────────────┘
              │
┌─────────────▼───────────────────────────┐
│   Rendering Layer (Three.js)             │
│  Avatar mesh, animations, profile icon   │
└─────────────────────────────────────────┘
```

### 2.3 Component Responsibilities

**TypeScript (Avatar System):**
- Gather input from mouse/keyboard/gamepad
- Calculate movement intent (target direction, speed)
- Handle animations (walk/idle/jump)
- Render avatar mesh and profile icon
- Network synchronization (Nostr events)

**Rust (Physics System):**
- Manage PhysicsWorld simulation
- Kinematic body for avatar with capsule collider
- Collision detection with voxel world
- Ground detection via raycasting
- Slope/step handling
- Velocity application and collision response

**Bridge Layer:**
- Convert TypeScript movement commands to Rust physics calls
- Return physics state (position, on-ground, collisions) to TypeScript
- Handle coordinate system conversions if needed

---

## 3. Implementation Plan

### Phase 1: Character Controller Foundation (Week 1)

**Goal:** Create basic physics-based character controller in Rust

#### 1.1 Create CharacterController Module
**File:** `crates/physics/src/character_controller.rs`

**Structure:**
```rust
pub struct CharacterController {
    body_handle: RigidBodyHandle,
    collider_handle: ColliderHandle,

    // Configuration
    height: f32,          // Capsule height
    radius: f32,          // Capsule radius
    step_height: f32,     // Max climbable step
    max_slope: f32,       // Max walkable slope angle

    // State
    is_grounded: bool,
    ground_normal: Vec3,
    vertical_velocity: f32,
}

impl CharacterController {
    pub fn new(world: &mut PhysicsWorld, position: Vec3, height: f32, radius: f32) -> Self;

    // Movement
    pub fn move_with_velocity(&mut self, world: &mut PhysicsWorld, velocity: Vec3, dt: f32);
    pub fn jump(&mut self, world: &mut PhysicsWorld, impulse: f32);

    // Queries
    pub fn position(&self, world: &PhysicsWorld) -> Vec3;
    pub fn is_grounded(&self) -> bool;
    pub fn ground_normal(&self) -> Vec3;

    // Ground detection (internal)
    fn update_ground_state(&mut self, world: &PhysicsWorld);
}
```

**Implementation Tasks:**
- [ ] Create kinematic rigid body
- [ ] Attach capsule collider (better than box for smooth movement)
- [ ] Implement ground detection via raycasting (downward from body center)
- [ ] Add slope detection (reject movement on steep slopes)
- [ ] Implement step climbing (small upward adjustment)
- [ ] Handle vertical velocity (gravity + jumping)

**Ground Detection Algorithm:**
```rust
fn update_ground_state(&mut self, world: &PhysicsWorld) {
    // Raycast down from center, slightly longer than capsule bottom
    let ray_origin = self.position(world);
    let ray_dir = Vec3::NEG_Y;
    let max_distance = self.height / 2.0 + 0.1; // Small tolerance

    if let Some(hit) = world.cast_ray(ray_origin, ray_dir, max_distance, true) {
        self.is_grounded = true;
        self.ground_normal = hit.normal;
        self.vertical_velocity = 0.0; // Reset on landing
    } else {
        self.is_grounded = false;
        self.ground_normal = Vec3::Y;
    }
}
```

**Deliverables:**
- [ ] `crates/physics/src/character_controller.rs`
- [ ] Unit tests for ground detection
- [ ] Example program: `examples/character_movement.rs`

#### 1.2 Add Raycasting Support to PhysicsWorld
**File:** `crates/physics/src/world.rs`

**Add Methods:**
```rust
pub struct RaycastHit {
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
    pub collider: ColliderHandle,
}

impl PhysicsWorld {
    pub fn cast_ray(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        ignore_kinematic: bool,
    ) -> Option<RaycastHit>;
}
```

**Implementation:**
- [ ] Use `QueryPipeline::cast_ray()` from Rapier
- [ ] Filter kinematic bodies if requested
- [ ] Return hit information

#### 1.3 Testing
**File:** `crates/physics/examples/character_movement.rs`

**Test Scenarios:**
- [ ] Character spawns above ground and falls
- [ ] Character walks on flat terrain
- [ ] Character stops at walls
- [ ] Character climbs small steps
- [ ] Character slides down steep slopes
- [ ] Jump mechanics work correctly

---

### Phase 2: WASM Bridge Integration (Week 2)

**Goal:** Expose character controller to JavaScript/TypeScript

#### 2.1 Extend WasmPhysicsWorld
**File:** `crates/physics/src/wasm.rs`

**Add Methods:**
```rust
#[wasm_bindgen]
impl WasmPhysicsWorld {
    /// Create a character controller
    /// Returns controller ID
    pub fn createCharacter(
        &mut self,
        x: f32, y: f32, z: f32,
        height: f32,
        radius: f32,
    ) -> u32;

    /// Move character with horizontal velocity
    pub fn moveCharacter(
        &mut self,
        id: u32,
        vx: f32, vz: f32,
        dt: f32,
    );

    /// Make character jump
    pub fn jumpCharacter(&mut self, id: u32, impulse: f32);

    /// Get character state
    pub fn getCharacterPosition(&self, id: u32) -> Vec<f32>;
    pub fn isObjectGrounded(&self, id: u32) -> bool;
    pub fn getObjectGroundNormal(&self, id: u32) -> Vec<f32>;

    /// Remove character
    pub fn removeCharacter(&mut self, id: u32);
}
```

**Internal Storage:**
```rust
struct WasmPhysicsWorld {
    world: RefCell<PhysicsWorld>,
    objects: RefCell<HashMap<u32, CubeObject>>,
    characters: RefCell<HashMap<u32, CharacterController>>, // NEW
    next_id: RefCell<u32>,
}
```

**Implementation Tasks:**
- [ ] Store CharacterController instances in HashMap
- [ ] Implement all WASM methods
- [ ] Handle error cases (invalid IDs)
- [ ] Add TypeScript type definitions

#### 2.2 TypeScript Bindings
**File:** `packages/app/src/physics/physics-bridge.ts`

**Create Wrapper:**
```typescript
export interface CharacterControllerHandle {
  id: number;
}

export interface CharacterState {
  position: THREE.Vector3;
  isGrounded: boolean;
  groundNormal: THREE.Vector3;
}

export class PhysicsBridge {
  private wasmPhysics: WasmPhysicsWorld;

  createCharacter(
    position: THREE.Vector3,
    height: number,
    radius: number
  ): CharacterControllerHandle;

  moveCharacter(
    handle: CharacterControllerHandle,
    velocity: THREE.Vector2,
    deltaTime: number
  ): void;

  jumpCharacter(handle: CharacterControllerHandle, impulse: number): void;

  getCharacterState(handle: CharacterControllerHandle): CharacterState;

  removeCharacter(handle: CharacterControllerHandle): void;

  step(deltaTime: number): void;
}
```

**Implementation Tasks:**
- [ ] Initialize WASM module
- [ ] Convert between Three.js and Rapier coordinate systems
- [ ] Handle Vector3/Vector2 conversions
- [ ] Add error handling

**Deliverables:**
- [ ] Updated `crates/physics/src/wasm.rs`
- [ ] New `packages/app/src/physics/physics-bridge.ts`
- [ ] TypeScript type definitions
- [ ] Integration tests

---

### Phase 3: Avatar Integration (Week 3)

**Goal:** Integrate physics character controller with existing avatar system

#### 3.1 Modify BaseAvatar
**File:** `packages/app/src/renderer/base-avatar.ts`

**Add Physics Integration:**
```typescript
abstract class BaseAvatar implements IAvatar {
  // Existing fields...
  protected group: THREE.Group;
  protected transform: Transform;
  protected velocity: THREE.Vector2;

  // NEW: Physics integration
  protected physicsHandle: CharacterControllerHandle | null = null;
  protected physicsBridge: PhysicsBridge | null = null;
  protected usePhysics: boolean = false;

  constructor(physicsBridge?: PhysicsBridge) {
    // ...
    if (physicsBridge) {
      this.physicsBridge = physicsBridge;
      this.usePhysics = true;
      this.initPhysics();
    }
  }

  protected initPhysics(): void {
    if (!this.physicsBridge) return;

    const position = this.transform.position;
    const height = 1.8; // Average human height
    const radius = 0.3; // Capsule radius

    this.physicsHandle = this.physicsBridge.createCharacter(
      position,
      height,
      radius
    );
  }

  update(deltaTime_s: number): void {
    if (this.usePhysics && this.physicsHandle && this.physicsBridge) {
      this.updateWithPhysics(deltaTime_s);
    } else {
      this.updateWithoutPhysics(deltaTime_s); // Existing logic
    }
  }

  protected updateWithPhysics(deltaTime: number): void {
    // 1. Calculate desired velocity from input/target
    const desiredVelocity = this.calculateDesiredVelocity(deltaTime);

    // 2. Apply velocity to physics character
    this.physicsBridge!.moveCharacter(
      this.physicsHandle!,
      desiredVelocity,
      deltaTime
    );

    // 3. Read back physics state
    const state = this.physicsBridge!.getCharacterState(this.physicsHandle!);

    // 4. Update transform and rendering
    this.transform.setPosition(state.position);
    this.updateRotation(deltaTime);
    this.updateAnimationState(state.isGrounded);

    // 5. Update Three.js group
    this.group.position.copy(state.position);
    this.group.quaternion.copy(this.transform.quaternion);
  }

  protected calculateDesiredVelocity(deltaTime: number): THREE.Vector2 {
    // Use existing velocity calculation logic
    const direction = new THREE.Vector2(
      this.targetTransform.position.x - this.transform.position.x,
      this.targetTransform.position.z - this.transform.position.z
    );

    if (direction.length() < 0.1) {
      this.isMoving = false;
      return new THREE.Vector2(0, 0);
    }

    direction.normalize();
    const speed = this.isRunning ? this.baseMoveSpeed * 2.0 : this.baseMoveSpeed;
    return direction.multiplyScalar(speed);
  }

  public jump(): void {
    if (!this.usePhysics || !this.physicsHandle || !this.physicsBridge) return;

    const state = this.physicsBridge.getCharacterState(this.physicsHandle);
    if (state.isGrounded) {
      this.physicsBridge.jumpCharacter(this.physicsHandle, 5.0); // Jump strength
      this.onJump(); // Trigger jump animation
    }
  }

  protected abstract onJump(): void; // Subclasses implement animation

  dispose(): void {
    // Cleanup physics
    if (this.physicsHandle && this.physicsBridge) {
      this.physicsBridge.removeCharacter(this.physicsHandle);
    }

    // Existing cleanup...
  }
}
```

**Implementation Tasks:**
- [ ] Add optional physics constructor parameter
- [ ] Implement `initPhysics()` method
- [ ] Split `update()` into physics/non-physics paths
- [ ] Add `jump()` method with ground check
- [ ] Update collision raycasting logic
- [ ] Ensure backward compatibility (physics optional)

#### 3.2 Update Avatar Subclasses
**Files:** `packages/app/src/renderer/avatar.ts`, `voxel-avatar.ts`

**Add Jump Animations:**
```typescript
// In Avatar (GLB)
protected onJump(): void {
  // Trigger jump animation if available
  if (this.animations.has('jump')) {
    this.playAnimation('jump', false); // Play once
  }
}

protected onStartMoving(): void {
  this.playAnimation('walk', true);
}

protected onStopMoving(): void {
  this.playAnimation('idle', true);
}
```

```typescript
// In VoxelAvatar
protected onJump(): void {
  // Simple scale animation for voxel characters
  gsap.to(this.group.scale, {
    y: 1.2,
    duration: 0.15,
    yoyo: true,
    repeat: 1,
    ease: 'power2.out'
  });
}
```

**Implementation Tasks:**
- [ ] Implement `onJump()` in both subclasses
- [ ] Update animation state machine
- [ ] Test jump animations

#### 3.3 Connect Input Systems
**File:** `packages/app/src/input/gamepad-controller.ts`

**Enable Jump Button:**
```typescript
class GamepadController {
  update(avatar: IAvatar, deltaTime: number): void {
    const gamepad = navigator.getGamepads()[0];
    if (!gamepad) return;

    // Existing movement code...

    // NEW: Jump on A button (button 0)
    if (gamepad.buttons[0]?.pressed) {
      avatar.jump();
    }
  }
}
```

**File:** `packages/app/src/renderer/scene.ts`

**Add Keyboard Jump:**
```typescript
// In input handler
document.addEventListener('keydown', (e) => {
  if (e.code === 'Space' && this.localAvatar) {
    this.localAvatar.jump();
  }
});
```

**Implementation Tasks:**
- [ ] Wire up Space key to jump
- [ ] Wire up gamepad A button
- [ ] Add jump cooldown (prevent spam)
- [ ] Visual feedback for jump

**Deliverables:**
- [ ] Updated `base-avatar.ts` with physics integration
- [ ] Jump animations in `avatar.ts` and `voxel-avatar.ts`
- [ ] Connected input systems
- [ ] Integration tests

---

### Phase 4: World Collision Integration (Week 4)

**Goal:** Enable collision between avatars and voxel world

#### 4.1 Generate World Collision Geometry
**File:** `packages/app/src/world/world-physics.ts`

**Create World Physics Manager:**
```typescript
export class WorldPhysics {
  private physicsBridge: PhysicsBridge;
  private chunkColliders: Map<string, number> = new Map();

  constructor(physicsBridge: PhysicsBridge) {
    this.physicsBridge = physicsBridge;
  }

  addChunkCollision(chunk: Chunk): void {
    // Generate collision geometry from chunk voxels
    const colliderData = this.generateChunkCollider(chunk);
    const colliderId = this.physicsBridge.addVoxelCollider(colliderData);
    this.chunkColliders.set(chunk.id, colliderId);
  }

  removeChunkCollision(chunkId: string): void {
    const colliderId = this.chunkColliders.get(chunkId);
    if (colliderId !== undefined) {
      this.physicsBridge.removeCollider(colliderId);
      this.chunkColliders.delete(chunkId);
    }
  }

  private generateChunkCollider(chunk: Chunk): VoxelColliderData {
    // Convert chunk CSM data to collision geometry
    // Use existing VoxelColliderBuilder from Rust
    return {
      csm: chunk.csm, // Compact sparse matrix
      depth: 5,       // Collision detail level
      isStatic: true
    };
  }
}
```

**Implementation Tasks:**
- [ ] Create WorldPhysics class
- [ ] Hook into chunk loading system
- [ ] Generate colliders when chunks load
- [ ] Remove colliders when chunks unload
- [ ] Optimize collision geometry (LOD, simplification)

#### 4.2 Integrate with Chunk System
**File:** `packages/app/src/world/chunk-manager.ts`

**Add Physics Callbacks:**
```typescript
class ChunkManager {
  private worldPhysics: WorldPhysics;

  onChunkLoaded(chunk: Chunk): void {
    // Existing render setup...

    // NEW: Add physics collision
    this.worldPhysics.addChunkCollision(chunk);
  }

  onChunkUnloaded(chunk: Chunk): void {
    // Existing cleanup...

    // NEW: Remove physics collision
    this.worldPhysics.removeChunkCollision(chunk.id);
  }
}
```

**Implementation Tasks:**
- [ ] Initialize WorldPhysics in Scene
- [ ] Connect to chunk loading events
- [ ] Handle chunk updates (terrain modification)
- [ ] Performance profiling

#### 4.3 Collision Response Tuning
**File:** `crates/physics/src/character_controller.rs`

**Add Configuration:**
```rust
pub struct CharacterControllerConfig {
    pub height: f32,
    pub radius: f32,
    pub step_height: f32,          // Max climbable step (e.g., 0.5m)
    pub max_slope_angle: f32,      // Max walkable slope (e.g., 45°)
    pub gravity: f32,              // Downward acceleration (e.g., 9.8)
    pub jump_impulse: f32,         // Jump strength (e.g., 5.0)
    pub ground_check_distance: f32, // Raycast length (e.g., 0.1)
}

impl Default for CharacterControllerConfig {
    fn default() -> Self {
        Self {
            height: 1.8,
            radius: 0.3,
            step_height: 0.5,
            max_slope_angle: 45.0,
            gravity: 9.8,
            jump_impulse: 5.0,
            ground_check_distance: 0.1,
        }
    }
}
```

**Tuning Parameters:**
- [ ] Test different capsule sizes
- [ ] Adjust step height for voxel blocks
- [ ] Tune slope angles for natural feel
- [ ] Balance jump strength with gravity
- [ ] Ground check tolerance

**Deliverables:**
- [ ] WorldPhysics system
- [ ] Chunk collision integration
- [ ] Tuned controller parameters
- [ ] Performance benchmarks

---

### Phase 5: Polish and Optimization (Week 5)

**Goal:** Refine feel, fix bugs, optimize performance

#### 5.1 Movement Feel Tuning
- [ ] Acceleration curves (responsive vs. realistic)
- [ ] Deceleration smoothing
- [ ] Turn speed adjustment
- [ ] Sprint transition smoothness
- [ ] Jump arc tuning

#### 5.2 Edge Case Handling
- [ ] Spawning in walls (teleport to safe position)
- [ ] Falling off world (respawn logic)
- [ ] Stuck detection (auto-unstuck)
- [ ] Collision with other avatars
- [ ] Moving platform support (future)

#### 5.3 Performance Optimization
- [ ] Profile physics step time
- [ ] Optimize voxel collider complexity
- [ ] Batch physics updates
- [ ] LOD for distant collision geometry
- [ ] Spatial partitioning

#### 5.4 Debug Visualization
**File:** `packages/app/src/debug/physics-debug.ts`

**Add Debug Overlays:**
```typescript
class PhysicsDebugRenderer {
  showColliders: boolean = false;
  showRaycasts: boolean = false;
  showVelocity: boolean = false;

  render(scene: THREE.Scene): void {
    if (this.showColliders) {
      // Draw capsule wireframes for characters
    }
    if (this.showRaycasts) {
      // Draw ground check rays
    }
    if (this.showVelocity) {
      // Draw velocity vectors
    }
  }
}
```

**Implementation Tasks:**
- [ ] Capsule collider visualization
- [ ] Ground raycast display
- [ ] Velocity vector arrows
- [ ] Collision point markers
- [ ] Toggle UI controls

#### 5.5 Testing
- [ ] Unit tests for CharacterController
- [ ] Integration tests for avatar movement
- [ ] Performance benchmarks
- [ ] User acceptance testing
- [ ] Network synchronization validation

**Deliverables:**
- [ ] Tuned movement parameters
- [ ] Bug fixes
- [ ] Performance optimizations
- [ ] Debug tools
- [ ] Test coverage

---

## 4. Technical Challenges and Solutions

### Challenge 1: Coordinate System Conversion

**Problem:**
- Rapier uses right-handed Y-up coordinate system
- Three.js uses right-handed Y-up (compatible!)
- Need to verify no conversion needed

**Solution:**
- Add unit tests to verify coordinate alignment
- Document coordinate conventions
- Create helper functions if conversion needed

### Challenge 2: WASM Performance

**Problem:**
- Frequent JS ↔ WASM boundary crossings
- Vector data copying overhead

**Solution:**
- Batch physics updates (update all characters in one call)
- Use shared memory for state data (WASM memory views)
- Profile and optimize hot paths

**Optimization Example:**
```rust
// Instead of: getCharacterPosition() per character per frame
// Do: updateAllCharacters() → returns packed array

#[wasm_bindgen]
impl WasmPhysicsWorld {
    pub fn updateAllCharacters(&mut self, dt: f32) -> Vec<f32> {
        // Returns flat array: [id, x, y, z, grounded, id, x, y, z, grounded, ...]
        // Single allocation, single crossing
    }
}
```

### Challenge 3: Animation Synchronization

**Problem:**
- Physics state updates at variable rate
- Animations need smooth transitions
- Jump animations need timing coordination

**Solution:**
- Use animation state machine
- Blend animations based on physics state
- Add animation events (e.g., land callback)

### Challenge 4: Network Synchronization

**Problem:**
- Physics simulation is deterministic locally
- Need to sync state across network
- Latency compensation

**Solution:**
- Send physics state in Nostr events (position, velocity, grounded)
- Client-side prediction for local avatar
- Interpolation for remote avatars
- Existing AvatarStateService handles this

### Challenge 5: Voxel Collision Complexity

**Problem:**
- High-detail voxel geometry = many colliders
- Performance impact on collision detection
- LOD for distant chunks

**Solution:**
- Use `max_depth` parameter in VoxelColliderBuilder
- Simplify collision geometry (merge adjacent faces)
- Only load collision for nearby chunks
- Profile and tune depth parameter (recommended: 3-5)

---

## 5. API Design

### 5.1 Rust API (CharacterController)

```rust
// In crates/physics/src/character_controller.rs

pub struct CharacterControllerConfig {
    pub height: f32,
    pub radius: f32,
    pub step_height: f32,
    pub max_slope_angle: f32,
    pub gravity: f32,
    pub jump_impulse: f32,
    pub ground_check_distance: f32,
}

pub struct CharacterController {
    body_handle: RigidBodyHandle,
    collider_handle: ColliderHandle,
    config: CharacterControllerConfig,
    is_grounded: bool,
    ground_normal: Vec3,
    vertical_velocity: f32,
}

impl CharacterController {
    pub fn new(
        world: &mut PhysicsWorld,
        position: Vec3,
        config: CharacterControllerConfig,
    ) -> Self;

    // Movement
    pub fn move_with_velocity(
        &mut self,
        world: &mut PhysicsWorld,
        horizontal_velocity: Vec3,
        dt: f32,
    );

    pub fn jump(&mut self, world: &mut PhysicsWorld);

    // State queries
    pub fn position(&self, world: &PhysicsWorld) -> Vec3;
    pub fn rotation(&self, world: &PhysicsWorld) -> Quat;
    pub fn is_grounded(&self) -> bool;
    pub fn ground_normal(&self) -> Vec3;
    pub fn velocity(&self, world: &PhysicsWorld) -> Vec3;

    // Configuration
    pub fn set_height(&mut self, world: &mut PhysicsWorld, height: f32);
    pub fn set_radius(&mut self, world: &mut PhysicsWorld, radius: f32);

    // Lifecycle
    pub fn destroy(self, world: &mut PhysicsWorld);
}
```

### 5.2 WASM API

```rust
// In crates/physics/src/wasm.rs

#[wasm_bindgen]
impl WasmPhysicsWorld {
    // Character management
    pub fn createCharacter(
        &mut self,
        x: f32, y: f32, z: f32,
        height: f32,
        radius: f32,
    ) -> u32;

    pub fn removeCharacter(&mut self, id: u32);

    // Movement
    pub fn moveCharacter(
        &mut self,
        id: u32,
        vx: f32, vz: f32,
        dt: f32,
    );

    pub fn jumpCharacter(&mut self, id: u32);

    // State queries
    pub fn getCharacterPosition(&self, id: u32) -> Vec<f32>; // [x, y, z]
    pub fn getCharacterRotation(&self, id: u32) -> Vec<f32>; // [x, y, z, w]
    pub fn getCharacterVelocity(&self, id: u32) -> Vec<f32>; // [x, y, z]
    pub fn isObjectGrounded(&self, id: u32) -> bool;
    pub fn getObjectGroundNormal(&self, id: u32) -> Vec<f32>; // [x, y, z]

    // Batch operations (performance optimization)
    pub fn updateAllCharacters(&mut self, dt: f32) -> Vec<f32>;
    // Returns: [id, x, y, z, grounded, nx, ny, nz, ...]
}
```

### 5.3 TypeScript API

```typescript
// In packages/app/src/physics/physics-bridge.ts

export interface CharacterControllerHandle {
  id: number;
}

export interface CharacterState {
  position: THREE.Vector3;
  rotation: THREE.Quaternion;
  velocity: THREE.Vector3;
  isGrounded: boolean;
  groundNormal: THREE.Vector3;
}

export interface CharacterConfig {
  height: number;
  radius: number;
  stepHeight?: number;
  maxSlopeAngle?: number;
  gravity?: number;
  jumpImpulse?: number;
}

export class PhysicsBridge {
  private wasmPhysics: WasmPhysicsWorld;

  constructor();

  // Character management
  createCharacter(
    position: THREE.Vector3,
    config: CharacterConfig
  ): CharacterControllerHandle;

  removeCharacter(handle: CharacterControllerHandle): void;

  // Movement
  moveCharacter(
    handle: CharacterControllerHandle,
    velocity: THREE.Vector2,
    deltaTime: number
  ): void;

  jumpCharacter(handle: CharacterControllerHandle): void;

  // State queries
  getCharacterState(handle: CharacterControllerHandle): CharacterState;

  // World management
  addVoxelCollider(csmData: Uint8Array, depth: number): number;
  removeCollider(id: number): void;

  // Simulation
  step(deltaTime: number): void;

  // Batch operations
  updateAllCharacters(deltaTime: number): Map<number, CharacterState>;
}
```

### 5.4 Avatar API Extension

```typescript
// In packages/app/src/renderer/base-avatar.ts

interface IAvatar {
  // Existing methods...
  getObject3D(): THREE.Group;
  getTransform(): Transform;
  setTargetPosition(x: number, z: number): void;
  teleportTo(x: number, z: number, animationType: TeleportAnimationType): void;
  update(deltaTime_s: number): void;
  dispose(): void;

  // NEW: Physics methods
  jump(): void;
  isGrounded(): boolean;
  getVelocity(): THREE.Vector3;
  setPhysicsEnabled(enabled: boolean): void;
}
```

---

## 6. Testing Strategy

### 6.1 Unit Tests (Rust)

**File:** `crates/physics/tests/character_controller_tests.rs`

```rust
#[test]
fn test_character_spawns_and_falls() {
    let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
    let mut controller = CharacterController::new(
        &mut world,
        Vec3::new(0.0, 10.0, 0.0),
        CharacterControllerConfig::default(),
    );

    // Character should not be grounded initially
    assert!(!controller.is_grounded());

    // Simulate for 1 second
    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }

    // Character should have fallen
    assert!(controller.position(&world).y < 5.0);
}

#[test]
fn test_character_ground_detection() { /* ... */ }

#[test]
fn test_character_collision_with_wall() { /* ... */ }

#[test]
fn test_character_step_climbing() { /* ... */ }

#[test]
fn test_character_jump() { /* ... */ }
```

### 6.2 Integration Tests (TypeScript)

**File:** `packages/app/tests/avatar-physics.test.ts`

```typescript
describe('Avatar Physics Integration', () => {
  let physicsBridge: PhysicsBridge;
  let avatar: BaseAvatar;

  beforeEach(() => {
    physicsBridge = new PhysicsBridge();
    avatar = new Avatar(physicsBridge);
  });

  test('avatar spawns with physics enabled', () => {
    expect(avatar.isGrounded()).toBe(false);
  });

  test('avatar falls and lands on ground', async () => {
    // Simulate physics for 1 second
    for (let i = 0; i < 60; i++) {
      physicsBridge.step(1 / 60);
      avatar.update(1 / 60);
    }

    expect(avatar.isGrounded()).toBe(true);
  });

  test('avatar can jump', () => {
    avatar.jump();
    physicsBridge.step(0.1);
    expect(avatar.getVelocity().y).toBeGreaterThan(0);
  });

  test('avatar collides with walls', () => { /* ... */ });

  test('avatar climbs steps', () => { /* ... */ });
});
```

### 6.3 Manual Testing Checklist

- [ ] Character spawns correctly
- [ ] Character falls with gravity
- [ ] Character walks on flat ground
- [ ] Character stops at walls
- [ ] Character climbs small steps (< step_height)
- [ ] Character cannot climb tall walls
- [ ] Character slides down steep slopes
- [ ] Jump feels responsive
- [ ] Jump works only when grounded
- [ ] Animation syncs with movement state
- [ ] Network sync works (position updates)
- [ ] No jittering or stuttering
- [ ] Performance is acceptable (60 FPS)
- [ ] Works with both GLB and voxel avatars
- [ ] Gamepad controls work
- [ ] Keyboard controls work
- [ ] Mouse click-to-move works

---

## 7. Performance Considerations

### 7.1 Target Metrics

- **Physics step time**: < 5ms per frame (60 FPS)
- **Character update time**: < 1ms per character
- **Collision query time**: < 0.5ms per character
- **WASM boundary crossing**: < 100 per frame total
- **Memory usage**: < 10MB for physics system

### 7.2 Optimization Strategies

#### Batch Updates
```typescript
// Instead of:
for (const avatar of avatars) {
  physicsBridge.moveCharacter(avatar.handle, avatar.velocity, dt);
  const state = physicsBridge.getCharacterState(avatar.handle);
}

// Do:
const states = physicsBridge.updateAllCharacters(dt);
for (const avatar of avatars) {
  const state = states.get(avatar.handle.id);
  avatar.applyPhysicsState(state);
}
```

#### Collision LOD
```rust
// Near characters: High detail
let depth = if distance_to_camera < 20.0 { 5 } else { 3 };
VoxelColliderBuilder::from_cube(&cube, depth)
```

#### Spatial Partitioning
- Only load collision for chunks near characters
- Unload distant chunk colliders
- Use Rapier's broad-phase optimizations

#### Fixed Timestep
```typescript
// Physics at fixed 60 Hz, rendering at variable rate
const PHYSICS_DT = 1 / 60;
let accumulator = 0;

function update(deltaTime: number) {
  accumulator += deltaTime;

  while (accumulator >= PHYSICS_DT) {
    physicsBridge.step(PHYSICS_DT);
    accumulator -= PHYSICS_DT;
  }

  // Render with interpolation
  const alpha = accumulator / PHYSICS_DT;
  renderWithInterpolation(alpha);
}
```

---

## 8. Future Enhancements

### Phase 6: Advanced Features (Post-MVP)

#### 8.1 Swimming and Water Physics
- Buoyancy forces
- Water drag
- Surface detection
- Swimming animations

#### 8.2 Climbing System
- Ladder detection
- Climbing animation
- Hand IK positioning

#### 8.3 Vehicle Physics
- Rideable vehicles
- Physics-based driving
- Mount/dismount system

#### 8.4 Ragdoll Physics
- Death animations
- Physics-based falls
- Procedural reactions

#### 8.5 Advanced Movement
- Wall running
- Ledge grabbing
- Vaulting
- Sliding

#### 8.6 Multiplayer Physics
- Server-authoritative physics
- Client-side prediction
- Lag compensation
- Rollback netcode

---

## 9. Success Criteria

### Minimum Viable Product (MVP)

- [x] Physics system integrated with avatar system
- [ ] Characters have collision with voxel world
- [ ] Smooth, responsive movement feel
- [ ] Jump mechanics work reliably
- [ ] Ground detection is accurate
- [ ] Performance meets target metrics
- [ ] Works with existing avatar types (GLB, voxel)
- [ ] Backward compatible (physics optional)

### Definition of Done

- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Manual testing checklist complete
- [ ] Performance benchmarks meet targets
- [ ] Code reviewed and approved
- [ ] Documentation updated
- [ ] User testing feedback addressed
- [ ] No critical bugs

---

## 10. Timeline and Milestones

| Week | Phase | Deliverable |
|------|-------|-------------|
| 1 | Foundation | CharacterController in Rust, raycasting, examples |
| 2 | WASM Bridge | JavaScript bindings, TypeScript types |
| 3 | Avatar Integration | BaseAvatar physics mode, jump controls |
| 4 | World Collision | WorldPhysics system, chunk integration |
| 5 | Polish | Tuning, optimization, debug tools, testing |

**Total Estimated Time:** 5 weeks (1 developer)

---

## 11. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Performance issues (WASM overhead) | Medium | High | Batch operations, profiling, optimization |
| Collision complexity (voxel detail) | Medium | Medium | LOD system, depth tuning |
| Movement feel not responsive | Low | High | Early prototype, iteration |
| Integration with existing code | Low | Medium | Optional physics flag, backward compatibility |
| Rapier WASM compatibility issues | Low | High | Early testing, fallback options |
| Network sync problems | Medium | Medium | Use existing AvatarStateService |

---

## 12. Conclusion

This plan provides a comprehensive roadmap for integrating the Rapier3D physics system with the existing avatar system in Crossworld. The phased approach allows for incremental development and testing, with each phase building on the previous one.

**Key Technical Decisions:**
1. **Kinematic character controller** for responsive feel
2. **Capsule collider** for smooth movement
3. **WASM bridge** for JavaScript integration
4. **Optional physics** for backward compatibility
5. **Batch updates** for performance

**Next Steps:**
1. Review and approve this plan
2. Set up development environment
3. Begin Phase 1 implementation
4. Regular progress reviews and adjustments

The integration will enable realistic physics-based movement, collision detection with the voxel world, and future-proof the codebase for advanced features like swimming, climbing, and vehicles.
