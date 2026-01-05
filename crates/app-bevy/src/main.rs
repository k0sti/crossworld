use bevy::prelude::*;
use std::f32::consts::PI;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

/// Spawns a hierarchical tree structure made of cuboids
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.5, 0.3),
            ..default()
        })),
    ));

    // Materials for different tree parts
    let trunk_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.25, 0.1),
        ..default()
    });
    let branch_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.2, 0.1),
        ..default()
    });
    let leaf_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.6, 0.2),
        ..default()
    });

    // Create tree mesh primitives
    let trunk_mesh = meshes.add(Cuboid::new(0.5, 3.0, 0.5));
    let branch_mesh = meshes.add(Cuboid::new(0.2, 1.5, 0.2));
    let leaf_mesh = meshes.add(Cuboid::new(0.8, 0.8, 0.8));

    // Spawn the tree trunk as the root of the hierarchy
    let trunk = commands
        .spawn((
            Mesh3d(trunk_mesh.clone()),
            MeshMaterial3d(trunk_material.clone()),
            Transform::from_xyz(0.0, 1.5, 0.0),
        ))
        .id();

    // Spawn main branches attached to trunk
    let branch_configs = [
        (Vec3::new(0.0, 1.2, 0.0), PI / 6.0, 0.0),          // Front branch
        (Vec3::new(0.0, 1.2, 0.0), PI / 6.0, PI / 2.0),     // Right branch
        (Vec3::new(0.0, 1.2, 0.0), PI / 6.0, PI),           // Back branch
        (Vec3::new(0.0, 1.2, 0.0), PI / 6.0, 3.0 * PI / 2.0), // Left branch
    ];

    for (offset, tilt, rotation) in branch_configs {
        let branch = commands
            .spawn((
                Mesh3d(branch_mesh.clone()),
                MeshMaterial3d(branch_material.clone()),
                Transform::from_translation(offset)
                    .with_rotation(
                        Quat::from_rotation_y(rotation) * Quat::from_rotation_x(tilt),
                    ),
            ))
            .id();

        commands.entity(trunk).add_child(branch);

        // Add leaves to each branch
        let leaf_positions = [
            Vec3::new(0.0, 0.9, 0.0),
            Vec3::new(0.3, 0.6, 0.0),
            Vec3::new(-0.3, 0.6, 0.0),
        ];

        for leaf_pos in leaf_positions {
            let leaf = commands
                .spawn((
                    Mesh3d(leaf_mesh.clone()),
                    MeshMaterial3d(leaf_material.clone()),
                    Transform::from_translation(leaf_pos)
                        .with_scale(Vec3::splat(0.5)),
                ))
                .id();

            commands.entity(branch).add_child(leaf);
        }
    }

    // Add top crown of leaves directly on trunk
    let crown_positions = [
        Vec3::new(0.0, 2.0, 0.0),
        Vec3::new(0.4, 1.8, 0.4),
        Vec3::new(-0.4, 1.8, 0.4),
        Vec3::new(0.4, 1.8, -0.4),
        Vec3::new(-0.4, 1.8, -0.4),
    ];

    for pos in crown_positions {
        let leaf = commands
            .spawn((
                Mesh3d(leaf_mesh.clone()),
                MeshMaterial3d(leaf_material.clone()),
                Transform::from_translation(pos).with_scale(Vec3::splat(0.6)),
            ))
            .id();

        commands.entity(trunk).add_child(leaf);
    }

    // Light
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 300.0,
    });

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(6.0, 5.0, 6.0).looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y),
    ));
}
