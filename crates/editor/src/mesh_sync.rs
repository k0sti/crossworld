use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy_mesh::{Indices, PrimitiveTopology};
use crossworld_world::GeometryData;
use crate::voxel_scene::VoxelScene;

/// System that synchronizes WorldCube changes to Bevy mesh
pub fn sync_voxel_mesh(
    mut scene: ResMut<VoxelScene>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mesh_query: Query<Entity, With<VoxelMeshMarker>>,
) {
    // Only regenerate if dirty flag is set
    if !scene.mesh_dirty {
        return;
    }

    info!("Regenerating voxel mesh...");

    // Generate mesh from WorldCube (acquire lock)
    let geometry = scene.world.lock().generate_frame();

    if geometry.vertices().is_empty() || geometry.indices().is_empty() {
        info!("No geometry to render");
        scene.clear_dirty();
        return;
    }

    // Convert GeometryData to Bevy Mesh
    let bevy_mesh = convert_geometry_to_mesh(&geometry);

    info!("Generated mesh: {} vertices, {} triangles",
          geometry.vertices().len() / 3,
          geometry.indices().len() / 3);

    // Remove old mesh entity if it exists
    if let Some(entity) = scene.mesh_entity {
        if let Ok(old_entity) = mesh_query.get(entity) {
            commands.entity(old_entity).despawn();
            info!("Despawned old voxel mesh entity");
        }
    }

    // Spawn new mesh entity
    let mesh_handle = meshes.add(bevy_mesh);
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        cull_mode: None, // Show both sides for debugging
        ..default()
    });

    let entity = commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        Transform::from_xyz(0.0, 0.0, 0.0),
        VoxelMeshMarker,
    )).id();

    scene.mesh_entity = Some(entity);
    scene.clear_dirty();

    info!("Voxel mesh updated successfully");
}

/// Convert GeometryData from world crate to Bevy Mesh
pub fn convert_geometry_to_mesh(geometry: &GeometryData) -> Mesh {
    // Convert vertices: Vec<f32> (flat array) to Vec<[f32; 3]>
    let positions: Vec<[f32; 3]> = geometry.vertices()
        .chunks_exact(3)
        .map(|v| [v[0], v[1], v[2]])
        .collect();

    // Convert normals: Vec<f32> (flat array) to Vec<[f32; 3]>
    let normals: Vec<[f32; 3]> = geometry.normals()
        .chunks_exact(3)
        .map(|n| [n[0], n[1], n[2]])
        .collect();

    // Convert colors: Vec<f32> (RGB flat array) to Vec<[f32; 4]> (RGBA)
    let colors: Vec<[f32; 4]> = geometry.colors()
        .chunks_exact(3)
        .map(|c| [c[0], c[1], c[2], 1.0]) // Add alpha channel
        .collect();

    // Create mesh
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(geometry.indices().to_vec()));

    mesh
}

/// Marker component for the voxel mesh entity
#[derive(Component)]
pub struct VoxelMeshMarker;

/// Plugin for mesh synchronization
pub struct MeshSyncPlugin;

impl Plugin for MeshSyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_voxel_mesh);
    }
}
