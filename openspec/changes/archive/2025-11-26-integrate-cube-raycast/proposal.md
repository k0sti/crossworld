# Change: Integrate Cube Raycast into CPU Tracer

## Why

The `crates/cube` raycast implementation was recently reimplemented from scratch with comprehensive testing (30 tests, all passing). However, the renderer's CPU tracer still uses a stub raycast function in `gpu_tracer.rs` that only handles solid cubes and returns miss for subdivided octrees.

**Current situation:**
- `cube::Cube::raycast()` - Fully working octree traversal with DDA algorithm
- `gpu_tracer::raycast()` - Stub that returns `hit=false` for all `Cube::Cubes` variants
- `cpu_tracer.rs` calls the stub, resulting in incorrect rendering for subdivided octrees

**Impact:** CPU tracer cannot render subdivided octrees, limiting visual output to simple solid cubes.

**Scope:** Replace stub raycast with cube library integration. Keep GPU tracer structure unchanged for future GPU implementation.

## What Changes

### Phase 1: Enhance `cube::RaycastHit` Structure
- **`crates/cube/src/raycast/mod.rs`** - Add voxel value to RaycastHit
  - Make `RaycastHit` generic: `RaycastHit<T>`
  - Add `value: T` field containing the hit voxel's value
  - Update `raycast_recursive()` to extract and return voxel value
  - Update all 30 tests to work with generic structure
  - Maintain backward compatibility

### Phase 2: Integrate into CPU Tracer
- **`crates/renderer/src/cpu_tracer.rs`** - Integrate cube raycast directly
  - Replace `gpu_tracer::raycast()` call with direct `cube::Cube::raycast()`
  - Transform between coordinate systems (world space ↔ normalized [0,1]³)
  - Convert `cube::raycast::RaycastHit<i32>` to renderer hit info for lighting
  - Add `is_empty` predicate (voxel value == 0)
  - Set proper `max_depth` for traversal
  - Remove dependency on `gpu_tracer::raycast()` stub
  - Access voxel value directly from hit result (future: for materials)

### Not Changed
- Cube raycast implementation (already complete)
- `gpu_tracer.rs` stub (leave for future GPU shader implementation)
- Renderer API or lighting calculations
- Camera, bounding box, or other renderer systems

## Impact

### Affected Specs
- **NEW**: `renderer-raycast` - Spec for renderer raycast integration

### Dependencies
- Requires completed `reimplement-raycast` change (already applied)
- Uses `cube::raycast::RaycastHit` structure
- Uses `cube::Cube::raycast()` method

### Breaking Changes
None - API remains compatible, only implementation changes

### Success Criteria
- CPU tracer correctly renders subdivided octrees
- All existing renderer tests pass (if any)
- Visual output matches expected raycast behavior
- No performance regressions
- Code passes clippy with no warnings
