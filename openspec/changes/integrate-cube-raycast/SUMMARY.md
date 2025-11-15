# OpenSpec Proposal Summary: integrate-cube-raycast

## ✅ Proposal Complete and Validated

**Status**: Ready for implementation
**Task Count**: 68 tasks across 3 phases
**Validation**: Passes `openspec validate --strict`

---

## What This Proposal Does

Integrates the newly reimplemented cube raycast into the renderer's CPU tracer in two phases:

### Phase 1: Enhance cube::RaycastHit (NEW!)
Make the cube library's `RaycastHit` structure **generic** and add **voxel value** field:

```rust
// BEFORE
pub struct RaycastHit {
    pub coord: CubeCoord,
    pub position: Vec3,
    pub normal: Vec3,
}

// AFTER ✨
pub struct RaycastHit<T> {
    pub coord: CubeCoord,
    pub position: Vec3,
    pub normal: Vec3,
    pub value: T,  // 🆕 Voxel value at hit
}
```

**Why**: Provides complete hit information without requiring tree traversal, enabling future material systems.

### Phase 2: Integrate into CPU Tracer
Replace stub raycast with direct cube library integration:

- **File**: `crates/renderer/src/cpu_tracer.rs`
- **Change**: Replace `gpu_tracer::raycast()` call with `cube.raycast()`
- **Transform**: World space ↔ normalized [0,1]³ coordinate conversion
- **Result**: Subdivided octrees render correctly (new capability!)

---

## Key Design Decisions

### 1. Why Make RaycastHit Generic?
**Your Question**: "Should cube raycast return the value?"

**Answer**: YES!
- ✅ Complete information in hit result
- ✅ No tree traversal needed
- ✅ Type-safe for any voxel type
- ✅ Enables future material systems
- ✅ Cleaner renderer integration

### 2. Why Two RaycastHit Types?
**Your Question**: "Could they be unified?"

**Answer**: NO - different purposes (see `RAYCAST_STRUCTURES.md`)
- `cube::RaycastHit<T>` = Library primitive (coordinate-agnostic)
- `gpu_tracer::RaycastHit` = Renderer-specific (world space, GPU-friendly)

### 3. Why Implement in cpu_tracer.rs Directly?
**Your Feedback**: "Do not use gpu_tracer.rs, implement directly in cpu file"

**Result**: Cleaner separation:
- ✅ No intermediate conversion layer
- ✅ CPU tracer is self-contained
- ✅ `gpu_tracer.rs` stub left for future GPU work
- ✅ Simpler implementation

---

## Implementation Phases

### Phase 1: Enhance cube::RaycastHit (23 tasks)
1. Make structure generic over `T`
2. Add `value: T` field
3. Update `raycast_recursive()` to extract voxel value
4. Update all 30 tests
5. Validate: all tests pass, no regressions

### Phase 2: Integrate into CPU Tracer (25 tasks)
1. Replace `gpu_tracer::raycast()` call
2. Add coordinate transformations (world ↔ normalized)
3. Call `cube.raycast()` with `is_empty` predicate
4. Convert result to `HitInfo` for lighting
5. Remove old fallback logic

### Phase 3: Testing & Validation (20 tasks)
1. Test solid cube rendering (baseline)
2. Test subdivided octree rendering (new!)
3. Test empty voxel filtering
4. Visual validation
5. Performance validation
6. Code quality checks

---

## Files Modified

### crates/cube/src/raycast/mod.rs
- Make `RaycastHit` generic: `RaycastHit<T>`
- Add `value: T` field
- Extract voxel value in `raycast_recursive()`
- Update 30 tests for generic structure

### crates/renderer/src/cpu_tracer.rs
- Remove `gpu_tracer::raycast()` dependency
- Add coordinate transformations
- Call `cube.raycast()` directly
- Convert to `HitInfo` for lighting

### Files NOT Changed
- `crates/renderer/src/gpu_tracer.rs` - stub remains for future GPU work
- Raycast algorithm - already complete from previous change

---

## Success Criteria

- ✅ `cube::RaycastHit` is generic with `value: T` field
- ✅ All 30 cube raycast tests pass
- ✅ All 78 total cube tests pass
- ✅ Solid cube rendering works (baseline maintained)
- ✅ Subdivided octree rendering works (new capability!)
- ✅ Empty voxels correctly filtered
- ✅ Voxel values accessible without traversal
- ✅ No dependency on `gpu_tracer::raycast()` stub
- ✅ No clippy warnings
- ✅ Code well-documented

---

## Next Steps

Ready to implement! Use:

```bash
# Start implementation
/openspec:apply

# Or view proposal details
openspec show integrate-cube-raycast
```

---

## Why This Approach?

Based on your feedback, this proposal:

1. ✅ **Adds voxel value to RaycastHit** - Complete hit information
2. ✅ **Implements directly in cpu_tracer.rs** - No intermediate layers
3. ✅ **Keeps structures separate** - Different purposes explained
4. ✅ **Future-proof** - Ready for material systems
5. ✅ **Clean separation** - GPU tracer stub untouched

The result: A complete, production-ready raycast integration with full voxel information available for future enhancements!
