use bevy::prelude::*;

use crate::voxel_scene::VoxelScene;

/// Marker component for the voxel mesh entity
#[derive(Component)]
pub struct VoxelMeshMarker;

/// System that synchronizes the voxel cube to a Bevy mesh
/// TODO: Implement proper mesh generation from Cube data using NonSend resources
fn sync_voxel_mesh(
    mut scene: ResMut<VoxelScene>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    mesh_query: Query<Entity, With<VoxelMeshMarker>>,
) {
    if !scene.mesh_dirty {
        return;
    }

    scene.mesh_dirty = false;

    // Remove old mesh entity if it exists
    for entity in mesh_query.iter() {
        commands.entity(entity).despawn();
    }

    // Create a placeholder cube mesh for now
    let mesh = Cuboid::new(16.0, 1.0, 16.0);

    // Spawn mesh entity
    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.6, 0.3),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.5, 0.0),
        VoxelMeshMarker,
    ));

    info!("Voxel mesh synchronized (placeholder)");
}

/// Plugin for mesh synchronization
pub struct MeshSyncPlugin;

impl Plugin for MeshSyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_voxel_mesh);
    }
}
