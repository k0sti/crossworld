# Capability: world-collision-strategies

Provides configurable world collision strategies for voxel terrain with performance benchmarking.

## ADDED Requirements

### Requirement: WorldCollider trait interface

The system SHALL provide a `WorldCollider` trait that abstracts world collision strategy implementation.

#### Scenario: Initialize world collider
- **GIVEN** a world cube with depth 13 (8192 units)
- **AND** a physics world instance
- **WHEN** calling `init()` on a WorldCollider
- **THEN** the collider SHALL be ready to handle collision queries
- **AND** `metrics().strategy_name` SHALL return the strategy identifier

#### Scenario: Update collider based on dynamic objects
- **GIVEN** an initialized WorldCollider
- **AND** a list of dynamic body AABBs
- **WHEN** calling `update()`
- **THEN** the collider SHALL adjust its internal state based on object positions
- **AND** the operation SHALL complete in under 1ms for 100 objects

#### Scenario: Query collision metrics
- **GIVEN** an initialized WorldCollider
- **WHEN** calling `metrics()`
- **THEN** result SHALL include `init_time_ms`, `update_time_us`, `active_colliders`, `total_faces`

### Requirement: Monolithic strategy preserves current behavior

The system SHALL provide a `MonolithicCollider` that generates a single compound collider for the entire world.

#### Scenario: Generate world collider at startup
- **GIVEN** a world cube with terrain
- **WHEN** initializing MonolithicCollider
- **THEN** a single compound collider SHALL be created with all exposed faces
- **AND** the collider SHALL be attached to a fixed rigid body

#### Scenario: Update is no-op for monolithic
- **GIVEN** an initialized MonolithicCollider
- **WHEN** calling `update()` with any dynamic AABBs
- **THEN** no changes SHALL occur to the physics world
- **AND** update time SHALL be under 1 microsecond

### Requirement: Chunked strategy loads colliders on demand

The system SHALL provide a `ChunkedCollider` that loads/unloads chunk colliders based on object proximity.

#### Scenario: Load chunk when object approaches
- **GIVEN** a ChunkedCollider with chunk_size=64 and load_radius=128
- **AND** a dynamic object at position (0, 50, 0)
- **WHEN** calling `update()`
- **THEN** chunks within 128 units of (0, 50, 0) SHALL have colliders created
- **AND** chunks outside this radius SHALL NOT have colliders

#### Scenario: Unload chunk when objects leave
- **GIVEN** a ChunkedCollider with loaded chunks around (0, 50, 0)
- **AND** no dynamic objects within load_radius of those chunks
- **WHEN** calling `update()`
- **THEN** those chunk colliders SHALL be removed from the physics world

#### Scenario: Chunk collider uses region-bounded face traversal
- **GIVEN** a chunk at position (0, 0, 0) with chunk_size=64
- **WHEN** generating the chunk collider
- **THEN** only faces within the chunk's AABB SHALL be included
- **AND** `VoxelColliderBuilder::from_cube_with_region_scaled()` SHALL be used

### Requirement: Hybrid strategy uses direct octree queries

The system SHALL provide a `HybridOctreeCollider` that bypasses Rapier for world collision.

#### Scenario: No Rapier colliders for world
- **GIVEN** a HybridOctreeCollider
- **WHEN** `init()` completes
- **THEN** no world colliders SHALL be added to the physics world
- **AND** `metrics().active_colliders` SHALL be 0

#### Scenario: Resolve collision via octree query
- **GIVEN** a dynamic body with AABB intersecting solid terrain
- **WHEN** calling `resolve_collision()`
- **THEN** faces within the body's AABB SHALL be queried from the octree
- **AND** a penetration correction vector SHALL be returned

#### Scenario: Octree query uses visit_faces_in_region
- **GIVEN** a body AABB converted to RegionBounds
- **WHEN** querying for collision faces
- **THEN** `visit_faces_in_region()` SHALL be called with those bounds
- **AND** only faces within the region SHALL be processed

### Requirement: Configuration selects collision strategy

The system SHALL read collision strategy from configuration.

#### Scenario: Select strategy via config
- **GIVEN** config.toml with `physics.world_collision_strategy = "chunked"`
- **WHEN** initializing physics
- **THEN** ChunkedCollider SHALL be used

#### Scenario: Default to monolithic if unspecified
- **GIVEN** config.toml without `physics.world_collision_strategy`
- **WHEN** initializing physics
- **THEN** MonolithicCollider SHALL be used (backward compatible)

#### Scenario: Configure chunked strategy parameters
- **GIVEN** config.toml with:
  ```toml
  [physics.chunked]
  chunk_size = 32.0
  load_radius = 64.0
  ```
- **WHEN** initializing ChunkedCollider
- **THEN** chunk_size SHALL be 32.0
- **AND** load_radius SHALL be 64.0

### Requirement: Benchmark harness compares strategies

The system SHALL provide benchmarking utilities to compare collision strategy performance.

#### Scenario: Run benchmark with all strategies
- **GIVEN** a test world and 100 dynamic objects
- **WHEN** running the collision benchmark
- **THEN** each strategy SHALL be tested for 300 frames (5 seconds)
- **AND** results SHALL include init_time, avg_frame_time, and metrics for each

#### Scenario: Benchmark outputs comparable metrics
- **GIVEN** benchmark results for all strategies
- **THEN** results SHALL be in a format suitable for tabular comparison
- **AND** results SHALL include percentage improvement over baseline

### Requirement: Chunked strategy performance target

The ChunkedCollider SHALL achieve significant performance improvement over MonolithicCollider.

#### Scenario: Measure frame time improvement
- **GIVEN** 100 dynamic objects in a clustered region
- **WHEN** comparing ChunkedCollider to MonolithicCollider
- **THEN** average physics step time SHALL be at least 3x faster
- **OR** if not achieved, benchmark SHALL log detailed metrics for analysis

#### Scenario: Measure memory reduction
- **GIVEN** ChunkedCollider with typical object distribution
- **THEN** active_colliders × avg_faces_per_chunk SHALL be less than monolithic total_faces
- **AND** reduction SHALL be at least 50%
