# Tasks: Add Fabric Model Generation System

## 1. Pre-Requirements: Cube Value Extension
- [ ] 1.1 Add `fn value(&self) -> Option<&T>` method to `Cube<T>` in `crates/cube/src/core/cube.rs`
- [ ] 1.2 Implement default value behavior for `Cube::Cubes` variant (return None or first child's value)
- [ ] 1.3 Add tests for value retrieval on all Cube variants

## 2. Fabric Module Foundation
- [ ] 2.1 Create `crates/cube/src/fabric/mod.rs` with module structure
- [ ] 2.2 Create `crates/cube/src/fabric/types.rs` with:
  - `FabricConfig` (root_magnitude, boundary_magnitude, surface_radius, additive_states, max_depth)
  - `AdditiveState` (rotation: f32, magnitude: f32)
- [ ] 2.3 Create `crates/cube/src/fabric/interpolation.rs` with non-normalizing LERP for quaternion blending
- [ ] 2.4 Add fabric module to `crates/cube/src/lib.rs` exports
- [ ] 2.5 Add `glam` quaternion dependency verification (already present via glam crate)

## 3. Quaternion Interpolation Implementation
- [ ] 3.1 Implement `lerp_quaternion(q1, q2, t)` non-normalizing linear interpolation
- [ ] 3.2 Implement `octant_rotation(octant_index) -> Quat` returning ±90° per axis based on octant bits
- [ ] 3.3 Implement `octant_offset(octant_index) -> Vec3` returning position offset for child center
- [ ] 3.4 Implement `magnitude_from_distance(distance, config) -> f32`:
  - t = distance / surface_radius
  - magnitude = lerp(root_magnitude, boundary_magnitude, t)
- [ ] 3.5 Implement `calculate_child_quaternion(parent_rotation, world_pos, config)`:
  - rotation = parent_rotation * octant_rotation[octant]
  - magnitude = magnitude_from_distance(length(world_pos), config)
  - Q_child = rotation.normalize() * magnitude
- [ ] 3.6 Implement `apply_additive_state(base_quat, additive_state, position)` with rotation and magnitude noise
- [ ] 3.7 Add unit tests verifying:
  - Octant rotation encodes position (different octants → different rotations)
  - Magnitude computed from Euclidean distance (spherical isosurface)
  - Surface emerges where |Q| crosses 1.0

## 4. Fabric Cube Generator
- [ ] 4.1 Create `crates/cube/src/fabric/generator.rs`
- [ ] 4.2 Implement `FabricGenerator` struct holding FabricConfig (root_magnitude, boundary_magnitude, surface_radius, additive_states)
- [ ] 4.3 Implement `generate_cube(depth) -> Cube<Quat>` recursive generation:
  - Track world position during recursion (start at origin, add octant_offset per level)
  - Compute magnitude from Euclidean distance to origin
  - Apply octant rotation for position encoding
- [ ] 4.4 Implement lazy/cached quaternion evaluation for performance
- [ ] 4.5 Add generation tests verifying:
  - Root magnitude matches config (|Q| < 1 = solid at origin)
  - Magnitude increases with distance from origin (spherical gradient)
  - Surface (|Q| = 1.0) forms a sphere at expected radius

## 5. Surface Detection and Normals
- [ ] 5.1 Create `crates/cube/src/fabric/surface.rs`
- [ ] 5.2 Implement `is_surface(current_quat, neighbor_quat) -> bool` using magnitude threshold crossing (|Q| crosses 1.0)
- [ ] 5.3 Implement `calculate_normal(position, fabric_cube, depth) -> Vec3` using magnitude gradient
- [ ] 5.4 Implement `quaternion_to_color(quat) -> [u8; 3]` for HSV color mapping
- [ ] 5.5 Add surface detection tests with known geometries (verify sphere-like surface from decay)

## 6. Max Depth Rendering Support
- [ ] 6.1 Add `max_depth: Option<u32>` field to raycast/traversal context structures
- [ ] 6.2 Modify traversal to treat nodes at max_depth as leaves
- [ ] 6.3 Update CPU tracer to respect max_depth parameter
- [ ] 6.4 Update BCF tracer to respect max_depth parameter
- [ ] 6.5 Add tests for max_depth early termination

## 7. Renderer Configuration Refactor
- [ ] 7.1 Rename `crates/renderer/models.ron` to `crates/renderer/config.ron`
- [ ] 7.2 Extend `ModelsConfig` struct in `model_config.rs` to `RendererConfig`
- [ ] 7.3 Add `SingleCubeConfig` section with default_material field
- [ ] 7.4 Add `FabricConfig` section with root_magnitude, boundary_magnitude, additive_states (rotation + magnitude), and default_max_depth
- [ ] 7.5 Add `RenderingConfig` section for resolution and future settings
- [ ] 7.6 Update all config loading code to use new structure
- [ ] 7.7 Add config validation and helpful error messages

## 8. Model Selector Page UI
- [ ] 8.1 Create `ModelSelectorPanel` struct in `egui_app.rs` or new file
- [ ] 8.2 Implement collapsible category sections (Single Cube, VOX, CSM, Fabric)
- [ ] 8.3 Move model dropdown logic from top panel to category sections
- [ ] 8.4 Add Single Cube material selector within its category
- [ ] 8.5 Add Fabric parameters UI (root/boundary magnitude sliders, additive states editor, max depth slider)
- [ ] 8.6 Implement side panel layout with render views in center
- [ ] 8.7 Update egui_app to use new panel-based layout

## 9. Fabric Model Integration
- [ ] 9.1 Add "Fabric" model type to `ModelEntry` enum
- [ ] 9.2 Implement fabric cube creation in `ModelEntry::create_cube()`
- [ ] 9.3 Add fabric-specific raycast hit handling (use quaternion normal/color)
- [ ] 9.4 Add fabric model entries to config.ron
- [ ] 9.5 Test fabric rendering in all renderer modes (CPU, GL, BCF, Mesh)

## 10. Validation and Documentation
- [ ] 10.1 Run `cargo test --workspace` to verify all tests pass
- [ ] 10.2 Run `cargo clippy --workspace` to fix warnings
- [ ] 10.3 Test fabric rendering with various additive state configurations
- [ ] 10.4 Verify model selector UI works correctly
- [ ] 10.5 Update renderer --help text with new options
