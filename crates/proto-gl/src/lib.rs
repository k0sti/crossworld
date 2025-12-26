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
pub use config::{PhysicsConfig, ProtoGlConfig, RenderConfig, SpawningConfig, WorldConfig};
pub use models::{SpawnedObject, VoxModel};

use config::load_config;
use crossworld_physics::{PhysicsWorld, world_collider::WorldCollider};
use models::load_vox_models;
use physics::spawn_cube_objects;
use std::error::Error;
use std::rc::Rc;
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
    let world_cube_rc = Rc::new(world_cube);

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

    // Create world collider
    let mut world_collider = WorldCollider::new();
    world_collider.init(
        &world_cube_rc,
        world_size,
        config.world.border_materials,
        &mut physics_world,
    );

    // Debug: check world collider
    println!(
        "World collider ready (strategy: {})",
        world_collider.metrics().strategy_name
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
        // Step physics
        physics_world.step(timestep);

        for obj in objects.iter() {
            // Use physics crate's world_aabb for accurate AABB calculation
            let body_aabb = obj.physics.world_aabb(&physics_world);

            // Get correction from world collider and apply collision response
            let correction = world_collider.resolve_collision(obj.physics.body_handle(), &body_aabb);
            obj.physics.apply_collision_response(&mut physics_world, correction);
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
