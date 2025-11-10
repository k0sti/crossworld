use crossworld_physics::{
    create_box_collider, CharacterController, CharacterControllerConfig, CubeObject, PhysicsWorld,
};
use glam::Vec3;

fn main() {
    println!("Character Movement Example");
    println!("==========================\n");

    // Create physics world with gravity
    let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));

    // Create a ground plane
    println!("Creating ground plane...");
    let mut ground = CubeObject::new_static(&mut world, Vec3::new(0.0, -0.5, 0.0));
    let ground_collider = create_box_collider(Vec3::new(50.0, 0.5, 50.0));
    ground.attach_collider(&mut world, ground_collider);

    // Create character controller
    println!("Creating character controller...");
    let config = CharacterControllerConfig {
        height: 1.8,
        radius: 0.3,
        step_height: 0.5,
        max_slope_angle: 45.0,
        gravity: 9.8,
        jump_impulse: 5.0,
        ground_check_distance: 0.1,
    };

    let mut character = CharacterController::new(&mut world, Vec3::new(0.0, 5.0, 0.0), config);

    println!("Initial position: {}", character.position(&world));
    println!("Initial grounded: {}\n", character.is_grounded());

    // Simulation parameters
    let dt = 1.0 / 60.0; // 60 FPS
    let total_time = 5.0; // 5 seconds
    let steps = (total_time / dt) as usize;

    println!("Simulating {} seconds of physics...\n", total_time);

    // Phase 1: Fall to ground (first 2 seconds)
    println!("Phase 1: Falling to ground");
    for i in 0..120 {
        character.move_with_velocity(&mut world, Vec3::ZERO, dt);
        world.step(dt);

        if i % 30 == 0 {
            let pos = character.position(&world);
            println!(
                "  t={:.2}s: pos=({:.2}, {:.2}, {:.2}), grounded={}, vy={:.2}",
                i as f32 * dt,
                pos.x,
                pos.y,
                pos.z,
                character.is_grounded(),
                character.vertical_velocity()
            );
        }
    }

    println!("\nPhase 2: Walking forward");
    // Phase 2: Walk forward (next 1 second)
    for i in 0..60 {
        let velocity = Vec3::new(3.0, 0.0, 0.0); // Walk speed
        character.move_with_velocity(&mut world, velocity, dt);
        world.step(dt);

        if i % 20 == 0 {
            let pos = character.position(&world);
            println!(
                "  t={:.2}s: pos=({:.2}, {:.2}, {:.2}), grounded={}",
                (120 + i) as f32 * dt,
                pos.x,
                pos.y,
                pos.z,
                character.is_grounded()
            );
        }
    }

    println!("\nPhase 3: Jump and continue walking");
    // Phase 3: Jump (next 2 seconds)
    character.jump();
    println!(
        "  Jumped! Initial vertical velocity: {:.2}",
        character.vertical_velocity()
    );

    for i in 0..120 {
        let velocity = Vec3::new(3.0, 0.0, 0.0); // Continue walking
        character.move_with_velocity(&mut world, velocity, dt);
        world.step(dt);

        if i % 20 == 0 {
            let pos = character.position(&world);
            println!(
                "  t={:.2}s: pos=({:.2}, {:.2}, {:.2}), grounded={}, vy={:.2}",
                (180 + i) as f32 * dt,
                pos.x,
                pos.y,
                pos.z,
                character.is_grounded(),
                character.vertical_velocity()
            );
        }
    }

    // Final state
    let final_pos = character.position(&world);
    println!("\n=== Final State ===");
    println!(
        "Position: ({:.2}, {:.2}, {:.2})",
        final_pos.x, final_pos.y, final_pos.z
    );
    println!("Grounded: {}", character.is_grounded());
    println!("Vertical velocity: {:.2}", character.vertical_velocity());

    // Verify expectations
    println!("\n=== Verification ===");

    // For now, just check that simulation ran
    println!(
        "Character moved from (0, 5, 0) to ({:.2}, {:.2}, {:.2})",
        final_pos.x, final_pos.y, final_pos.z
    );
    println!("✓ Simulation completed successfully!");

    // Cleanup
    character.destroy(&mut world);
    println!("\nCharacter destroyed. Example complete!");
}
