# Specification: Add Fabric Model Generation System

## Overview

Implement a procedural voxel world generation system based on quaternion field interpolation. This "fabric" approach generates continuous surfaces by evaluating quaternion values at octree nodes and detecting sign changes (positive to negative) for surface extraction during rendering or raycasting.

## Source

Migrated from: `openspec/changes/add-fabric-model-generation/`

## Current Status

**Completion: 96% (45/47 tasks complete)**

### Completed Work
- Phase 1: Pre-Requirements - Cube Value Extension (3/3)
- Phase 2: Fabric Module Foundation (5/5)
- Phase 3: Quaternion Interpolation Implementation (7/7)
- Phase 4: Fabric Cube Generator (5/5)
- Phase 5: Surface Detection and Normals (5/5)
- Phase 6: Max Depth Rendering Support (5/5)
- Phase 7: Renderer Configuration Refactor (7/7)
- Phase 8: Model Selector Page UI (7/7)
- Phase 9: Fabric Model Integration (5/5)
- Phase 10: Validation and Documentation (3/5)

### Pending Work
- Manual testing of fabric rendering with various additive state configurations
- Manual verification of model selector UI

## Problem Statement

The project needs a procedural voxel world generation system that can create continuous surfaces without pre-defined geometry. Traditional voxel approaches require explicit material assignment to each voxel, limiting the ability to generate smooth, organic terrain.

## Solution: Quaternion Field Interpolation

### Core Concepts

1. **Non-normalizing LERP interpolation**: Preserves magnitude for SDF fields
2. **Origin-centered generation**: Root magnitude > 1.0 (inside), boundary magnitude < 1.0 (outside)
3. **Natural surface emergence**: Surface appears where magnitude crosses 1.0 threshold
4. **Additive states per depth**: Both rotation and magnitude components for terrain variation
5. **Normal calculation**: Derived from magnitude gradient
6. **Color derivation**: From quaternion rotation component

### Key Components

#### Fabric Module (`crates/cube/src/fabric/`)
- `mod.rs` - Module structure
- `types.rs` - FabricConfig, AdditiveState structs
- `interpolation.rs` - Non-normalizing LERP, octant rotation, magnitude calculation
- `generator.rs` - FabricGenerator for Cube<Quat> generation
- `surface.rs` - Surface detection, normal calculation, color mapping

#### Renderer Integration
- Extended `Cube<T>` with value() method for all node types
- Model selector page with categories (Single Cube, VOX, CSM, Fabric)
- Unified renderer config (`config.ron`)
- max_depth rendering parameter for LOD control

## Affected Files

### New Files
- `crates/cube/src/fabric/mod.rs`
- `crates/cube/src/fabric/types.rs`
- `crates/cube/src/fabric/interpolation.rs`
- `crates/cube/src/fabric/generator.rs`
- `crates/cube/src/fabric/surface.rs`
- `crates/renderer/config.ron`

### Modified Files
- `crates/cube/src/lib.rs` - Add fabric module
- `crates/cube/src/core/cube.rs` - Add value getter for all node types
- `crates/renderer/src/egui_app.rs` - Model selector panel
- `crates/renderer/src/scenes/model_config.rs` - Extended config structure
- `crates/renderer/src/cpu_tracer/trace.rs` - max_depth support
- `crates/renderer/src/bcf_tracer/trace.rs` - max_depth support

## Configuration

### Fabric Model Parameters
```ron
FabricConfig(
    root_magnitude: 2.0,      // Inside value at origin
    boundary_magnitude: 0.5,  // Outside value at world boundary
    surface_radius: 0.7,      // Radius where surface emerges (|Q| = 1.0)
    additive_states: [        // Per-depth terrain variation
        AdditiveState(rotation: 0.1, magnitude: 0.05),
        // ... more states per depth
    ],
    default_max_depth: 8,     // LOD control
)
```

## Success Criteria

1. Fabric module generates Cube<Quat> structures correctly
2. Surface detection works using magnitude threshold crossing at |Q| = 1.0
3. Normal calculation produces smooth gradients for lighting
4. Quaternion-to-color mapping produces visible terrain colors
5. Model selector UI allows interactive configuration of fabric parameters
6. Renderer respects max_depth parameter for LOD control
7. All renderer modes (CPU, GL, BCF, Mesh) support fabric models

## Development Environment

```bash
# Run fabric tests
cargo test -p cube fabric

# Check renderer builds
cargo check -p crossworld-renderer

# Run renderer with fabric model
cargo run -p crossworld-renderer

# Run all workspace tests
cargo test --workspace
```

## Key Algorithms

### Magnitude from Distance
```rust
fn magnitude_from_distance(distance: f32, config: &FabricConfig) -> f32 {
    let t = distance / config.surface_radius;
    lerp(config.root_magnitude, config.boundary_magnitude, t)
}
```

### Surface Detection
```rust
fn is_surface(current: Quat, neighbor: Quat) -> bool {
    let current_mag = current.length();
    let neighbor_mag = neighbor.length();
    (current_mag - 1.0) * (neighbor_mag - 1.0) < 0.0  // Sign change
}
```

### Quaternion to Color
```rust
fn quaternion_to_color(quat: Quat) -> [u8; 3] {
    // Convert rotation component to HSV, then to RGB
    let normalized = quat.normalize();
    // ... HSV mapping from rotation angles
}
```
