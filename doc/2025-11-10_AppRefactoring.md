# App Initialization Refactoring - 2025-11-10

## Current Issues


2. **Fragmented initialization flow**: Initialization happens across multiple components (main.tsx → Root.tsx → App.tsx → WorldCanvas.tsx → SceneManager) with async dependencies scattered throughout.

3. **Mixed responsibility**: App.tsx handles both UI state (login, avatar selection) and world/physics initialization, creating tight coupling.

4. **Multiple WASM modules**: Two separate WASM modules (cube geometry and physics) are initialized at different times with no clear sequencing.

## Current Initialization Flow

### Sequence

```
main.tsx
  └─> ensureCubeWasmInitialized() [WASM #1 - Cube Geometry]
      └─> <Root />
          └─> <App />
              ├─> Auto-login (async)
              ├─> Avatar state restoration (async)
              ├─> World loading (async)
              └─> <WorldCanvas />
                  └─> SceneManager.initialize()
                      ├─> PhysicsBridge.init() [WASM #2 - Physics]
                      │   ├─> ensurePhysicsWasmInitialized()
                      │   ├─> new WasmPhysicsWorld()
                      │   └─> createGroundPlane() at Y=0
                      ├─> Renderer setup
                      ├─> Sun system
                      ├─> Post-processing
                      ├─> Event listeners
                      ├─> CameraController
                      └─> MaterialsLoader (async, non-blocking)
```

### Avatar Creation (Separate, Later)

Avatar physics system is under development.

```
App.tsx (after login)
  └─> Avatar constructor
      ├─> Initial position: (4, 5, 4) - 5 units above ground (this is for debugging, WIP)
      └─> If physicsBridge provided:
          ├─> createCharacter(position, height=1.8, radius=0.3)
          │   └─> CharacterController::new()
          │       ├─> Kinematic capsule body
          │       └─> Position is at capsule CENTER
          └─> Returns physicsHandle
```

### Update Loop (Every Frame)

```
SceneManager.render()
  ├─> physicsBridge.step(dt)
  ├─> avatar.update(dt)
  │   ├─> moveCharacter(velocity, dt)
  │   └─> Sync position from physics
  │       └─> visualY = physicsPos.y - height/2
  ├─> cameraController.update(dt)
  └─> renderer.render()
```

## Identified Problems

### 1. Ground Detection Issues

**Location**: `crates/physics/src/character_controller.rs:236-304`

This is under development. Can be refactored into better module structure.


### 3. Asynchronous Initialization Chaos

Multiple async operations without clear dependency management:
- Cube WASM loading (required before React render)
- Physics WASM loading (required before physics use)
- Auto-login (independent)
- Avatar state restoration (depends on login)
- World loading (depends on login)
- Materials/textures loading (independent, non-blocking)

### 4. No Single Source of Truth for Initialization State

No centralized system tracking:
- Which subsystems are initialized
- Which dependencies are satisfied
- When app is ready for user interaction

## Proposed Architecture

### Single Initialization Manager

Create `AppInitializer` class that:
1. Manages all initialization sequencing
2. Tracks subsystem dependencies
3. Provides initialization state to UI
4. Ensures proper error handling and recovery

### Initialization Phases

```
Phase 1: Core WASM Modules (Parallel)
  ├─> Cube WASM
  └─> Physics WASM

Phase 2: Rendering Infrastructure
  ├─> WebGL context
  ├─> Three.js renderer
  ├─> Scene setup
  └─> Physics world + ground plane

Phase 3: User Session (Parallel)
  ├─> Auto-login
  ├─> Profile loading
  └─> World loading

Phase 4: Game World
  ├─> Materials/textures (non-blocking)
  ├─> Sun system
  ├─> Post-processing
  └─> Input controllers

Phase 5: Avatar Spawning
  ├─> Create avatar at spawn position
  ├─> Create character controller
  └─> Attach camera

Phase 6: Ready
  └─> Start render loop
```

### New File Structure

```
packages/app/src/Initializer.ts       # Main initialization orchestrator
# Separated to other files if too much code
```
PhysicsBridge should be renamed to World. Handles world model and status with wasm interface

### AppInitializer API

```typescript
class AppInitializer {
  // Initialization state observable
  state$: Observable<InitializationState>

  // Initialize all subsystems in correct order
  async initialize(canvas: HTMLCanvasElement): Promise<void>

  // Get initialized subsystems
  getWorld(): World
  getSceneManager(): SceneManager
  getRenderer(): THREE.WebGLRenderer

  // Cleanup on unmount
  dispose(): void
}

interface InitializationState {
  phase: 'wasm' | 'rendering' | 'session' | 'world' | 'avatar' | 'ready' | 'error'
  progress: number  // 0-100
  message: string
  error?: Error
}
```


## Backwards Compatibility - NOT NEEDED

As specified, old functionality and backwards compatibility are NOT needed. We can:
- Remove fallback mesh-based raycasting
- Remove non-physics movement code
- Assume physics is always enabled
- Clean up legacy edit mode code

## Implementation Steps

### Step 1: Create Initialization Infrastructure
1. Create `AppInitializer` class
2. Define initialization phases and state
3. Implement WASM loading coordination

### Step 2: Refactor Component Initialization
1. Move physics init to AppInitializer
2. Move renderer init to AppInitializer
3. Move scene setup to AppInitializer

### Step 3: Refactor User Session
1. Move auto-login to SessionInitializer
2. Separate UI state from initialization logic
3. Use observables for state updates

### Step 5: Clean Up Legacy Code
1. Remove mesh-based raycasting fallback
2. Remove non-physics movement paths
3. Simplify BaseAvatar class

### Step 6: Add Loading UI
1. Create initialization progress UI
2. Show phase and progress
3. Handle errors gracefully

## Expected Benefits

1. **Clear initialization flow**: Single entry point, well-defined phases
2. **Better error handling**: Centralized error management
3. **Improved debugging**: Initialization state is observable
4. **Cleaner code**: Separation of concerns between UI and initialization
5. **Physics reliability**: Proper ground detection from the start
6. **No legacy burden**: Clean implementation without backwards compatibility

## Files to Modify

### Delete (Legacy)
- None initially - will identify during refactoring

### Create (New)
- `packages/app/src/initialization/AppInitializer.ts`

### Modify (Refactor)
- `packages/app/src/main.tsx` - Use AppInitializer
- `packages/app/src/App.tsx` - Remove initialization logic, keep UI only
- `packages/app/src/components/WorldCanvas.tsx` - Simplify to receive initialized systems
- `packages/app/src/renderer/scene.ts` - Remove initialization, receive initialized systems
- `packages/app/src/renderer/base-avatar.ts` - Add ground snap, remove non-physics code
- `packages/app/src/physics/physics-bridge.ts` - Minor cleanup
-> - `packages/app/src/world.ts` - Minor cleanup

- `crates/physics/src/character_controller.rs` - Improve ground detection logging

## Testing Strategy

1. **Unit Tests**: Test each initializer independently
2. **Integration Tests**: Test initialization phases sequentially
3. **Manual Testing**: Verify avatar spawns correctly on ground
4. **Error Testing**: Verify graceful failure of each subsystem

## Migration Plan

1. Create new initialization infrastructure alongside existing code
2. Add feature flag to switch between old and new initialization
3. Test new initialization thoroughly
4. Remove old code once new system is validated
5. Clean up deprecated patterns

---

## Implementation Summary (2025-11-10)

### Changes Made

#### 3. Built Physics WASM
- Rebuild `crossworld-physics` WASM module with updated ground detection logic (racast from y infinity to ground cube)

#### 4. Created Initialization Infrastructure
Created new initialization system in `packages/app/src/initialization/`:
- `types.ts` - Initialization state machine and types
- `WasmLoader.ts` - Parallel WASM module loading
- `PhysicsInitializer.ts` - Physics world setup
- `RendererInitializer.ts` - Three.js renderer setup
- `AppInitializer.ts` - Orchestrates initialization phases
- `index.ts` - Module exports


#### 5. Added SceneManager.getRenderer() Method
**File**: `packages/app/src/renderer/scene.ts:1838-1840`
- Added getter method to expose renderer for external access
- Required by RendererInitializer


1. Integrate AppInitializer into main app flow
   - Modify `WorldCanvas.tsx` to use AppInitializer
   - Add loading UI showing initialization progress
   - Better error handling and recovery

2. **Optional**: Remove legacy code
   - Remove mesh-based raycasting fallback (physics always used)
   - Remove non-physics movement code paths
   - Simplify BaseAvatar class

### Files Modified

**Core Fixes** (Implemented):
- `crates/physics/src/character_controller.rs` - Adaptive ground detection
- `packages/app/src/renderer/scene.ts` - Added getRenderer()

**New Infrastructure** (Ready for future use):
- `packages/app/src/AppInitializer.ts` (new)
