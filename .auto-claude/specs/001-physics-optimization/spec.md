# Specification: Proto-GL Terrain Collision System Optimization

## Overview

The proto-gl terrain collision system uses Rapier physics with a `VoxelTerrainCollider` that generates triangle geometry from octree voxel data. While functionally correct, the system exhibits performance issues due to frequent BVH rebuilds, triangle regeneration, and excessive memory allocations during active region updates. This optimization effort aims to identify and implement performance improvements while maintaining collision accuracy and existing behavior.

## Workflow Type

**Type**: feature (optimization work)

**Rationale**: This is performance optimization of an existing, working system. It requires profiling, identifying bottlenecks, and implementing improvements without breaking functionality. This follows the feature workflow as it involves significant code changes across multiple files with measurable acceptance criteria.

## Task Scope

### Services Involved
- **physics** (primary) - Rust crate containing terrain collision system (`crates/physics/`)
- **proto-gl** (integration) - OpenGL prototype application that uses the physics system (`crates/proto-gl/`)
- **cube** (reference) - Octree voxel data structures and traversal (`crates/cube/`)

### This Task Will:
- [ ] Profile current terrain collision performance to establish baseline metrics
- [ ] Identify specific bottlenecks in `VoxelTerrainCollider::update_triangle_bvh()`
- [ ] Optimize triangle BVH rebuild frequency and cost
- [ ] Reduce memory allocations in region caching and triangle generation
- [ ] Implement incremental BVH updates where possible
- [ ] Optimize `ActiveRegionTracker` to reduce unnecessary rebuilds
- [ ] Update benchmarks to validate improvements

### Out of Scope:
- Changing the fundamental collision algorithm (still uses TypedCompositeShape)
- Modifying Rapier physics engine internals
- Altering voxel octree data structures in the cube crate
- Graphics/rendering optimizations (only physics collision)
- WebAssembly-specific optimizations

## Service Context

### Physics Crate

**Tech Stack:**
- Language: Rust
- Framework: Rapier 3D physics engine
- Key directories: `src/terrain/`, `src/`

**Entry Point:** `src/lib.rs`

**How to Run:**
```bash
# Run tests
cargo test -p crossworld-physics

# Run benchmarks
cargo bench --bench collision -p crossworld-physics
```

**Port:** N/A (library crate)

### Proto-GL Application

**Tech Stack:**
- Language: Rust
- Framework: glow/glutin (OpenGL), winit, egui
- Key directories: `src/`

**Entry Point:** `src/main.rs`

**How to Run:**
```bash
cd crates/proto-gl && cargo run
```

**Port:** N/A (native desktop application)

## Files to Modify

| File | Service | What to Change |
|------|---------|---------------|
| `crates/physics/src/terrain/collider.rs` | physics | Optimize `update_triangle_bvh()`, reduce allocations, implement incremental updates |
| `crates/physics/src/terrain/region_cache.rs` | physics | Optimize `RegionCollisionData::from_octree()`, cache triangle data not just faces |
| `crates/physics/src/terrain/active_region.rs` | physics | Reduce rebuild trigger frequency, add hysteresis to region tracking |
| `crates/physics/src/terrain/triangle_gen.rs` | physics | Pre-compute triangles, reduce per-frame allocations |
| `crates/physics/benches/collision.rs` | physics | Add detailed per-component benchmarks for optimization validation |
| `crates/proto-gl/src/app.rs` | proto-gl | Potentially optimize terrain collider update loop timing |

## Files to Reference

These files show patterns to follow:

| File | Pattern to Copy |
|------|----------------|
| `crates/physics/src/world_collider.rs` | Alternative collision strategy (direct octree queries) |
| `openspec/changes/optimize-world-collision/design.md` | Existing optimization strategies documentation |
| `doc/architecture/physics.md` | Architecture patterns and design decisions |
| `crates/physics/benches/collision.rs` | Benchmark patterns using Criterion |

## Patterns to Follow

### BVH Construction Pattern

From `crates/physics/src/terrain/collider.rs`:

```rust
pub fn update_triangle_bvh(&mut self, active_aabb: &Aabb) {
    let mut aabbs: Vec<Aabb> = Vec::new();
    let mut part_ids: Vec<TerrainPartId> = Vec::new();

    // Find all regions intersecting active area
    for region in RegionId::from_world_aabb(active_aabb, self.world_size, self.region_depth) {
        // Ensure region is cached
        let data = self.region_cache.entry(region).or_insert_with(|| {
            RegionCollisionData::from_octree(...)
        });
        // ... add triangles
    }

    // Rebuild the triangle BVH
    self.triangle_bvh = Bvh::from_leaves(BvhBuildStrategy::Binned, &aabbs);
}
```

**Key Points:**
- Allocates new vectors every call - opportunity for reuse
- Full BVH rebuild even for small changes - opportunity for incremental update
- Region caching helps but triangles regenerated each time

### Region Cache Pattern

From `crates/physics/src/terrain/region_cache.rs`:

```rust
pub struct RegionCollisionData {
    pub region: RegionId,
    pub aabb: Aabb,
    pub faces: Vec<FaceInfo>,  // Faces stored, triangles computed on-demand
    pub version: u64,
}
```

**Key Points:**
- Faces are cached but triangles computed per-query
- Could pre-compute and cache triangles alongside faces
- Version tracking exists but not fully utilized

### Active Region Tracking Pattern

From `crates/physics/src/terrain/active_region.rs`:

```rust
impl ActiveRegionTracker {
    pub fn update(&mut self, dynamic_aabbs: &[Aabb]) -> Option<Aabb> {
        // Returns Some(new_aabb) if rebuild needed, None otherwise
    }
}
```

**Key Points:**
- Triggers rebuild when combined AABB changes significantly
- No hysteresis - small movements can trigger rebuilds
- Consider movement prediction or larger buffer zones

## Requirements

### Functional Requirements

1. **Maintain Collision Accuracy**
   - Description: Collision detection must produce identical results before and after optimization
   - Acceptance: All existing physics tests pass, objects land on terrain correctly in proto-gl

2. **Reduce Per-Frame Update Time**
   - Description: `update_triangle_bvh()` should complete faster during active simulation
   - Acceptance: Benchmark shows measurable improvement (target: 50%+ reduction)

3. **Reduce Memory Allocations**
   - Description: Minimize heap allocations during physics frame updates
   - Acceptance: Memory profiling shows reduced allocation count per frame

4. **Maintain Existing API**
   - Description: External API of `VoxelTerrainCollider` remains unchanged
   - Acceptance: proto-gl compiles and runs without modifications to physics calls

### Edge Cases

1. **Large Active Regions** - When many objects spread across world, BVH may need to cover most terrain
2. **Rapid Object Movement** - Fast-moving objects may cross region boundaries frequently
3. **Terrain Modifications** - Cache invalidation must work correctly when voxels change
4. **Empty Regions** - Skip work for regions with no collision geometry
5. **First Frame** - Initial BVH build must complete in reasonable time

## Implementation Notes

### DO
- Follow the existing Rapier integration patterns in `shape_impl.rs`
- Reuse allocated vectors across frames using `clear()` instead of `new()`
- Profile before and after each optimization to measure impact
- Add micro-benchmarks for specific functions being optimized
- Use `#[inline]` hints for hot path functions
- Consider `BvhBuildStrategy::Balanced` vs `Binned` for different workloads

### DON'T
- Create new allocations in the per-frame physics loop when reuse is possible
- Change the TypedCompositeShape trait implementation contract
- Remove existing caching without measuring impact
- Optimize prematurely - profile first, then optimize measured bottlenecks
- Break compatibility with existing proto-gl configuration options

## Development Environment

### Start Services

```bash
# Build physics crate
cargo build -p crossworld-physics

# Run proto-gl with default config
cd crates/proto-gl && cargo run

# Run with debug output
cd crates/proto-gl && cargo run -- --debug
```

### Service URLs
- Proto-GL: Native desktop window (no URL)

### Required Environment Variables
- None required (uses config.toml in crates/proto-gl/)

### Key Configuration (config.toml)
```toml
[physics]
gravity = -9.81
timestep = 0.016666  # 60 Hz

[spawning]
spawn_count = 50  # Number of dynamic objects
```

## Success Criteria

The task is complete when:

1. [ ] `cargo bench --bench collision` shows measurable improvement in `world_collider_frame`
2. [ ] proto-gl runs at stable 60 FPS with 50 objects
3. [ ] All existing tests in `crates/physics/` pass
4. [ ] Objects still land correctly on terrain (visual verification)
5. [ ] No console errors or warnings during normal operation
6. [ ] Memory usage does not increase (stable or reduced)

## QA Acceptance Criteria

**CRITICAL**: These criteria must be verified by the QA Agent before sign-off.

### Unit Tests
| Test | File | What to Verify |
|------|------|----------------|
| test_update_triangle_bvh | `crates/physics/src/terrain/collider.rs` | BVH correctly indexes triangles after update |
| test_composite_shape_bvh | `crates/physics/src/terrain/shape_impl.rs` | TypedCompositeShape trait works correctly |
| test_cache_region | `crates/physics/src/terrain/collider.rs` | Region caching produces valid collision data |
| test_face_to_triangles_* | `crates/physics/src/terrain/triangle_gen.rs` | Triangle generation has correct normals |

### Integration Tests
| Test | Services | What to Verify |
|------|----------|----------------|
| Physics frame benchmark | physics | update + step time within acceptable range |
| Collision resolution | physics | Objects correctly collide with terrain voxels |
| Region tracking | physics | ActiveRegionTracker correctly triggers rebuilds |

### Performance Benchmarks
| Benchmark | Target | What to Verify |
|-----------|--------|----------------|
| world_collider_init | < 100ms | Initialization completes quickly |
| world_collider_frame | 50%+ improvement | Per-frame update time reduced |
| world_collider_new | < 1us | Constructor is fast |

### Browser Verification (if frontend)
| Page/Component | URL | Checks |
|----------------|-----|--------|
| N/A | N/A | This is a native desktop application |

### Native Application Verification
| Check | How | Expected |
|-------|-----|----------|
| Stable frame rate | Run proto-gl, watch FPS counter | 60 FPS stable |
| Object collision | Spawn objects, watch them fall | Objects land on terrain, don't fall through |
| No visual glitches | Move camera around terrain | No flickering, correct collision shapes |

### Database Verification (if applicable)
| Check | Query/Command | Expected |
|-------|---------------|----------|
| N/A | N/A | No database in this system |

### QA Sign-off Requirements
- [ ] All unit tests pass (`cargo test -p crossworld-physics`)
- [ ] Benchmark shows improvement (`cargo bench --bench collision`)
- [ ] proto-gl runs without errors for 60 seconds
- [ ] Objects correctly collide with terrain
- [ ] No regressions in existing functionality
- [ ] Code follows established patterns in physics crate
- [ ] No memory leaks (stable memory usage over time)

## Technical Details

### Current Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Per Physics Frame                      │
├─────────────────────────────────────────────────────────┤
│  1. Collect dynamic object AABBs                        │
│  2. ActiveRegionTracker.update() → triggers rebuild?    │
│  3. VoxelTerrainCollider.update_triangle_bvh()          │
│     └── For each region in active AABB:                 │
│         └── Cache miss: visit_faces_in_region()         │
│         └── Generate triangles from faces               │
│         └── Build AABB list for BVH                     │
│     └── Bvh::from_leaves() → Full rebuild               │
│  4. physics_world.update_terrain_collider()             │
│     └── Remove old Rapier collider                      │
│     └── Create new trimesh from triangles               │
│     └── Add new Rapier collider                         │
│  5. physics_world.step()                                │
└─────────────────────────────────────────────────────────┘
```

### Optimization Opportunities

1. **Reuse Allocations**: `aabbs` and `part_ids` vectors allocated each call
2. **Incremental BVH**: Only rebuild affected subtrees when regions change
3. **Triangle Caching**: Store pre-computed triangles in RegionCollisionData
4. **Hysteresis**: Add buffer zone to ActiveRegionTracker to reduce thrashing
5. **Batch Updates**: Defer Rapier collider updates when changes are minimal
6. **Pre-allocation**: Size vectors based on expected region count

### Performance Metrics to Track

- `update_triangle_bvh()` execution time
- Number of regions cached vs queried
- Triangle count per frame
- BVH rebuild frequency
- Memory allocation count in hot path
