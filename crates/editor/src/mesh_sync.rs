use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy_mesh::{Indices, PrimitiveTopology};
use cube::{generate_face_mesh, DefaultMeshBuilder, VoxColorMapper, ColorMapper};
use crate::voxel_scene::VoxelScene;
use crate::config::EditorConfig;

/// System that synchronizes Cube changes to Bevy mesh
pub fn sync_voxel_mesh(
    mut scene: ResMut<VoxelScene>,
    config: Res<EditorConfig>,
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

    // Generate mesh from Cube (acquire and release lock)
    let builder = {
        let cube = scene.cube.lock();
        let mut builder = DefaultMeshBuilder::new();
        let color_mapper = VoxColorMapper::new();
        let border_materials = [0, 0, 0, 0]; // All empty (no borders)

        generate_face_mesh(
            &cube,
            &mut builder,
            |v| color_mapper.map(v),
            config.max_depth,
            border_materials,
            config.max_depth, // base_depth = max_depth for proper scaling
        );

        builder
    }; // Lock is released here

    info!("Cube generated: {} vertex components, {} indices, {} normals, {} colors",
          builder.vertices.len(),
          builder.indices.len(),
          builder.normals.len(),
          builder.colors.len());

    if builder.vertices.is_empty() || builder.indices.is_empty() {
        warn!("No geometry to render! vertices: {}, indices: {}",
              builder.vertices.len(),
              builder.indices.len());
        scene.clear_dirty();
        return;
    }

    // Convert mesh data to Bevy Mesh
    let bevy_mesh = convert_mesh_to_bevy(&builder, config.max_depth);

    info!("Generated mesh: {} vertices, {} triangles",
          builder.vertices.len() / 3,
          builder.indices.len() / 3);

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

/// Convert mesh builder data to Bevy Mesh
pub fn convert_mesh_to_bevy(builder: &DefaultMeshBuilder, max_depth: u32) -> Mesh {
    // Scale factor from normalized [0,1] space to world space
    let scale = (1 << max_depth) as f32;

    // Convert vertices: Vec<f32> (flat array) to Vec<[f32; 3]>, scaled to world space
    let positions: Vec<[f32; 3]> = builder.vertices
        .chunks_exact(3)
        .map(|v| [v[0] * scale, v[1] * scale, v[2] * scale])
        .collect();

    // Convert normals: Vec<f32> (flat array) to Vec<[f32; 3]>
    let normals: Vec<[f32; 3]> = builder.normals
        .chunks_exact(3)
        .map(|n| [n[0], n[1], n[2]])
        .collect();

    // Convert colors: Vec<f32> (RGB flat array) to Vec<[f32; 4]> (RGBA)
    let colors: Vec<[f32; 4]> = builder.colors
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
    mesh.insert_indices(Indices::U32(builder.indices.clone()));

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
