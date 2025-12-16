# Design: Fabric Model Generation System

## Context
The fabric system generates continuous voxel surfaces using quaternion fields. Unlike discrete material IDs, quaternions enable smooth interpolation and gradient-based normal calculation. This is useful for procedural terrain, organic shapes, and mathematical surfaces.

## Goals
- Generate procedural voxel models via quaternion field evaluation
- Support multi-scale detail through additive quaternion states
- Integrate with existing renderer without modifying core raycast/mesh logic
- Provide intuitive UI for fabric parameter tuning

## Non-Goals
- Real-time animation of fabric surfaces (static generation only)
- Physics collision from fabric (use mesh conversion)
- Network serialization of fabric parameters

## Decisions

### Decision 1: Quaternion Value Calculation Formula

Child quaternion values are derived through hierarchical interpolation of intermediate points. The parent's influence (~1/4) emerges naturally from the interpolation chain.

**Step 1: Calculate 6 face midpoints**
```
Q_face[axis][side] = SLERP(Q_parent, Q_neighbor[axis][side], 0.5)

Where axis ∈ {X, Y, Z} and side ∈ {NEG, POS}
Example: Q_face[X][POS] = SLERP(Q_parent, Q_neighbor_pos_x, 0.5)
```

**Step 2: Calculate 12 edge midpoints**
```
Q_edge[axis][corner] = SLERP(Q_face[axis1][side1], Q_face[axis2][side2], 0.5)

Where the edge is the intersection of two faces perpendicular to axis.
Example: Q_edge[Z][0] = SLERP(Q_face[X][NEG], Q_face[Y][NEG], 0.5)
```

**Step 3: Calculate 8 corner points (octant centers)**
```
Q_corner[octant] = SLERP(SLERP(Q_edge[X][e1], Q_edge[Y][e2], 0.5), Q_edge[Z][e3], 0.33)

Where e1, e2, e3 are the edges adjacent to the octant's corner.
```

**Step 4: Child quaternion with additive noise**
```
Q_child[i] = Q_corner[i] * Q_additive[depth]
```

**Additive state contribution:**
```
additive_angle = additive_states[depth] * noise(position, depth)
Q_additive = Quaternion::from_axis_angle(random_axis, additive_angle)
```

**Why this works:**
- Face midpoint = 1/2 parent + 1/2 neighbor
- Edge midpoint = average of 2 faces → ~1/4 parent contribution
- Corner = average of 3 edges → maintains ~1/4 parent influence
- The 3/4 neighbor / 1/4 parent ratio emerges naturally from the hierarchy

**Alternatives considered:**
- NLERP: Faster but produces less smooth surfaces at low depths
- Direct linear blend: Doesn't preserve quaternion properties
- Explicit 0.75 weighting: Redundant given hierarchical interpolation

### Decision 2: Quaternion Encodes World Position via Octant Rotation

Each octant's quaternion is derived from its parent by applying a **positional rotation** based on the octant index. This creates a deterministic mapping from world position to quaternion rotation:

```
// Octant indices 0-7 map to corners of unit cube
// Each axis contributes +90° or -90° based on octant bit
octant_rotation[i] = Quat::from_euler(
    if (i & 1) != 0 { +90° } else { -90° },  // X-axis from bit 0
    if (i & 2) != 0 { +90° } else { -90° },  // Y-axis from bit 1
    if (i & 4) != 0 { +90° } else { -90° },  // Z-axis from bit 2
)
```

**Child quaternion calculation:**
```
// 1. Apply positional rotation (encodes spatial location)
Q_positioned = Q_parent * octant_rotation[octant_index]

// 2. Interpolate with blend factor (controls sharpness of transitions)
Q_blended = LERP(Q_parent, Q_positioned, blend_factor)  // Non-normalizing

// 3. Apply additive state (noise/variation per depth)
Q_child = Q_blended * Q_additive[depth]
```

**Position-to-quaternion properties:**
- Root quaternion Q_root at origin (typically identity with magnitude > 1)
- Each subdivision rotates by ±90° per axis based on octant
- Quaternion rotation component = "address" in rotation space
- Quaternion magnitude = field density value

### Decision 3: Dual-Purpose Quaternion (Rotation + Magnitude)

Quaternions are **not normalized**, encoding two properties:

1. **Rotation (direction)**: Encodes world position via accumulated octant rotations
2. **Magnitude (length)**: Encodes field density for surface detection

```
Q = (x, y, z, w)  where |Q| ≠ 1 in general

rotation_part = Q.normalize()  // For color, spatial queries
magnitude = |Q| = sqrt(x² + y² + z² + w²)  // For surface detection
```

**Surface detection (SDF convention):**
```
|Q| < 1  →  inside surface (solid)     // Equivalent to SDF < 0
|Q| > 1  →  outside surface (air/empty) // Equivalent to SDF > 0
|Q| = 1  →  surface boundary            // Equivalent to SDF = 0
```

This follows standard SDF sign convention where negative = inside, positive = outside.

**Magnitude from Euclidean Distance (Spherical Surface):**

To produce spherical surfaces, magnitude is computed from the **actual Euclidean distance** of the voxel center from world origin:

```
// World coordinates: origin at (0,0,0), world spans [-1, 1] in each axis
// Voxel center position is tracked during generation

distance = sqrt(x² + y² + z²)  // Euclidean distance from origin
max_distance = sqrt(3)          // Corner distance ≈ 1.732

// Linear interpolation based on distance
t = distance / surface_radius   // surface_radius is configurable (e.g., 0.8)

magnitude = root_magnitude + (boundary_magnitude - root_magnitude) * t
```

**Configuration for spherical model:**
```
root_magnitude = 0.5      // Center is solid (|Q| < 1)
boundary_magnitude = 2.0  // Edges are air (|Q| > 1)
surface_radius = 0.8      // Sphere surface at 80% of world half-size

// At distance = 0: magnitude = 0.5 (solid)
// At distance = surface_radius: magnitude ≈ 1.0 (surface!)
// At distance > surface_radius: magnitude > 1.0 (air)
```

**Child quaternion with position tracking:**
```
// Track world position during recursive generation
child_world_pos = parent_world_pos + octant_offset * (world_size / 2^depth)
child_distance = length(child_world_pos)

// Compute magnitude from distance
t = clamp(child_distance / surface_radius, 0.0, 1.0)
child_magnitude = lerp(root_magnitude, boundary_magnitude, t)

// Apply rotation for position encoding + color
Q_child = (Q_parent * octant_rotation[i]).normalize() * child_magnitude
```

This produces a true sphere where the surface (|Q| = 1.0) forms at `distance = surface_radius`.

### Decision 4: Normal Calculation from Magnitude Gradient

Normal vector derived from the gradient of quaternion magnitude using central differences:
```
gradient = Vec3(
    |Q(x+h)| - |Q(x-h)|,
    |Q(y+h)| - |Q(y-h)|,
    |Q(z+h)| - |Q(z-h)|
) / (2*h)

normal = -gradient.normalize()  // Points toward |Q| > 1 (outside/air), standard SDF convention
```

Where `h` is the voxel half-size at current depth.

The normal points from solid (|Q| < 1) toward air (|Q| > 1), following standard SDF convention.

### Decision 5: Color from Quaternion Rotation

Map quaternion rotation to HSV color:
```
// Extract rotation axis and angle
axis = Q.xyz.normalize()
angle = 2 * acos(Q.w)

// Map to HSV
hue = atan2(axis.y, axis.x) / (2*PI) + 0.5  // 0-1 range
saturation = axis.z.abs()  // Vertical component affects saturation
value = 0.5 + 0.5 * cos(angle)  // Angle affects brightness

color = hsv_to_rgb(hue, saturation, value)
```

### Decision 6: Max Depth Rendering Parameter

Add `max_depth: Option<u32>` to rendering context. When set:
- Traversal stops at `max_depth` regardless of octree structure
- All nodes at `max_depth` are treated as leaves
- Enables LOD control and performance tuning

### Decision 7: Renderer Config Unification

Single `config.ron` file structure:
```ron
(
    models: [...],  // Existing model entries

    single_cube: (
        default_material: 224,
    ),

    fabric: (
        root_magnitude: 0.5,           // Magnitude at origin (|Q| < 1 = inside/solid)
        boundary_magnitude: 2.0,       // Magnitude at max distance (|Q| > 1 = outside/air)
        surface_radius: 0.8,           // Distance where |Q| = 1.0 (surface), relative to world half-size
        additive_states: [
            (rotation: 0.0, magnitude: 0.0),   // depth 0
            (rotation: 0.1, magnitude: 0.05),  // depth 1
            (rotation: 0.2, magnitude: 0.1),   // depth 2
            // ...
        ],
        default_max_depth: 5,
        color_mode: "quaternion",  // or "depth"
    ),

    rendering: (
        default_resolution: [400, 300],
    ),
)
```

### Decision 8: Model Selector Page Layout

Replace top-bar dropdown with sidebar/page selector:
```
+------------------+------------------------+
| Model Categories | Render Views           |
|                  |                        |
| > Single Cube    | [CPU] [GL] [GPU]       |
|   - Material: _  |                        |
|                  | [BCF] [Mesh] [Diff]    |
| > VOX Models     |                        |
|   - alien_bot    |                        |
|   - chr_army     |                        |
|                  |                        |
| > CSM Models     |                        |
|   - octa         |                        |
|   - extended     |                        |
|                  |                        |
| > Fabric Models  |                        |
|   - Additive: _  |                        |
|   - Max Depth: _ |                        |
+------------------+------------------------+
```

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| SLERP performance at high depths | Cache intermediate quaternions, limit max depth |
| Quaternion precision loss | Use f64 for intermediate calculations if needed |
| Complex UI for fabric params | Start with simple array input, iterate based on usage |

## Migration Plan

1. Add fabric module without modifying existing code
2. Extend Cube<T> with `fn value(&self) -> Option<&T>`
3. Update renderer config (rename models.ron -> config.ron)
4. Implement model selector page
5. Integrate fabric rendering

No breaking changes to existing functionality.

## Open Questions

1. Should we support custom noise functions for additive component?
2. Should fabric models support saving/loading quaternion field state?
3. Should boundary magnitude be configurable per-axis (allowing non-spherical base shapes)?
