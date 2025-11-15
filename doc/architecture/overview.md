# Architecture Overview

## System Components

Crossworld is a Nostr-based metaverse prototyping platform combining high-performance voxel rendering with decentralized identity and communication.

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Browser (Client)                          │
├──────────────────┬──────────────────────┬────────────────────┤
│  TypeScript/React│    WASM Modules      │  External Services │
│                  │                      │                    │
│  • UI/UX         │  • World Simulation  │  • Nostr Relays    │
│  • Rendering     │  • Voxel Engine      │  • MoQ Relay       │
│  • Input         │  • Physics Engine    │                    │
│  • Networking    │  • CPU Raytracer     │                    │
└──────────────────┴──────────────────────┴────────────────────┘
```

### Technology Stack Layers

**1. Presentation Layer** (TypeScript/React):
- React 18 with TypeScript 5.8
- Chakra UI 2.10.9 for components
- React Router DOM 7.1.3 for navigation
- Vite 6 for build tooling

**2. Rendering Layer** (TypeScript/Three.js):
- Three.js 0.170 for 3D scene management
- WebGL for GPU rendering
- Custom shader system
- Post-processing effects

**3. Simulation Layer** (Rust → WASM):
- Voxel octree engine (cube crate)
- Rapier3D physics (physics crate)
- CPU raytracer (renderer crate)
- World state management (world crate)

**4. Communication Layer** (TypeScript):
- Nostr protocol via Applesauce 4.0
- MoQ (Media over QUIC) for voice/streaming
- WebTransport for networking

## Data Flow

### Initialization Flow

```
1. App Loads
   ↓
2. Initialize WASM Modules
   - cube.wasm (voxel engine)
   - physics.wasm (physics simulation)
   ↓
3. Fetch Live Event (kind 30311)
   - Get server config
   - Get MoQ relay URL
   - Get Nostr relays
   ↓
4. Setup Scene
   - Three.js renderer
   - Camera and controls
   - Lighting system
   ↓
5. Connect Services
   - Nostr relays
   - MoQ relay
   ↓
6. Load World
   - Voxel terrain from CSM
   - Physics collision meshes
   ↓
7. User Ready
```

### Runtime Data Flow

**User Input → Simulation → Rendering**:
```
Mouse/Keyboard/Gamepad
   ↓
Input Handler (TypeScript)
   ↓
Avatar Controller (TypeScript)
   ↓
Physics Bridge → WASM Physics
   ↓
Position Update
   ↓
Three.js Scene Update
   ↓
WebGL Render
```

**Voxel Drawing**:
```
User Click on Voxel
   ↓
Raycast (3D → Voxel Coords)
   ↓
WASM: Update Octree
   ↓
WASM: Generate Mesh
   ↓
TypeScript: Update Three.js Geometry
   ↓
Render Updated World
```

**Network Synchronization**:
```
Local State Change
   ↓
Create Nostr Event
   ↓
Publish to Relays
   ↓
Other Clients Receive
   ↓
Update Remote State
   ↓
Render Changes
```

## Rust ↔ WASM ↔ TypeScript Boundaries

### WASM Interface Pattern

**Rust Side** (crates/*/src/lib.rs):
```rust
#[wasm_bindgen]
pub struct WasmEngine {
    // Internal state
}

#[wasm_bindgen]
impl WasmEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { ... }

    pub fn update(&mut self, dt: f32) { ... }

    pub fn get_data(&self) -> JsValue { ... }
}
```

**TypeScript Side** (packages/wasm-*/):
```typescript
import init, { WasmEngine } from './wasm-module'

await init()  // Initialize WASM module
const engine = new WasmEngine()
engine.update(deltaTime)
const data = engine.get_data()
```

### Data Transfer

**From Rust to TypeScript**:
- Primitive types: Direct pass
- Structures: Serialize to JsValue (serde)
- Arrays: Float32Array, Uint32Array (zero-copy where possible)
- Geometry: GeometryData struct (vertices, indices, normals, colors)

**From TypeScript to Rust**:
- Simple params: Direct pass
- Complex objects: JSON serialize → Rust deserialize
- User input: Event → TypeScript handler → WASM method call

## Core Subsystems

### 1. Voxel System

**Purpose**: Efficient voxel storage and rendering

**Implementation**: `crates/cube/`
- Octree data structure for hierarchical voxel storage
- CSM (Cube Script Model) text format for serialization
- Mesh generation with face culling and greedy meshing
- Support for multiple depths and resolutions

**Key Types**:
```rust
pub enum Cube<T> {
    Empty,
    Solid(T),
    Subdivided(Box<[Cube<T>; 8]>),
}
```

See: [voxel-system.md](voxel-system.md)

### 2. Physics System

**Purpose**: Realistic physics simulation and collision detection

**Implementation**: `crates/physics/`
- Rapier3D physics engine
- Character controllers for avatars
- Voxel collision mesh generation
- Kinematic and dynamic rigid bodies

**Features**:
- Gravity simulation
- Collision detection and response
- Raycasting for ground detection
- Step climbing and slope handling

See: [physics.md](physics.md)

### 3. Rendering Pipeline

**Purpose**: Visual presentation of the 3D world

**Implementation**:
- Three.js for scene management
- WebGL for GPU rendering
- Custom shaders for voxel materials
- Post-processing effects

**Render Targets**:
- Voxel terrain (from WASM geometry)
- Avatar models (GLB or voxel-based)
- Sky/atmosphere
- UI overlays

See: [rendering.md](rendering.md)

### 4. Raycasting

**Purpose**: Ray-octree intersection for voxel interaction

**Implementation**: `crates/cube/src/raycast/`
- DDA-based octree traversal
- Normal calculation for hit faces
- Efficient empty space skipping
- Depth-limited traversal

**Use Cases**:
- Voxel selection (click to edit)
- Ground detection for avatars
- Line-of-sight checks
- Physics queries

See: [raycast.md](raycast.md)

## Workspace Organization

### Rust Crates (crates/)

**Core Engine**:
- `world/` - Main WASM module, world state management
- `cube/` - Voxel octree engine and CSM format
- `physics/` - Rapier3D integration and character controllers
- `renderer/` - CPU raytracer (experimental)
- `assets/` - Asset management and loading

**Tools**:
- `worldtool/` - CLI for Nostr live events and server management

### TypeScript Packages (packages/)

**Application**:
- `app/` - Main React application
  - `src/components/` - UI components
  - `src/renderer/` - Three.js rendering
  - `src/services/` - Business logic
  - `src/voice/` - MoQ voice chat

**Shared**:
- `common/` - Shared UI components and utilities
- `editor/` - Voxel model editor (future)

**Generated WASM Bindings**:
- `wasm-world/` - From crates/world
- `wasm-cube/` - From crates/cube
- `wasm-physics/` - From crates/physics

### Build Artifacts (Generated)

**Not in Version Control**:
- `target/` - Rust compilation output
- `dist/` - Frontend production builds
- `node_modules/` - JavaScript dependencies
- `packages/wasm-*/` - Generated WASM bindings

## Development Workflow

### Build Process

**Development**:
```bash
just dev  # Build WASM (dev mode) + start dev server
```

This runs:
1. `just build-wasm-dev` - Compile Rust to WASM (dev mode)
2. `cd packages/app && bun run dev` - Start Vite dev server

**Production**:
```bash
just build  # Build everything for production
```

This runs:
1. `just build-wasm` - Compile Rust to WASM (release mode)
2. `bun run build` - Build TypeScript frontend

### WASM Compilation

**Development Mode** (faster compilation):
- `wasm-pack build --target web --dev`
- No optimizations
- Debug symbols included
- ~2-3 minutes for all crates

**Release Mode** (optimized):
- `wasm-pack build --target web --release`
- Full optimizations (opt-level=3, lto=true)
- No debug symbols
- ~5-10 minutes for all crates

### Parallel Builds

Independent WASM crates build in parallel:
```bash
# In justfile:
build-wasm:
  wasm-pack build crates/world &
  wasm-pack build crates/cube &
  wasm-pack build crates/physics &
  wait
```

## State Management

### Local State (TypeScript)

**UI State**:
- React useState/useEffect
- Local component state
- No global state manager

**Scene State**:
- Three.js scene graph
- Avatar positions and rotations
- Camera state

### Persistent State (Nostr)

**User State** (kind 30317):
- Avatar configuration
- Current position
- Voice/mic status

**World State** (kind 30311):
- Live event metadata
- Server configuration
- MoQ relay URL

**Updates** (kind 1317):
- Position changes
- Movement style
- Temporary state updates

### Simulation State (Rust/WASM)

**Voxel World**:
- Octree structure in WASM memory
- Modified via TypeScript calls
- Persisted to CSM files

**Physics**:
- Rigid bodies and colliders
- Character controllers
- Simulated in Rust, queried from TypeScript

## Performance Characteristics

### WASM Performance

**Initialization**:
- WASM module load: <100ms
- Initial octree setup: <50ms
- Physics world init: <50ms

**Runtime**:
- Voxel updates: 1-5ms
- Mesh generation: 5-20ms (varies by complexity)
- Physics step: 1-3ms per frame
- Raycast queries: <1ms

### Rendering Performance

**Target**: 60 FPS (16.6ms per frame)

**Frame Budget**:
- Input handling: <1ms
- Physics simulation: 1-3ms
- Scene updates: 2-5ms
- WebGL render: 8-12ms
- Overhead: 1-2ms

### Network Performance

**Nostr Events**:
- State updates: ~100-500ms latency
- Chat messages: ~200-1000ms latency
- Event size: typically <5KB

**MoQ Voice**:
- Audio latency: 100-300ms
- Bandwidth: ~32kbps per participant

## Related Documentation

- [voxel-system.md](voxel-system.md) - Voxel engine details
- [physics.md](physics.md) - Physics integration
- [raycast.md](raycast.md) - Raycasting system
- [rendering.md](rendering.md) - Rendering pipeline
- [../features/avatar-system.md](../features/avatar-system.md) - Avatar implementation
- [../features/voice-chat.md](../features/voice-chat.md) - MoQ voice chat
- [../reference/build-system.md](../reference/build-system.md) - Build process
- [../reference/project-structure.md](../reference/project-structure.md) - Repository layout
