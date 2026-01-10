// Proto-GL Physics Viewer Library
// Modular structure for the physics testing viewer

#![allow(clippy::arc_with_non_send_sync)]
#![allow(clippy::collapsible_if)]

pub mod app;
pub mod camera;
pub mod config;
pub mod models;
pub mod physics;
pub mod structures;
pub mod ui;
pub mod world;

// Re-export commonly used types
pub use app::ProtoGlApp;
pub use camera::OrbitCamera;
pub use config::{PhysicsConfig, ProtoGlConfig, RenderConfig, SpawningConfig, WorldConfig};
pub use models::{SpawnedObject, VoxModel};

use config::load_config;
use crossworld_physics::{
    PhysicsWorld,
    collision::Aabb,
    rapier3d::parry::bounding_volume::Aabb as RapierAabb,
    terrain::{ActiveRegionTracker, VoxelTerrainCollider},
};
use models::load_vox_models;
use physics::spawn_cube_objects;
use std::error::Error;
use std::sync::Arc;
use world::generate_world;

/// Run physics simulation in debug mode without graphics
/// Logs physics data for each iteration and exits
pub fn run_physics_debug(iterations: u32) -> Result<(), Box<dyn Error>> {
    // Load config
    let config = load_config().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load config.toml: {}", e);
        eprintln!("Using default configuration");
        ProtoGlConfig::default()
    });

    println!("Configuration:");
    println!("  Gravity: {:.2}", config.physics.gravity);
    println!("  Timestep: {:.4}s", config.physics.timestep);
    println!("  Spawn count: {}", config.spawning.spawn_count);
    println!(
        "  Spawn height: {:.1} - {:.1}",
        config.spawning.min_height, config.spawning.max_height
    );
    println!();

    // Generate world
    let (world_cube, world_depth) = generate_world(&config.world);
    let world_cube_arc = Arc::new(world_cube);

    let world_size = config.world.world_size();
    let half_world = config.world.half_world();
    println!("World generated:");
    println!("  Depth: {}", world_depth);
    println!(
        "  World size: {:.0} (2^{} units)",
        world_size,
        config.world.macro_depth + config.world.border_depth
    );
    println!(
        "  World bounds: [{:.0}, {:.0}] centered at origin",
        -half_world, half_world
    );
    println!("  Ground surface: Y=0 (border midpoint)");
    println!();

    // Initialize physics world
    let gravity = glam::Vec3::new(0.0, config.physics.gravity, 0.0);
    let mut physics_world = PhysicsWorld::new(gravity);

    // Create terrain collider using TypedCompositeShape
    let mut terrain_collider = VoxelTerrainCollider::new(
        world_cube_arc,
        world_size,
        3, // region_depth
        config.world.border_materials,
    );
    let mut active_region_tracker = ActiveRegionTracker::new(10.0);

    println!(
        "Terrain collider ready (region_depth: {})",
        terrain_collider.region_depth()
    );

    // Load models and spawn dynamic cubes
    let models = load_vox_models(&config.spawning.models_csv, &config.spawning.models_path);
    println!("Loaded {} models", models.len());

    let objects = spawn_cube_objects(&config.spawning, &models, &mut physics_world);
    println!("Spawned {} objects\n", objects.len());

    // Debug: check object colliders
    for (i, obj) in objects.iter().enumerate() {
        let pos = obj.physics.position(&physics_world);
        println!(
            "Object {}: {} at ({:.2}, {:.2}, {:.2})",
            i, obj.model_name, pos.x, pos.y, pos.z
        );
    }

    // Count total colliders
    let collider_count = physics_world.collider_count();
    let body_count = physics_world.rigid_body_count();
    println!("\nPhysics world:");
    println!("  Rigid bodies: {}", body_count);
    println!("  Colliders: {}", collider_count);
    println!();

    println!("=== Starting simulation ===\n");

    // Run physics simulation
    let timestep = config.physics.timestep;
    let log_interval = (iterations / 10).max(1); // Log 10 times during simulation

    for iter in 0..iterations {
        // Update terrain collider BVH based on dynamic object positions
        let rapier_aabbs: Vec<RapierAabb> = objects
            .iter()
            .map(|obj| {
                let aabb = obj.physics.world_aabb(&physics_world);
                RapierAabb::new(
                    [aabb.min.x, aabb.min.y, aabb.min.z].into(),
                    [aabb.max.x, aabb.max.y, aabb.max.z].into(),
                )
            })
            .collect();

        if let Some(active_aabb) = active_region_tracker.update(&rapier_aabbs) {
            terrain_collider.update_triangle_bvh(&active_aabb);
        }

        // Step physics
        physics_world.step(timestep);

        // Resolve terrain collisions
        for obj in objects.iter() {
            let body_aabb = obj.physics.world_aabb(&physics_world);
            let rapier_aabb = RapierAabb::new(
                [body_aabb.min.x, body_aabb.min.y, body_aabb.min.z].into(),
                [body_aabb.max.x, body_aabb.max.y, body_aabb.max.z].into(),
            );

            let correction = resolve_terrain_collision(&terrain_collider, &rapier_aabb, &body_aabb);
            obj.physics
                .apply_collision_response(&mut physics_world, correction);
        }

        // Log at intervals
        if iter % log_interval == 0 || iter == iterations - 1 {
            println!(
                "--- Iteration {} (t = {:.3}s) ---",
                iter,
                iter as f32 * timestep
            );

            for (i, obj) in objects.iter().enumerate() {
                let pos = obj.physics.position(&physics_world);
                let vel = obj.physics.velocity(&physics_world);
                // Check sleeping state via the body handle
                let is_sleeping = physics_world
                    .get_rigid_body(obj.physics.body_handle())
                    .map(|b| b.is_sleeping())
                    .unwrap_or(false);

                println!(
                    "  [{}] {} pos=({:.2}, {:.2}, {:.2}) vel=({:.2}, {:.2}, {:.2}) {}",
                    i,
                    obj.model_name,
                    pos.x,
                    pos.y,
                    pos.z,
                    vel.x,
                    vel.y,
                    vel.z,
                    if is_sleeping { "[SLEEPING]" } else { "" }
                );

                // Check if fallen through world (below ground level at Y=0)
                if pos.y < -10.0 {
                    println!("    ⚠️  WARNING: Object fell through world!");
                }
            }
            println!();
        }
    }

    println!("=== Simulation complete ===\n");

    // Final summary
    println!("Final positions (ground at Y=0):");
    let mut fell_through = 0;
    for (i, obj) in objects.iter().enumerate() {
        let pos = obj.physics.position(&physics_world);
        let status = if pos.y < -10.0 {
            fell_through += 1;
            "❌ FELL THROUGH"
        } else if pos.y < 0.0 {
            "⚠️  Below ground"
        } else {
            "✓ OK"
        };
        println!("  [{}] {} Y={:.2} {}", i, obj.model_name, pos.y, status);
    }

    println!();
    if fell_through > 0 {
        println!("❌ {} objects fell through the world!", fell_through);
        println!("\nPossible causes:");
        println!("  1. World collider not generated correctly (empty or wrong shape)");
        println!("  2. Object colliders not generated correctly");
        println!("  3. Objects spawned outside world bounds");
        println!("  4. Physics timestep too large");
    } else {
        println!("✓ All objects stayed within bounds");
    }

    Ok(())
}

/// Resolve terrain collision for an object AABB using the terrain collider's BVH
fn resolve_terrain_collision(
    terrain_collider: &VoxelTerrainCollider,
    rapier_aabb: &RapierAabb,
    body_aabb: &Aabb,
) -> glam::Vec3 {
    let bvh = terrain_collider.triangle_bvh();
    let mut max_correction = glam::Vec3::ZERO;

    for leaf_idx in bvh.intersect_aabb(rapier_aabb) {
        if let Some(triangle) = terrain_collider.get_triangle_by_index(leaf_idx) {
            let correction = compute_aabb_triangle_correction(body_aabb, &triangle);
            if correction.length_squared() > max_correction.length_squared() {
                max_correction = correction;
            }
        }
    }

    max_correction
}

/// Compute penetration correction for AABB-triangle intersection
fn compute_aabb_triangle_correction(
    aabb: &Aabb,
    triangle: &crossworld_physics::rapier3d::parry::shape::Triangle,
) -> glam::Vec3 {
    use glam::Vec3;

    let a = Vec3::new(triangle.a.x, triangle.a.y, triangle.a.z);
    let b = Vec3::new(triangle.b.x, triangle.b.y, triangle.b.z);
    let c = Vec3::new(triangle.c.x, triangle.c.y, triangle.c.z);

    let edge1 = b - a;
    let edge2 = c - a;
    let normal = edge1.cross(edge2);
    let normal_len = normal.length();
    if normal_len < 1e-6 {
        return Vec3::ZERO;
    }
    let normal = normal / normal_len;

    let tri_min = a.min(b).min(c);
    let tri_max = a.max(b).max(c);

    if aabb.max.x < tri_min.x
        || aabb.min.x > tri_max.x
        || aabb.max.y < tri_min.y
        || aabb.min.y > tri_max.y
        || aabb.max.z < tri_min.z
        || aabb.min.z > tri_max.z
    {
        return Vec3::ZERO;
    }

    let aabb_corner = Vec3::new(
        if normal.x > 0.0 {
            aabb.min.x
        } else {
            aabb.max.x
        },
        if normal.y > 0.0 {
            aabb.min.y
        } else {
            aabb.max.y
        },
        if normal.z > 0.0 {
            aabb.min.z
        } else {
            aabb.max.z
        },
    );

    let d = normal.dot(a);
    let dist = normal.dot(aabb_corner) - d;

    if dist < 0.0 {
        let center = (aabb.min + aabb.max) * 0.5;
        let proj = center - normal * (normal.dot(center) - d);

        if point_in_triangle_2d(&proj, &a, &b, &c, &normal) {
            return normal * (-dist);
        }
    }

    Vec3::ZERO
}

fn point_in_triangle_2d(
    p: &glam::Vec3,
    a: &glam::Vec3,
    b: &glam::Vec3,
    c: &glam::Vec3,
    normal: &glam::Vec3,
) -> bool {
    let abs_normal = normal.abs();
    let (u_idx, v_idx) = if abs_normal.x >= abs_normal.y && abs_normal.x >= abs_normal.z {
        (1, 2)
    } else if abs_normal.y >= abs_normal.z {
        (0, 2)
    } else {
        (0, 1)
    };

    let get_uv = |v: &glam::Vec3| -> (f32, f32) {
        let arr = [v.x, v.y, v.z];
        (arr[u_idx], arr[v_idx])
    };

    let (pu, pv) = get_uv(p);
    let (au, av) = get_uv(a);
    let (bu, bv) = get_uv(b);
    let (cu, cv) = get_uv(c);

    let sign = |p1: (f32, f32), p2: (f32, f32), p3: (f32, f32)| -> f32 {
        (p1.0 - p3.0) * (p2.1 - p3.1) - (p2.0 - p3.0) * (p1.1 - p3.1)
    };

    let d1 = sign((pu, pv), (au, av), (bu, bv));
    let d2 = sign((pu, pv), (bu, bv), (cu, cv));
    let d3 = sign((pu, pv), (cu, cv), (au, av));

    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;

    !(has_neg && has_pos)
}
