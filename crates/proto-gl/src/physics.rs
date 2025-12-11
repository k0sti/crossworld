use rand::Rng;
use crossworld_physics::{rapier3d::prelude::*, PhysicsWorld, VoxelColliderBuilder};
use crate::config::SpawningConfig;
use crate::models::{VoxModel, CubeObject};

/// Spawn dynamic cube objects with physics
pub fn spawn_cube_objects(
    config: &SpawningConfig,
    models: &[VoxModel],
    physics_world: &mut PhysicsWorld,
) -> Vec<CubeObject> {
    let mut objects = Vec::new();
    let mut rng = rand::thread_rng();

    for i in 0..config.spawn_count {
        // Random position
        let x = rng.gen_range(-config.spawn_radius..config.spawn_radius);
        let y = rng.gen_range(config.min_height..config.max_height);
        let z = rng.gen_range(-config.spawn_radius..config.spawn_radius);

        // Random model
        let model = &models[i as usize % models.len()];

        // Create physics body
        let rb = RigidBodyBuilder::dynamic()
            .translation(vector![x, y, z])
            .build();
        let rb_handle = physics_world.add_rigid_body(rb);

        // Create collider from voxel cube
        let collider = VoxelColliderBuilder::from_cube(&model.cube, model.depth);
        let coll_handle = physics_world.add_collider(collider, rb_handle);

        objects.push(CubeObject {
            cube: model.cube.clone(),
            body_handle: rb_handle,
            collider_handle: coll_handle,
            model_name: model.name.clone(),
            depth: model.depth,
        });
    }

    println!("Spawned {} dynamic cubes", objects.len());
    objects
}
