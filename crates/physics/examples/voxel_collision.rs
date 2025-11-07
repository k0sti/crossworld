use crossworld_physics::{
    create_sphere_collider, glam::Vec3, rapier3d::prelude::*, CubeObject, PhysicsWorld,
    VoxelColliderBuilder,
};
use std::rc::Rc;

fn main() {
    println!("=== Voxel Collision Example ===\n");

    // Create physics world with gravity
    let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

    // Create a simple voxel cube (solid cube)
    let cube = Rc::new(crossworld_cube::Cube::Solid(1));
    println!("Created voxel cube (solid)");

    // Generate collision geometry from voxel cube
    let voxel_collider = VoxelColliderBuilder::from_cube(&cube, 3);
    println!("Generated collision geometry for voxel cube");

    // Create static rigid body for voxel terrain
    let mut voxel_body = CubeObject::new_static(&mut world, Vec3::new(0.0, 0.0, 0.0));
    voxel_body.attach_collider(&mut world, voxel_collider);
    println!("Added voxel terrain as static body\n");

    // Create falling sphere
    // The voxel cube octants span 0-0.25 in all dimensions
    // Position sphere at the center and just above the top face (y=0.25)
    let mut sphere = CubeObject::new_dynamic(&mut world, Vec3::new(0.125, 0.5, 0.125), 1.0);
    let sphere_collider = create_sphere_collider(0.05);
    sphere.attach_collider(&mut world, sphere_collider);
    println!("Created falling sphere at (0.125, 0.5, 0.125)\n");

    // Simulate
    let dt = 1.0 / 60.0;
    let total_time = 2.0;
    let steps = (total_time / dt) as usize;

    println!("Simulating for {} seconds...\n", total_time);

    for i in 0..steps {
        world.step(dt);

        if i % 30 == 0 {
            let time = i as f32 * dt;
            let pos = sphere.position(&world);
            let vel = sphere.velocity(&world);
            println!(
                "Time: {:.2}s | Position: ({:.3}, {:.3}, {:.3}) | Velocity: ({:.3}, {:.3}, {:.3})",
                time, pos.x, pos.y, pos.z, vel.x, vel.y, vel.z
            );
        }
    }

    let final_pos = sphere.position(&world);
    println!(
        "\nFinal sphere position: ({:.3}, {:.3}, {:.3})",
        final_pos.x, final_pos.y, final_pos.z
    );

    // Top of voxel cube is at y=0.25, sphere radius is 0.05, so expect y around 0.30-0.35
    if final_pos.y > 0.28 && final_pos.y < 0.38 {
        println!("✓ Sphere landed on voxel terrain!");
    } else {
        println!("✗ Sphere position unexpected (expected y around 0.30-0.35)");
    }
}
