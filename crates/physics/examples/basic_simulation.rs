use crossworld_physics::{
    create_box_collider, glam::Vec3, rapier3d::prelude::*, PhysicsWorld, CubeObject,
};

fn main() {
    println!("=== Basic Physics Simulation ===\n");

    // Create physics world with gravity
    let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
    println!("Created physics world with gravity: {:?}", world.gravity());

    // Create ground (static box)
    let mut ground = CubeObject::new_static(&mut world, Vec3::new(0.0, -0.5, 0.0));
    let ground_collider = create_box_collider(Vec3::new(10.0, 0.5, 10.0));
    ground.attach_collider(&mut world, ground_collider);
    println!("Created ground at y = -0.5");

    // Create falling box (dynamic)
    let mut falling_box = CubeObject::new_dynamic(&mut world, Vec3::new(0.0, 10.0, 0.0), 1.0);
    let box_collider = create_box_collider(Vec3::new(0.5, 0.5, 0.5));
    falling_box.attach_collider(&mut world, box_collider);
    println!("Created falling box at y = 10.0\n");

    // Simulate for 3 seconds
    let dt = 1.0 / 60.0; // 60 FPS
    let total_time = 3.0;
    let steps = (total_time / dt) as usize;

    println!("Simulating for {} seconds ({} steps)...\n", total_time, steps);

    for i in 0..steps {
        world.step(dt);

        // Print position every 30 frames (0.5 seconds)
        if i % 30 == 0 {
            let time = i as f32 * dt;
            let pos = falling_box.position(&world);
            let vel = falling_box.velocity(&world);
            println!(
                "Time: {:.2}s | Position: ({:.3}, {:.3}, {:.3}) | Velocity: ({:.3}, {:.3}, {:.3})",
                time, pos.x, pos.y, pos.z, vel.x, vel.y, vel.z
            );
        }
    }

    let final_pos = falling_box.position(&world);
    println!("\nFinal position: ({:.3}, {:.3}, {:.3})", final_pos.x, final_pos.y, final_pos.z);

    if final_pos.y > 0.0 && final_pos.y < 1.0 {
        println!("✓ Box landed on ground successfully!");
    } else {
        println!("✗ Box position unexpected");
    }
}
