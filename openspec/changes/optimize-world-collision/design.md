# Design: World Collision Optimization

## Problem Analysis

### Current Implementation

```
┌─────────────────────────────────────────────────────────┐
│                   Startup (Once)                        │
├─────────────────────────────────────────────────────────┤
│  VoxelColliderBuilder::from_cube_scaled()               │
│  └── visit_faces() traverses ALL exposed faces         │
│  └── Creates compound collider with N thin cuboids     │
│  └── N can be 10,000+ for complex terrain              │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│               Physics Step (60Hz)                       │
├─────────────────────────────────────────────────────────┤
│  Rapier broad-phase: test 100 objects vs BVH of N faces│
│  Rapier narrow-phase: exact collision for candidates   │
│  Bottleneck: BVH traversal and compound shape overhead │
└─────────────────────────────────────────────────────────┘
```

### Root Cause

Rapier's compound collider stores each face as a separate shape in a BVH. While BVH is O(log N) for queries, the constant factors for compound shapes are high:
- Each shape has its own isometry (position + rotation)
- BVH node traversal overhead per shape
- Memory cache misses for large compound shapes

## Strategy Comparison

| Strategy | Init Cost | Per-Frame Cost | Memory | Implementation Complexity |
|----------|-----------|----------------|--------|---------------------------|
| Monolithic | High (generate all faces) | Medium (BVH query) | High | Low (current) |
| Chunked | Medium (deferred) | Low-Medium (local BVH) | Medium | Medium |
| Hybrid Octree | Low | Low (octree query) | Low | High |

## Strategy 1: Monolithic Compound (Baseline)

Current approach, wrapped in trait interface for comparison.

```rust
pub struct MonolithicCollider {
    world_body: RigidBodyHandle,
    world_collider: ColliderHandle,
    face_count: usize,
}

impl WorldCollider for MonolithicCollider {
    fn init(&mut self, cube: &Rc<Cube<u8>>, world_size: f32, physics: &mut PhysicsWorld) {
        let collider = VoxelColliderBuilder::from_cube_scaled(cube, 0, world_size);
        self.face_count = count_compound_shapes(&collider);

        let body = RigidBodyBuilder::fixed().build();
        self.world_body = physics.add_rigid_body(body);
        self.world_collider = physics.add_collider(collider, self.world_body);
    }

    fn update(&mut self, _dynamic_aabbs: &[(RigidBodyHandle, Aabb)], _physics: &mut PhysicsWorld) {
        // No-op: collider is static
    }

    fn resolve_collision(&self, _body: RigidBodyHandle, _physics: &mut PhysicsWorld) -> Vec3 {
        Vec3::ZERO // Rapier handles collision resolution
    }
}
```

## Strategy 2: Chunked Colliders

Divide world into chunks, load colliders only near dynamic objects.

### Chunk System Design

```
World: 8192 x 8192 x 8192 units
Chunk size: 64 x 64 x 64 units (128 chunks per axis = 2M chunks max)

Load radius: 2 chunks (128 units)
Active chunks: ~27 per object (3³) worst case
With 100 objects, 50% overlap: ~1500 active chunks typical
```

### Data Structures

```rust
pub struct ChunkedCollider {
    cube: Rc<Cube<u8>>,
    world_size: f32,
    chunk_size: f32,              // e.g., 64.0
    load_radius: f32,             // e.g., 128.0

    // Spatial map: chunk position -> collider
    active_chunks: HashMap<IVec3, ChunkData>,

    // World body for all chunk colliders
    world_body: RigidBodyHandle,

    // Metrics
    chunks_loaded: usize,
    chunks_unloaded: usize,
    faces_generated: usize,
}

struct ChunkData {
    collider_handle: ColliderHandle,
    face_count: usize,
    last_active_frame: u64,
}
```

### Update Algorithm

```rust
fn update(&mut self, dynamic_aabbs: &[(RigidBodyHandle, Aabb)], physics: &mut PhysicsWorld) {
    // 1. Collect required chunks
    let mut required_chunks: HashSet<IVec3> = HashSet::new();
    for (_, aabb) in dynamic_aabbs {
        let expanded = aabb.expand(self.load_radius);
        for chunk_pos in self.chunks_in_aabb(&expanded) {
            required_chunks.insert(chunk_pos);
        }
    }

    // 2. Unload distant chunks
    let to_unload: Vec<IVec3> = self.active_chunks.keys()
        .filter(|pos| !required_chunks.contains(pos))
        .cloned()
        .collect();

    for pos in to_unload {
        if let Some(chunk) = self.active_chunks.remove(&pos) {
            physics.remove_collider(chunk.collider_handle);
            self.chunks_unloaded += 1;
        }
    }

    // 3. Load new chunks
    for pos in required_chunks {
        if !self.active_chunks.contains_key(&pos) {
            if let Some(chunk) = self.generate_chunk(pos, physics) {
                self.chunks_loaded += 1;
                self.faces_generated += chunk.face_count;
                self.active_chunks.insert(pos, chunk);
            }
        }
    }
}

fn generate_chunk(&self, pos: IVec3, physics: &mut PhysicsWorld) -> Option<ChunkData> {
    // Convert chunk position to world AABB
    let chunk_min = pos.as_vec3() * self.chunk_size - Vec3::splat(self.world_size / 2.0);
    let chunk_max = chunk_min + Vec3::splat(self.chunk_size);
    let chunk_aabb = Aabb::new(chunk_min, chunk_max);

    // Convert to octree local space [0,1]
    let local_aabb = self.world_to_local(&chunk_aabb);

    // Generate collider for just this region
    let collider = VoxelColliderBuilder::from_cube_region(
        &self.cube,
        0,
        Some(&local_aabb)
    );

    let face_count = count_compound_shapes(&collider);
    if face_count == 0 {
        return None; // Empty chunk
    }

    let handle = physics.add_collider(collider, self.world_body);

    Some(ChunkData {
        collider_handle: handle,
        face_count,
        last_active_frame: 0,
    })
}
```

### Chunk Position Calculation

```rust
fn world_to_chunk(&self, pos: Vec3) -> IVec3 {
    let half_world = self.world_size / 2.0;
    let normalized = (pos + Vec3::splat(half_world)) / self.chunk_size;
    normalized.floor().as_ivec3()
}

fn chunks_in_aabb(&self, aabb: &Aabb) -> impl Iterator<Item = IVec3> {
    let min_chunk = self.world_to_chunk(aabb.min);
    let max_chunk = self.world_to_chunk(aabb.max);

    (min_chunk.x..=max_chunk.x).flat_map(move |x| {
        (min_chunk.y..=max_chunk.y).flat_map(move |y| {
            (min_chunk.z..=max_chunk.z).map(move |z| IVec3::new(x, y, z))
        })
    })
}
```

## Strategy 3: Hybrid Octree Query

Bypass Rapier entirely for world collision, using direct octree queries.

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Physics Step                          │
├─────────────────────────────────────────────────────────┤
│  1. Rapier step() for dynamic↔dynamic only             │
│  2. For each dynamic body:                              │
│     a. Get body AABB                                    │
│     b. Query octree for faces in AABB                  │
│     c. Test box↔face collision                         │
│     d. Apply penetration correction                    │
└─────────────────────────────────────────────────────────┘
```

### Implementation

```rust
pub struct HybridOctreeCollider {
    cube: Rc<Cube<u8>>,
    world_size: f32,
    border_materials: [u8; 4],
}

impl WorldCollider for HybridOctreeCollider {
    fn init(&mut self, cube: &Rc<Cube<u8>>, world_size: f32, _physics: &mut PhysicsWorld) {
        self.cube = cube.clone();
        self.world_size = world_size;
        self.border_materials = [32, 32, 0, 0]; // From config
        // No Rapier colliders created!
    }

    fn update(&mut self, _dynamic_aabbs: &[(RigidBodyHandle, Aabb)], _physics: &mut PhysicsWorld) {
        // No collider management needed
    }

    fn resolve_collision(&self, body_handle: RigidBodyHandle, physics: &mut PhysicsWorld) -> Vec3 {
        let body = physics.get_rigid_body(body_handle)?;
        let position = Vec3::from_array(body.translation().into());

        // Get body's collider AABB (assume box for now)
        let half_extent = get_body_half_extent(body_handle, physics);
        let body_aabb = Aabb::new(
            position - half_extent,
            position + half_extent,
        );

        // Convert to octree local space
        let local_aabb = self.world_to_local(&body_aabb);

        // Query faces in region
        let mut total_correction = Vec3::ZERO;
        let region = RegionBounds::from_local_aabb(local_aabb.min, local_aabb.max, 3);

        if let Some(bounds) = region {
            visit_faces_in_region(&self.cube, &bounds, |face| {
                // Convert face to world space
                let face_world = self.local_to_world_face(face);

                // Test box↔face intersection
                if let Some(penetration) = box_face_penetration(&body_aabb, &face_world) {
                    total_correction += penetration.normal * penetration.depth;
                }
            }, self.border_materials);
        }

        total_correction
    }
}
```

### Box-Face Penetration Test

```rust
struct Penetration {
    normal: Vec3,
    depth: f32,
}

fn box_face_penetration(box_aabb: &Aabb, face: &WorldFace) -> Option<Penetration> {
    // Face is an axis-aligned quad at a specific position
    // Test if box overlaps the face's plane and is within face bounds

    let face_axis = face.normal.abs().max_element_index();
    let face_pos = face.center[face_axis];

    // Distance from box to face plane
    let box_min = box_aabb.min[face_axis];
    let box_max = box_aabb.max[face_axis];

    // Check if box straddles or penetrates the face plane
    let penetration_depth = if face.normal[face_axis] > 0.0 {
        box_max - face_pos
    } else {
        face_pos - box_min
    };

    if penetration_depth <= 0.0 {
        return None; // No penetration
    }

    // Check if box is within face's XZ/XY/YZ extent
    let (axis_a, axis_b) = other_axes(face_axis);
    if box_aabb.max[axis_a] < face.min[axis_a] || box_aabb.min[axis_a] > face.max[axis_a] {
        return None;
    }
    if box_aabb.max[axis_b] < face.min[axis_b] || box_aabb.min[axis_b] > face.max[axis_b] {
        return None;
    }

    Some(Penetration {
        normal: -face.normal, // Push out of solid
        depth: penetration_depth,
    })
}
```

## Benchmarking Framework

### Metrics Collection

```rust
pub struct ColliderMetrics {
    pub strategy_name: &'static str,
    pub init_time_ms: f32,
    pub update_time_us: f32,      // Per-frame average
    pub resolve_time_us: f32,     // Per-frame average
    pub active_colliders: usize,
    pub total_faces: usize,
    pub memory_bytes: usize,
}
```

### Config Integration

```toml
[physics]
gravity = -9.81
timestep = 0.016666

# World collision strategy: "monolithic", "chunked", "hybrid"
world_collision_strategy = "chunked"

[physics.chunked]
chunk_size = 64.0       # World units per chunk
load_radius = 128.0     # Distance to load chunks
unload_delay = 60       # Frames before unloading unused chunks
```

### Benchmark Test

```rust
#[test]
fn benchmark_collision_strategies() {
    let cube = generate_test_world(/* depth 10, typical terrain */);
    let mut bodies = spawn_test_bodies(100, /* random positions */);

    for strategy in ["monolithic", "chunked", "hybrid"] {
        let mut collider = create_strategy(strategy);

        let init_start = Instant::now();
        collider.init(&cube, 8192.0, &mut physics);
        let init_time = init_start.elapsed();

        let mut frame_times = Vec::new();
        for _ in 0..300 { // 5 seconds at 60fps
            let frame_start = Instant::now();

            let aabbs = collect_body_aabbs(&bodies, &physics);
            collider.update(&aabbs, &mut physics);
            physics.step(1.0/60.0);

            for (handle, _) in &aabbs {
                let correction = collider.resolve_collision(*handle, &mut physics);
                apply_correction(*handle, correction, &mut physics);
            }

            frame_times.push(frame_start.elapsed());
        }

        println!("{}: init={:?}, avg_frame={:?}, metrics={:?}",
            strategy, init_time, mean(&frame_times), collider.metrics());
    }
}
```

## Recommendation

Based on the analysis:

1. **Start with Chunked** - Best balance of performance and implementation effort
2. **Hybrid as stretch goal** - Highest potential performance but requires custom collision resolution
3. **Monolithic as baseline** - Keep for comparison and fallback

### Chunked Strategy Tuning

Optimal chunk size depends on:
- Object density: More objects → smaller chunks to reduce overlap
- Object size: Larger objects → larger chunks to reduce boundary crossings
- Terrain complexity: Complex terrain → smaller chunks for finer culling

Suggested starting values for proto-gl (100 objects, 10-unit size, 8192-unit world):
- Chunk size: 64 units (128 chunks per axis)
- Load radius: 128 units (2 chunk buffer)
- Expected active chunks: ~50-200 (depending on object clustering)
