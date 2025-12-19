// Proto-GL Physics Viewer Library
// Modular structure for the physics testing viewer

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
pub use config::{ProtoGlConfig, WorldConfig, PhysicsConfig, SpawningConfig, RenderConfig};
pub use models::{VoxModel, CubeObject};

use std::error::Error;
use std::rc::Rc;
use crossworld_physics::{rapier3d::prelude::*, PhysicsWorld, VoxelColliderBuilder};
use config::load_config;
use models::load_vox_models;
use physics::spawn_cube_objects;
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
    println!("  Spawn height: {:.1} - {:.1}", config.spawning.min_height, config.spawning.max_height);
    println!();

    // Generate world
    let (world_cube, world_depth) = generate_world(&config.world);
    let world_cube_rc = Rc::new(world_cube);

    let world_size = config.world.world_size();
    let half_world = config.world.half_world();
    println!("World generated:");
    println!("  Depth: {}", world_depth);
    println!("  World size: {:.0} (2^{} units)", world_size, config.world.macro_depth + config.world.border_depth);
    println!("  World bounds: [{:.0}, {:.0}] centered at origin", -half_world, half_world);
    println!("  Ground level: y = {:.0}", -half_world);
    println!();

    // Initialize physics world
    let gravity = glam::Vec3::new(0.0, config.physics.gravity, 0.0);
    let mut physics_world = PhysicsWorld::new(gravity);

    // Create world collider (static terrain) scaled to world coordinates
    let world_collider = VoxelColliderBuilder::from_cube_scaled(&world_cube_rc, world_depth, world_size);

    // Debug: check world collider
    if let Some(compound) = world_collider.shape().as_compound() {
        println!("World collider: {} shapes in compound", compound.shapes().len());
        // Sample some shapes
        for (i, (iso, _shape)) in compound.shapes().iter().enumerate().take(5) {
            let pos = iso.translation;
            println!("  Shape {}: at ({:.3}, {:.3}, {:.3})", i, pos.x, pos.y, pos.z);
        }
        if compound.shapes().len() > 5 {
            println!("  ... and {} more shapes", compound.shapes().len() - 5);
        }
    } else if world_collider.shape().as_ball().is_some() {
        println!("World collider: ball (empty/minimal)");
    } else {
        println!("World collider: unknown shape type");
    }

    let world_body = RigidBodyBuilder::fixed().build();
    let world_body_handle = physics_world.add_rigid_body(world_body);
    physics_world.add_collider(world_collider, world_body_handle);

    // Add explicit ground plane at ground level (y = -half_world)
    let ground_y = config.world.ground_y();
    let ground_body = RigidBodyBuilder::fixed()
        .translation(vector![0.0, ground_y, 0.0])
        .build();
    let ground_handle = physics_world.add_rigid_body(ground_body);
    let ground_collider = ColliderBuilder::cuboid(half_world, 0.1, half_world)  // Large flat ground
        .friction(0.5)
        .restitution(0.0)
        .build();
    physics_world.add_collider(ground_collider, ground_handle);
    println!("Added ground plane at Y={:.0}", ground_y);

    // Load models and spawn dynamic cubes
    let models = load_vox_models(&config.spawning.models_path);
    println!("Loaded {} models", models.len());

    let objects = spawn_cube_objects(&config.spawning, &models, &mut physics_world);
    println!("Spawned {} objects\n", objects.len());

    // Debug: check object colliders
    for (i, obj) in objects.iter().enumerate() {
        if let Some(body) = physics_world.get_rigid_body(obj.body_handle) {
            let pos = body.translation();
            println!("Object {}: {} at ({:.2}, {:.2}, {:.2})", i, obj.model_name, pos.x, pos.y, pos.z);
        }
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
        // Step physics
        physics_world.step(timestep);

        // Log at intervals
        if iter % log_interval == 0 || iter == iterations - 1 {
            println!("--- Iteration {} (t = {:.3}s) ---", iter, iter as f32 * timestep);

            for (i, obj) in objects.iter().enumerate() {
                if let Some(body) = physics_world.get_rigid_body(obj.body_handle) {
                    let pos = body.translation();
                    let vel = body.linvel();
                    let is_sleeping = body.is_sleeping();

                    println!(
                        "  [{}] {} pos=({:.2}, {:.2}, {:.2}) vel=({:.2}, {:.2}, {:.2}) {}",
                        i,
                        obj.model_name,
                        pos.x, pos.y, pos.z,
                        vel.x, vel.y, vel.z,
                        if is_sleeping { "[SLEEPING]" } else { "" }
                    );

                    // Check if fallen through world (below ground level)
                    if pos.y < ground_y - 10.0 {
                        println!("    ⚠️  WARNING: Object fell through world!");
                    }
                }
            }
            println!();
        }
    }

    println!("=== Simulation complete ===\n");

    // Final summary
    println!("Final positions (ground at Y={:.0}):", ground_y);
    let mut fell_through = 0;
    for (i, obj) in objects.iter().enumerate() {
        if let Some(body) = physics_world.get_rigid_body(obj.body_handle) {
            let pos = body.translation();
            let status = if pos.y < ground_y - 10.0 {
                fell_through += 1;
                "❌ FELL THROUGH"
            } else if pos.y < ground_y {
                "⚠️  Below ground"
            } else {
                "✓ OK"
            };
            println!("  [{}] {} Y={:.2} {}", i, obj.model_name, pos.y, status);
        }
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
