# RaycastHit Structures - Why Two Different Types?

## Question
Could `cube::raycast::RaycastHit` and `gpu_tracer::RaycastHit` be unified into a single structure?

## Answer: No - They Serve Different Purposes

### `cube::raycast::RaycastHit` - Library Primitive
**Purpose**: Pure octree traversal result
**Location**: `crates/cube/src/raycast/mod.rs`

```rust
pub struct RaycastHit {
    pub coord: CubeCoord,    // Precise octree coordinate (path from root)
    pub position: Vec3,      // Hit position in normalized [0,1]³ space
    pub normal: Vec3,        // Surface normal
}
// Returns: Option<RaycastHit> (None = miss)
```

**Characteristics:**
- ✅ Coordinate-system agnostic (normalized [0,1]³ space)
- ✅ No rendering assumptions
- ✅ Reusable in any context (GPU, CPU, physics, collision detection)
- ✅ Minimal dependencies (just `glam` for math)
- ✅ Returns `Option<T>` for Rust-idiomatic miss handling
- ✅ Contains precise octree path (`CubeCoord`)

**Use cases:**
- Octree raycast library (current)
- Physics collision detection (future)
- Voxel picking/selection (future)
- GPU compute shaders (future)

---

### `gpu_tracer::RaycastHit` - Renderer-Specific Structure
**Purpose**: Rendering and shader interop
**Location**: `crates/renderer/src/gpu_tracer.rs`

```rust
pub struct RaycastHit {
    pub hit: bool,           // Explicit miss flag (shader convention)
    pub t: f32,              // Distance along ray (for depth testing)
    pub point: Vec3,         // Hit point in world space (not normalized)
    pub normal: Vec3,        // Surface normal
    pub voxel_pos: IVec3,    // Simplified integer voxel position
    pub voxel_value: i32,    // Material/color ID for rendering
}
// Always returns a value, check .hit field
```

**Characteristics:**
- ✅ World-space oriented (renderer coordinates)
- ✅ Contains rendering metadata (`t`, `voxel_value`)
- ✅ Matches GPU shader conventions (`hit` bool flag)
- ✅ Includes material/color data for lighting
- ✅ Simplified position (`IVec3` vs `CubeCoord`)
- ✅ Always returns a value (never `Option`)

**Use cases:**
- GPU shader raytracing (future)
- CPU renderer lighting (current)
- Material/color lookup
- Depth buffer calculations

---

## Why Keep Them Separate?

### 1. Separation of Concerns
- **Cube library** = Pure voxel data structure operations
- **Renderer** = Graphics, materials, lighting, shading

### 2. Different Coordinate Systems
- **Cube**: Normalized [0,1]³ (coordinate-system independent)
- **Renderer**: World space (specific to scene configuration)

### 3. Different Return Semantics
- **Cube**: `Option<RaycastHit>` (Rust-idiomatic)
- **Renderer**: Always returns value with `hit` flag (GPU-friendly)

### 4. Different Use Cases
- **Cube**: Reusable library for physics, collision, selection, etc.
- **Renderer**: Specific to graphics rendering pipeline

### 5. Different Dependencies
- **Cube**: Minimal (just `glam`)
- **Renderer**: Many (image, rendering pipeline, materials, etc.)

---

## For This Change (integrate-cube-raycast)

We'll convert between the two types in `cpu_tracer.rs`:

```rust
// After bounding box hit
let normalized_pos = (hit.point - bounds.min) / (bounds.max - bounds.min);

// Call cube raycast
let cube_hit = cube.raycast(normalized_pos, dir, max_depth, &is_empty);

// Convert to HitInfo for lighting
match cube_hit {
    None => self.background_color,
    Some(hit) => {
        let world_point = hit.position * (bounds.max - bounds.min) + bounds.min;
        let hit_info = HitInfo {
            hit: true,
            t: (world_point - ray.origin).length(),
            point: world_point,
            normal: hit.normal,
        };
        calculate_lighting(&hit_info, ray.direction, self.light_dir)
    }
}
```

**Note**: We don't need `gpu_tracer::RaycastHit` for CPU tracer - we convert directly from `cube::RaycastHit` to `HitInfo` for lighting.

---

## Future: GPU Shader Implementation

When implementing GPU raytracing, `gpu_tracer::RaycastHit` will be useful for shader interop:
- GPU shaders can't use Rust `Option<T>`
- GPU needs explicit `hit` bool flag
- GPU needs `t` value for depth testing
- GPU needs `voxel_value` for material lookups

The cube library raycast will remain the single source of truth for the algorithm, but GPU shaders will use their own data structures that match GPU conventions.
