# Design Specification vs. Implementation Comparison

This document compares the authoritative design specifications in `obsidian/` against the current crate implementations. The Obsidian specs define the intended architecture - this report identifies where implementation needs to catch up.

## Executive Summary

The Obsidian design specs define the target architecture for Crossworld. This analysis identifies implementation gaps that need to be addressed to align the codebase with the design vision.

**Key Findings:**
- 16 design specs reviewed
- 4 components not yet implemented (System, Audio, Map, Logic, LLM)
- 5 implementations diverged from spec and need refactoring
- 3 implementations need additional features per spec
- 4 implementations are well-aligned with spec

---

## 1. Implementation Gaps (Spec Features Not Yet Built)

### 1.1 System Component - NOT IMPLEMENTED
**Spec:** System.md (design-proposal)
**Priority:** HIGH

The spec defines a unified platform abstraction layer that should consolidate:
- Window management abstraction
- Event loop abstraction
- Timer and timing utilities
- Platform detection (Native vs Web)
- File system path handling
- Environment variable access
- Camera controllers (orbit, first-person)
- Lua configuration support

**Current State:** Features scattered across `crates/app`, `crates/core`, `crates/scripting`

**Action Required:**
- Create `crates/system` to consolidate platform abstractions per spec
- Refactor app/core to use System as foundation
- Integrate App into System as specified

---

### 1.2 Audio Component - NOT IMPLEMENTED
**Spec:** Audio.md (design-proposal)
**Priority:** MEDIUM

The spec defines:
- AudioEngine - Main audio system
- Sound - Loaded audio asset
- Player - Playing sound handle
- Features: sound playback, music crossfade, 3D spatial audio, MoQ voice chat

**Current State:** No `crates/audio` exists

**Action Required:**
- Create `crates/audio` implementing spec types
- Integrate with MoQ for voice chat
- Support WAV/OGG/MP3 loading

---

### 1.3 Map Component - NOT IMPLEMENTED
**Spec:** Map.md (draft)
**Priority:** LOW

The spec defines:
- Area type for lat/long coordinates
- `get_height(Area)` - Height map data
- `get_image(Area)` - Map as image
- OpenStreetMap integration

**Current State:** No `crates/map` exists

**Action Required:**
- Create `crates/map` when geospatial features needed
- Implement OpenStreetMap tile fetching
- Height map data integration

---

### 1.4 Logic Component - NOT IMPLEMENTED
**Spec:** Logic.md (draft)
**Priority:** LOW

The spec defines:
- Rule type with match/set patterns
- RuleTx for 3D transforms
- Action type with execute callbacks
- Declarative rule systems for Cube transformations

**Current State:** No `crates/logic` exists. `cube::function` provides procedural generation but not rule-based transformations.

**Action Required:**
- Create `crates/logic` for rule-based transformers
- Integrate with Cube for pattern matching

---

### 1.5 LLM Component - NOT IMPLEMENTED
**Spec:** LLM.md (draft)
**Priority:** LOW

The spec defines:
- Background LLM task interface
- Tool call interface
- Text input/output

**Current State:** No LLM integration exists

**Action Required:**
- Create LLM client crate when AI features needed

---

### 1.6 Devices Component - PARTIALLY IMPLEMENTED
**Spec:** Devices.md (draft)
**Priority:** MEDIUM

**Spec Defines:**
- GamepadState, MouseButtons types ✓ (implemented in core)
- Accelerometer interface ✗
- Compass interface ✗
- Touch interface ✗

**Current State:** Basic input types exist but sensor support missing

**Action Required:**
- Add accelerometer support
- Add compass support
- Add touch input support

---

## 2. Implementation Divergences (Code Differs from Spec)

### 2.1 App Component - SPEC VIOLATION
**Spec:** App.md states "integrated into System" (deprecated status)
**Current:** App crate is actively used standalone

**Issue:** Spec says App should be deprecated and integrated into System, but System doesn't exist and App is the active framework.

**Action Required:**
- Implement System crate per spec
- Migrate App functionality into System
- Update App crate to be thin wrapper or remove

---

### 2.2 Scripting Component - IMPLEMENTATION DRIFT
**Spec:** Scripting.md defines Lua-first with StateTree

**Spec Defines:**
- LuaEngine with globals HashMap
- StateTree with KDL integration
- load_lua, load_kdl functions
- Hot-reload script changes

**Current Divergence:**
- KDL is primary (spec says Lua primary)
- Hot-reload not implemented

**Action Required:**
- Implement hot-reload per spec
- Clarify if KDL-first is intentional (update spec) or should revert to Lua-first

---

### 2.3 Network Component - NAMING MISMATCH
**Spec:** Network.md defines transport abstractions
**Current:** `crates/server` implements server-side only

**Issue:** Spec defines unified Network component for client/server transport. Implementation is server-only in differently-named crate.

**Action Required:**
- Create `crates/network` with shared transport abstractions
- Client WebTransport support
- Refactor server to use network crate

---

## 3. Implementations Needing Spec Features

### 3.1 Cube Component - MOSTLY ALIGNED
**Spec:** Cube.md

**Implemented per spec:**
- Cube enum (Empty, Solid, Octree) ✓
- CubeCoord, CubeBox, CubeGrid ✓
- Hit, Face types ✓
- CubeFunction for procedural generation ✓
- raycast, parse_csm, serialize_csm ✓
- visit_* traversals ✓

**Extra features not in spec (need spec update or removal):**
- BCF binary format
- Fabric surface extraction system
- Color mappers (HsvColorMapper, PaletteColorMapper, VoxColorMapper)
- Function CPU/GPU backends

**Action Required:**
- Decide: expand spec to cover extras, or remove undocumented features

---

### 3.2 Renderer Component - SPEC INCOMPLETE
**Spec:** Renderer.md (implemented status)

**Spec defines:**
- Camera type
- OrbitController, FirstPersonController
- Generic Renderer interface (marked TODO)

**Implementation has undocumented features:**
- CpuTracer, BcfTracer, GlTracer, ComputeTracer
- MeshRenderer, SkyboxRenderer
- CrtPostProcess

**Action Required:**
- Document all renderer types in spec
- Define Renderer trait interface per spec TODO

---

### 3.3 Nostr Component - EXTRA FEATURES
**Spec:** Nostr.md (prototype)

**Spec defines:**
- Identity type with Keys, PublicKey
- LiveEvent for NIP-53
- Key management (web extension, NIP-46, guest)

**Implementation extras:**
- NostrAccount, AccountState (UI state)
- AvatarState, PositionUpdate, WorldModel events
- QR code login flow

**Action Required:**
- Add event types to spec
- Document AccountState UI pattern

---

### 3.4 Physics Component - MINOR EXTRAS
**Spec:** Physics.md

**Well aligned.** All spec types implemented:
- PhysicsWorld, CubeObject, CharacterController ✓
- VoxelColliderBuilder, VoxelTerrainCollider ✓
- Collider builders ✓

**Undocumented extras:**
- Object trait
- Terrain active region system
- Native Bevy helpers

**Action Required:**
- Add terrain region system to spec

---

## 4. Well-Aligned Implementations

These implementations match their specs well:

| Component | Spec | Status |
|-----------|------|--------|
| World | World.md | ✓ Aligned |
| Game | Game.md | ✓ Aligned |
| Editor | Editor.md | ✓ Aligned |
| Testbed | Testbed.md | ✓ Aligned |

---

## 5. Priority Action Items

### Immediate (Architectural)

1. **Create System crate** - Core platform abstraction per spec
   - Consolidate from app/core/scripting
   - Deprecate App per spec intent

2. **Create Network crate** - Shared transport abstractions
   - WebTransport client/server
   - Refactor server to use it

### Short-term (Feature Gaps)

3. **Implement Scripting hot-reload** - Per spec requirement

4. **Implement Devices sensors** - Accelerometer, compass, touch

5. **Document Renderer types** - Complete spec TODO for Renderer trait

### Medium-term (New Components)

6. **Create Audio crate** - Sound system per spec

7. **Create Logic crate** - Rule-based transformations

### Long-term (Future Features)

8. **Create Map crate** - When geospatial features needed

9. **Create LLM crate** - When AI features needed

---

## 6. Undocumented Crates

These crates exist but have no spec. Action needed:

| Crate | Recommendation |
|-------|----------------|
| `test-client` | Utility - no spec needed |
| `trellis` | Needs investigation and spec if production |
| `robocube` | Needs investigation and spec if production |
| `xcube` | Needs investigation and spec if production |
| `proto-gl` | Prototype - no spec needed |
| `proto-bevy` | Prototype - no spec needed |
| `app-bevy` | Needs spec if production use planned |
| `editor-bevy` | Needs spec if production use planned |
| `worldtool` | Utility - no spec needed |

---

## Appendix: Spec Compliance Summary

| Spec | Status | Implementation | Compliance |
|------|--------|----------------|------------|
| Crossworld.md | - | - | Reference |
| **System.md** | design-proposal | **Not implemented** | **GAP** |
| Devices.md | draft | Partial | Partial |
| **Network.md** | design-proposal | Wrong structure | **DIVERGENT** |
| **Audio.md** | design-proposal | **Not implemented** | **GAP** |
| Scripting.md | draft | Missing hot-reload | Partial |
| Cube.md | - | Extra features | Exceeds |
| Renderer.md | implemented | Incomplete spec | Partial |
| Nostr.md | prototype | Extra features | Exceeds |
| Physics.md | - | Extra features | Aligned+ |
| World.md | - | Aligned | ✓ |
| Server.md | draft | Needs Network refactor | Partial |
| Game.md | - | Aligned | ✓ |
| Editor.md | - | Aligned | ✓ |
| Testbed.md | - | Aligned | ✓ |
| App.md | deprecated | Should integrate to System | **VIOLATION** |
| Assets.md | - | Aligned | ✓ |
| **Map.md** | draft | **Not implemented** | **GAP** |
| **Logic.md** | draft | **Not implemented** | **GAP** |
| **LLM.md** | draft | **Not implemented** | **GAP** |
| Core.md | - | Needs System integration | Partial |

---

## 7. Alignment Procedures

### Phase 1: Architectural Foundation (High Priority)

#### 1.1 Create System Crate
**Spec:** System.md requires unified platform abstraction

**Procedure:**
1. Create `crates/system/` with modules:
   - `platform.rs` - Platform enum (Native/Web), detection
   - `timer.rs` - High-resolution timing utilities
   - `path.rs` - PathResolver for cross-platform paths
   - `window.rs` - WindowHandle abstraction
   - `app.rs` - App trait (migrate from crates/app)
   - `input.rs` - InputState (consolidate from core)
   - `camera.rs` - Camera controllers (consolidate from core/app)

2. Refactor dependencies:
   ```
   system depends on: core, renderer, devices, audio, scripting
   app becomes: thin re-export layer or deprecated
   ```

3. Migration steps:
   - Move `App` trait from `crates/app` to `crates/system`
   - Move `FrameContext` to system
   - Move camera controllers to system
   - Update game/editor/testbed to use system

#### 1.2 Refactor Network Architecture
**Spec:** Network.md requires shared transport abstractions

**Procedure:**
1. Verify `crates/network` provides:
   - Transport trait for WebTransport/WebSocket
   - Reliable/unreliable message channels
   - Connection state management
   - Automatic reconnection

2. Refactor `crates/server` to use network crate:
   ```toml
   # server/Cargo.toml
   network = { path = "../network" }
   ```

3. Implement client-side WebTransport in network crate

---

### Phase 2: Missing Features (Medium Priority)

#### 2.1 Implement Scripting Hot-Reload
**Spec:** Scripting.md requires hot-reload

**Procedure:**
1. Add file watcher to `crates/scripting`:
   ```rust
   // scripting/src/hot_reload.rs
   pub struct ScriptWatcher {
       watcher: notify::RecommendedWatcher,
       lua_engine: Arc<Mutex<LuaEngine>>,
   }
   ```

2. Implement reload callback:
   - Detect file changes
   - Re-execute changed Lua scripts
   - Emit events for state tree updates

#### 2.2 Complete Devices Sensors
**Spec:** Devices.md requires accelerometer, compass, touch

**Procedure:**
1. Verify `crates/devices` has sensor modules
2. Implement platform-specific backends:
   - Web: DeviceMotion/DeviceOrientation APIs
   - Native: platform sensor libraries
3. Add touch input support for mobile

---

### Phase 3: Verify Recent Implementations

The merge to main added new crates. Verification completed 2026-02-06:

| Crate | Spec | Status | Notes |
|-------|------|--------|-------|
| `crates/audio` | Audio.md | ✓ ALIGNED | All spec types implemented plus extras |
| `crates/devices` | Devices.md | ✓ ALIGNED | Sensors, touch, gamepad all implemented |
| `crates/network` | Network.md | ✓ ALIGNED | Transport traits match spec |
| `crates/logic` | Logic.md | ✓ ALIGNED | Rule, RuleTx, Action per spec |
| `crates/map` | Map.md | ✓ ALIGNED | Area, get_height, get_image implemented |
| `crates/llm` | LLM.md | ✓ ALIGNED | Task/tools interface implemented |

#### Detailed Verification Results

**1. crates/audio vs Audio.md** ✅ FULLY ALIGNED

Spec requires:
- AudioEngine - Main audio system ✓ Implemented (`AudioEngine`, `AudioEngineConfig`)
- Sound - Loaded audio asset ✓ Implemented (`Sound`, `SoundData`)
- Player - Playing sound handle ✓ Implemented (`SoundPlayer`, `SoundHandle`)
- Music crossfade ✓ Implemented (`MusicPlayer`, `MusicTrack`, `CrossfadeConfig`)
- 3D spatial audio ✓ Implemented (`SpatialSource`, `AudioListener`, `SpatialConfig`)
- MoQ voice chat ✓ Implemented (`VoiceChatIntegration`, `VoiceParticipant`, `VoiceChatConfig`)

**Extras not in spec** (spec update recommended):
- `Volume` type with dB conversion
- `AudioCategory` enum (Master, Effects, Music, Voice, Ambient, Ui)
- `AudioPosition` with Doppler velocity
- `ParticipantState`, `DiscoverySource` for voice chat

---

**2. crates/devices vs Devices.md** ✅ FULLY ALIGNED

Spec requires:
- GamepadState ✓ Implemented (`GamepadState`, `ControllerInput`, `ControllerInfo`)
- MouseButtons ✓ Implemented (`MouseButtons`, `MouseButtonType`, `CursorMode`)
- Accelerometer ✓ Implemented (`Accelerometer` with `is_level()`, `magnitude()`)
- Compass ✓ Implemented (`Compass` with `direction()`, `cardinal()`)
- Touch interface ✓ Implemented (`TouchState`, `TouchPoint`, `TouchPhase`)

**Extras not in spec** (spec update recommended):
- `Gyroscope` sensor
- `SensorState` combining all sensors
- `KeyboardState`, `KeyState` types
- `ControllerBackend` trait with gilrs support
- Pinch/zoom gesture detection

---

**3. crates/network vs Network.md** ✅ FULLY ALIGNED

Spec requires:
- Transport trait ✓ Implemented (`Transport` trait with `Reliable` and `Unreliable` channels)
- WebTransport support ✓ Implemented (feature-gated `webtransport` module)
- Client/server abstractions ✓ Implemented (`TransportConnector`, `TransportListener`)
- Connection state management ✓ Implemented (`ConnectionState`, `ConnectionEvent`, `ConnectionInfo`)
- Reconnection strategies ✓ Types defined, implementation pending

**Extras not in spec** (spec update recommended):
- `ReliableMessage` / `UnreliableMessage` enums
- `PlayerState`, `PlayerIdentity`, `CompactPosition` types
- `AnimationState` for networked animations
- `TransportConfig` with TLS settings

---

**4. crates/logic vs Logic.md** ✅ FULLY ALIGNED

Spec requires:
- Rule type ✓ Implemented (`Rule` with conditions, actions, priority, tags)
- RuleTx for transactions ✓ Implemented (`RuleTx` with atomic commit/rollback)
- Action type ✓ Implemented (enum: SetVoxel, ClearVoxel, FillRegion, Replace, CopyRegion, Spawn, Emit)

**Extras not in spec** (spec update recommended):
- `Condition` enum (MaterialAt, Empty, SolidAt, etc.)
- `RuleEngine` orchestrator with priority-based execution
- `RuleExecutor` trait
- `RuleContext` for evaluation state
- `TxChange` for change tracking
- `CubeAdapter` for Cube integration (feature-gated)

---

**5. crates/map vs Map.md** ✅ FULLY ALIGNED

Spec requires:
- Area type ✓ Implemented (`Area` with lat/long bounding box)
- `get_height(Area)` ✓ Implemented (`get_height()`, `get_height_map()`, `HeightProvider` trait)
- `get_image(Area)` ✓ Implemented (`get_image()`, `ImageProvider` trait, `TileImage`)
- OpenStreetMap integration ✓ Module exists (`osm` module, placeholder)

**Extras not in spec** (spec update recommended):
- `GeoCoord`, `WorldCoord` coordinate types
- `WorldArea` for world-space areas
- `HeightMap` grid type with bilinear sampling
- `TileCoord` for Web Mercator tile addresses
- `tiles_for_area()` helper

---

**6. crates/llm vs LLM.md** ✅ FULLY ALIGNED

Spec requires:
- Background task interface ✓ Implemented (`spawn_task()`, `TaskBuilder`, `TaskHandle`)
- Tool call interface ✓ Implemented (`ToolHandler` trait, `ToolRegistry`, `ToolCall`)
- Text input/output ✓ Implemented (`Message`, `CompletionRequest`, `CompletionResponse`)

**Extras not in spec** (spec update recommended):
- `LlmClient` trait with streaming support (`ChunkStream`, `StreamAccumulator`)
- `TaskContext` with cancellation
- `TaskStatus` enum (Pending, Running, Completed, Failed, Cancelled)
- `ToolDefinitionBuilder` for fluent tool creation
- `FnTool` for function-based handlers
- `ToolResult` type
- Role, FinishReason, StreamChunk types

---

### Phase 4: Spec Documentation Updates

For components where implementation exceeds spec, update Obsidian:

**Cube.md additions needed:**
- BCF binary format specification
- Fabric surface extraction system
- Color mapper interfaces

**Renderer.md additions needed:**
- All tracer types (Cpu, Bcf, Gl, Compute)
- MeshRenderer, SkyboxRenderer
- CrtPostProcess effects
- Renderer trait interface

**Physics.md additions needed:**
- Terrain active region system
- Object trait specification

**Nostr.md additions needed:**
- AvatarState, PositionUpdate, WorldModel events
- AccountState UI pattern
- QR login flow

---

### Migration Order

```
1. Verify new crates against specs (audio, devices, network, logic, map, llm)
   ↓
2. Create System crate (unblocks App deprecation)
   ↓
3. Refactor Server to use Network crate
   ↓
4. Implement Scripting hot-reload
   ↓
5. Update Obsidian specs for implementation extras
```
