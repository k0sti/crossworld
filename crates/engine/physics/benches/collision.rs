//! Benchmark for world collision
//!
//! Benchmarks initialization time and per-frame resolution time for the
//! world collision system using direct octree queries.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crossworld_physics::collision::Aabb;
use crossworld_physics::world_collider::WorldCollider;
use crossworld_physics::PhysicsWorld;
use cube::{parse_csm, Cube};
use glam::Vec3;
use rapier3d::prelude::*;
use std::rc::Rc;

/// Create a test world cube with procedural terrain
fn create_test_world(_depth: u32) -> Rc<Cube<u8>> {
    // Create a simple layered terrain:
    // Bottom half solid (material 32), top half air (material 0)
    // This creates a ground plane with some complexity
    let csm = ">n [
        [32 32 32 32 0 0 0 0]
    ]";

    Rc::new(parse_csm(csm).expect("Failed to parse test cube"))
}

/// Benchmark configuration
struct BenchConfig {
    world_size: f32,
    border_materials: [u8; 4],
    dynamic_count: usize,
    spawn_radius: f32,
    spawn_height: f32,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            world_size: 1024.0,
            border_materials: [32, 32, 0, 0],
            dynamic_count: 100,
            spawn_radius: 100.0,
            spawn_height: 50.0,
        }
    }
}

/// Spawn dynamic bodies at random positions
fn spawn_dynamic_bodies(
    physics: &mut PhysicsWorld,
    count: usize,
    radius: f32,
    height: f32,
) -> Vec<RigidBodyHandle> {
    let mut handles = Vec::with_capacity(count);

    for i in 0..count {
        // Distribute in a grid pattern
        let cols = (count as f32).sqrt().ceil() as usize;
        let row = i / cols;
        let col = i % cols;

        let x = (col as f32 - cols as f32 / 2.0) * (radius * 2.0 / cols as f32);
        let z = (row as f32 - cols as f32 / 2.0) * (radius * 2.0 / cols as f32);
        let y = height;

        let body = RigidBodyBuilder::dynamic()
            .translation(vector![x, y, z])
            .build();
        let handle = physics.add_rigid_body(body);

        // Add box collider
        let collider = ColliderBuilder::cuboid(0.5, 0.5, 0.5).density(1.0).build();
        physics.add_collider(collider, handle);

        handles.push(handle);
    }

    handles
}

/// Collect AABBs for all dynamic bodies
fn collect_aabbs(
    physics: &PhysicsWorld,
    handles: &[RigidBodyHandle],
) -> Vec<(RigidBodyHandle, Aabb)> {
    handles
        .iter()
        .filter_map(|&handle| {
            physics.get_rigid_body(handle).map(|body| {
                let pos = body.translation();
                let half = 0.5; // Half-size of box collider
                (
                    handle,
                    Aabb::new(
                        Vec3::new(pos.x - half, pos.y - half, pos.z - half),
                        Vec3::new(pos.x + half, pos.y + half, pos.z + half),
                    ),
                )
            })
        })
        .collect()
}

fn bench_world_collider_init(c: &mut Criterion) {
    let cube = create_test_world(3);
    let config = BenchConfig::default();

    c.bench_function("world_collider_init", |b| {
        b.iter(|| {
            let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
            let mut collider = WorldCollider::new();
            collider.init(
                &cube,
                config.world_size,
                config.border_materials,
                &mut physics,
            );
            black_box(collider.metrics())
        });
    });
}

fn bench_world_collider_frame(c: &mut Criterion) {
    let cube = create_test_world(3);
    let config = BenchConfig::default();

    // Setup
    let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
    let mut collider = WorldCollider::new();
    collider.init(
        &cube,
        config.world_size,
        config.border_materials,
        &mut physics,
    );
    let handles = spawn_dynamic_bodies(
        &mut physics,
        config.dynamic_count,
        config.spawn_radius,
        config.spawn_height,
    );

    c.bench_function("world_collider_frame", |b| {
        b.iter(|| {
            physics.step(1.0 / 60.0);
            // Resolve world collision after physics step
            let aabbs = collect_aabbs(&physics, &handles);
            for (handle, aabb) in &aabbs {
                let correction = collider.resolve_collision(*handle, aabb);
                black_box(correction);
            }
            black_box(())
        });
    });
}

fn bench_world_collider_creation(c: &mut Criterion) {
    c.bench_function("world_collider_new", |b| {
        b.iter(|| black_box(WorldCollider::new()));
    });
}

criterion_group!(
    benches,
    bench_world_collider_init,
    bench_world_collider_frame,
    bench_world_collider_creation,
);

criterion_main!(benches);
