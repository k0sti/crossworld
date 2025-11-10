# Raycast System Design

## Overview

The raycast system provides efficient ray-octree intersection for voxel rendering. It consists of two main components:

1. **Octree Raycast** (`crates/cube/src/raycast/mod.rs`) - Hierarchical traversal of Cube octrees
2. **Renderer Raycast** (`crates/renderer/src/gpu_tracer.rs`) - Stub implementation for GPU/CPU raytracers

## Architecture

```
┌─────────────────────────────────────────┐
│  Renderer (cpu_tracer.rs)              │
│  ┌───────────────────────────────────┐  │
│  │ render_ray(ray: Ray)              │  │
│  │   1. Bounding box intersection    │  │
│  │   2. Recursive octree raycast     │  │
│  │   3. Lighting calculation         │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────┐
│  GPU Tracer (gpu_tracer.rs)            │
│  ┌───────────────────────────────────┐  │
│  │ raycast(cube, pos, dir)           │  │
│  │   - Traverse octree structure     │  │
│  │   - Find first solid voxel        │  │
│  │   - Return hit information        │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────┐
│  Cube Raycast (cube/raycast/mod.rs)    │
│  ┌───────────────────────────────────┐  │
│  │ Cube::raycast(pos, dir, depth)   │  │
│  │   - Recursive octree traversal    │  │
│  │   - DDA-based stepping            │  │
│  │   - Normal calculation            │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

## Coordinate Systems

### 1. World Space
- Ray origin and direction in world coordinates
- Bounding box intersection in world space

### 2. Normalized Cube Space [0, 1]³
- Octree traversal uses normalized coordinates
- Position (0, 0, 0) = cube minimum corner
- Position (1, 1, 1) = cube maximum corner

### 3. Octree Coordinate Space
- `CubeCoord { pos: IVec3, depth: u32 }`
- Position encoded as Morton code / octant path
- Depth = 0 is root, increasing depth = smaller voxels

## Raycast Algorithm

### High-Level Flow

```rust
fn render_ray(ray: Ray) -> Vec3 {
    // Step 1: Bounding box intersection
    let hit = intersect_box(ray, cube_bounds.min, cube_bounds.max);

    if !hit.hit {
        return background_color;
    }

    // Step 2: Recursive octree raycast
    let cube = gpu_tracer.cube();
    let result = raycast(cube, hit.point, ray.direction);

    if result.hit {
        // Step 3: Lighting calculation
        return calculate_lighting(&result, ray.direction, light_dir);
    }

    return background_color;
}
```

### Octree Traversal Algorithm

The octree raycast uses a recursive DDA (Digital Differential Analyzer) approach:

```
function raycast_recursive(cube, pos, dir, depth):
    # Validate position is in [0, 1]³
    if pos not in [0, 1]³:
        return None

    # Check cube type
    match cube:
        Cube::Solid(value):
            if value != 0:  # Non-empty voxel
                normal = calculate_entry_normal(pos, dir)
                return Hit(coord, pos, normal)
            else:
                return None  # Empty voxel

        Cube::Cubes(children) if depth > 0:
            # Calculate which octant we're in
            pos2 = pos * 2.0
            sign = dir.signum()
            bit = floor(pos2 * sign) adjusted by sign

            # Check octant validity
            if bit not in [0, 1]³:
                return None

            # Calculate octant index (0-7)
            index = (bit.x << 2) | (bit.y << 1) | bit.z

            # Transform to child coordinate space
            child_pos = (pos2 - bit) / 2.0
            child_octree_pos = (octree_pos << 1) + bit

            # Recursively raycast into child
            if hit = children[index].raycast_recursive(
                child_pos, dir, depth-1, child_octree_pos):
                return hit

            # Miss in this octant - step to next boundary
            next_pos = calculate_next_octant_position(pos2, dir, sign)

            # Continue raycasting from new position
            return raycast_recursive(cube, next_pos, dir, depth)

        _:
            # Max depth or unsupported structure
            return None
```

### Key Components

#### 1. Octant Indexing

Octants are numbered 0-7 based on position relative to center:

```
Octant bit encoding: (x_bit << 2) | (y_bit << 1) | z_bit

  z
  |   y
  |  /
  | /
  |/____x

Octant 0 (000): (-x, -y, -z) - back  bottom left
Octant 1 (001): (-x, -y, +z) - front bottom left
Octant 2 (010): (-x, +y, -z) - back  top    left
Octant 3 (011): (-x, +y, +z) - front top    left
Octant 4 (100): (+x, -y, -z) - back  bottom right
Octant 5 (101): (+x, -y, +z) - front bottom right
Octant 6 (110): (+x, +y, -z) - back  top    right
Octant 7 (111): (+x, +y, +z) - front top    right
```

#### 2. DDA Stepping

When missing in current octant, step to the next boundary:

```rust
fn next_integer_boundary(v: Vec3, sign: Vec3) -> Vec3 {
    let scaled = v * sign + Vec3::ONE;
    scaled.floor() * sign
}

fn calculate_next_position(pos2: Vec3, dir: Vec3, sign: Vec3) -> Vec3 {
    let next_integer = next_integer_boundary(pos2, sign);
    let diff = next_integer - pos2;

    // Calculate step size using inverse time
    let inv_time = dir / diff;
    let max_inv = max(inv_time.x, inv_time.y, inv_time.z);

    let step = diff * (inv_time / max_inv);
    let next_pos = (pos2 + step) / 2.0;

    clamp(next_pos, Vec3::ZERO, Vec3::ONE)
}
```

#### 3. Normal Calculation

Surface normal is determined by entry face:

```rust
fn calculate_entry_normal(pos: Vec3, _dir: Vec3) -> Vec3 {
    let dist_to_min = pos;
    let dist_to_max = Vec3::ONE - pos;

    let min_dist = min_element(dist_to_min);
    let max_dist = min_element(dist_to_max);

    if min_dist < max_dist {
        // Entered from min face (0, 0, 0)
        if dist_to_min.x == min_dist: return Vec3::new(-1, 0, 0)
        if dist_to_min.y == min_dist: return Vec3::new(0, -1, 0)
        if dist_to_min.z == min_dist: return Vec3::new(0, 0, -1)
    } else {
        // Entered from max face (1, 1, 1)
        if dist_to_max.x == max_dist: return Vec3::new(1, 0, 0)
        if dist_to_max.y == max_dist: return Vec3::new(0, 1, 0)
        if dist_to_max.z == max_dist: return Vec3::new(0, 0, 1)
    }
}
```

## Data Structures

### RaycastHit (Renderer)

```rust
struct RaycastHit {
    hit: bool,              // Did we hit something?
    t: f32,                 // Distance along ray
    point: Vec3,            // Hit point in world space
    normal: Vec3,           // Surface normal
    voxel_pos: IVec3,       // Voxel position (integer)
    voxel_value: i32,       // Voxel ID/color
}
```

### RaycastHit (Cube Library)

```rust
struct RaycastHit {
    coord: CubeCoord,       // Octree coordinate
    position: Vec3,         // Hit position in [0,1]³ space
    normal: Vec3,           // Surface normal
}

struct CubeCoord {
    pos: IVec3,             // Octree position (Morton code)
    depth: u32,             // Depth in octree (0 = root)
}
```

### Cube Structure

```rust
enum Cube<T> {
    Solid(T),                           // Leaf voxel with value
    Cubes(Box<[Rc<Cube<T>>; 8]>),     // 8 child octants
    Planes { axis, quad },              // 2D quadtree subdivision
    Slices { axis, layers },            // Layered subdivision
}
```

## Performance Considerations

### 1. Early Termination
- Exit immediately on first solid voxel hit
- No need to traverse entire octree

### 2. Depth Limiting
- `max_depth` parameter limits traversal
- Prevents infinite recursion in degenerate cases

### 3. Empty Space Skipping
- DDA stepping efficiently skips empty octants
- No need to recursively traverse empty regions

### 4. Coordinate Clamping
- Clamp positions to [0, 1]³ to prevent out-of-bounds
- Handle floating-point precision issues

## Edge Cases & Robustness

### Division by Zero
**Problem:** Ray direction components can be zero (axis-aligned rays)

**Solution:** Check for epsilon before division:
```rust
const EPSILON: f32 = 1e-8;

if ray_direction[i].abs() > EPSILON {
    let inv_dir = 1.0 / ray_direction[i];
    // ... safe to use inv_dir
}
```

### Grazing Rays
**Problem:** Rays tangent to cube faces can have precision issues

**Solution:**
- Use epsilon comparisons for boundary tests
- Clamp positions to valid range

### Depth 0 Handling
**Problem:** Root cube at depth 0 needs special handling

**Solution:**
- Check `if current_depth > 0` before subdividing
- At depth 0, treat as solid leaf

### Floating-Point Precision
**Problem:** Accumulated errors in recursive traversal

**Solution:**
- Renormalize positions when entering children
- Clamp to [0, 1]³ after each step

## Future Optimizations

### 1. Early Ray Termination
- Stop at maximum distance `t_max`
- Useful for shadow rays and AO

### 2. Beam Optimization
- For coherent rays, trace multiple rays together
- Amortize octree traversal cost

### 3. GPU Implementation
- Port algorithm to compute shader
- Parallel raycast for all pixels

### 4. Spatial Hashing
- Cache octree nodes in spatial hash
- Faster lookups for repeated traversal

### 5. Ray Packet Traversal
- SIMD optimization for 4-8 rays
- Shared octant tests

## Testing Strategy

### Unit Tests (cube/raycast/mod.rs)

1. **Basic Raycasts**
   - Solid voxel hit
   - Empty voxel miss
   - Boundary conditions

2. **Axis-Aligned Rays**
   - X, Y, Z positive/negative directions
   - Verify no infinity/NaN

3. **Subdivided Cubes**
   - Traverse multiple octants
   - Correct child selection
   - Proper normal calculation

4. **Depth Testing**
   - Depth 0, 1, 2, 3 cubes
   - Max depth limiting

5. **Edge Cases**
   - Grazing rays (tangent to faces)
   - Corner rays
   - Near-zero direction components

### Integration Tests (renderer)

1. **Full Render Pipeline**
   - Bounding box → octree → lighting
   - Verify visual output

2. **CubScript Integration**
   - Parse cube from script
   - Raycast through parsed structure

3. **Performance Tests**
   - Large octrees (depth 5-6)
   - Many rays (full screen)
   - Profile hotspots

## References

### Existing Implementation
- `crates/cube/src/raycast/mod.rs` - Octree raycast
- `crates/renderer/src/gpu_tracer.rs` - Renderer stub
- `crates/renderer/src/cpu_tracer.rs` - CPU raytracer

### Documentation
- `RAYCAST_TEST_FINDINGS.md` - Bug fixes and test results
- `packages/app/src/utils/worldRaycast.ts` - TypeScript reference

### Papers & Resources
- "An Efficient Parametric Algorithm for Octree Traversal" - Revelles et al.
- "A Fast Voxel Traversal Algorithm for Ray Tracing" - Amanatides & Woo
- "Efficient Sparse Voxel Octrees" - Laine & Karras

## Implementation Status

### ✅ Completed
- [x] Octree traversal algorithm
- [x] Normal calculation
- [x] Depth limiting
- [x] Axis-aligned ray handling
- [x] Bounding box intersection
- [x] Recursive raycast integration

### 🚧 Stub Implementation
- [ ] GPU shader version
- [ ] Voxel value extraction
- [ ] Hit point refinement
- [ ] Distance calculation

### 📋 TODO
- [ ] Beam optimization
- [ ] Ray packet SIMD
- [ ] Performance profiling
- [ ] GPU compute shader
- [ ] Shadow rays
- [ ] Ambient occlusion

## Usage Example

```rust
use crossworld_cube::{parse_csm, Cube};
use crate::gpu_tracer::raycast;

// Parse cube from cubscript
let cubscript = ">a [1 2 3 4 5 6 7 8]";
let octree = parse_csm(cubscript).unwrap();
let cube = Rc::new(octree.root);

// Setup ray
let ray_origin = Vec3::new(0.5, 0.5, -1.0);
let ray_direction = Vec3::new(0.0, 0.0, 1.0);

// Bounding box intersection
let bounds = CubeBounds::default();
let hit = intersect_box(
    Ray { origin: ray_origin, direction: ray_direction },
    bounds.min,
    bounds.max
);

if hit.hit {
    // Recursive octree raycast
    let result = raycast(&cube, hit.point, ray_direction);

    if result.hit {
        println!("Hit voxel {} at {:?}",
                 result.voxel_value,
                 result.point);
    }
}
```
