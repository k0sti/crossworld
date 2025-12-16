# Change: Add Fabric Model Generation System

## Why
The project needs a procedural voxel world generation system based on quaternion field interpolation. This "fabric" approach generates continuous surfaces by evaluating quaternion values at octree nodes and detecting sign changes (positive to negative) for surface extraction during rendering or raycasting.

## What Changes
- **NEW**: Add `crates/cube/src/fabric/` module implementing `Cube<Quaternion>` generation with:
  - Non-normalizing LERP interpolation for magnitude-based SDF fields
  - Origin-centered generation: root magnitude > 1.0 (inside), boundary magnitude < 1.0 (outside)
  - Natural surface emergence where magnitude crosses 1.0 threshold
  - Additive states per depth with both rotation and magnitude components
  - Normal calculation from magnitude gradient
  - Color derivation from quaternion rotation
- **MODIFIED**: Extend `Cube<T>` to support value retrieval for all node types (not just leaves)
- **MODIFIED**: Renderer model selection - replace top bar dropdown with dedicated model selector page
- **NEW**: Unified renderer configuration file (`config.ron`) consolidating:
  - Single cube material parameters
  - Fabric model parameters (additive quaternion states per depth)
  - Model paths (VOX, CSM)
- **MODIFIED**: Add `max_depth` rendering parameter - treats all nodes at that depth as leaves

## Impact
- Affected specs: NEW `fabric-octree`, NEW `renderer-model-selector`, NEW `renderer-config`
- Affected code:
  - `crates/cube/src/lib.rs` - add fabric module
  - `crates/cube/src/core/cube.rs` - add value getter for all node types
  - `crates/cube/src/fabric/` - new module (mod.rs, generator.rs, interpolation.rs)
  - `crates/renderer/src/egui_app.rs` - model selector page
  - `crates/renderer/src/scenes/model_config.rs` - extend to unified config
  - `crates/renderer/models.ron` -> `crates/renderer/config.ron`
