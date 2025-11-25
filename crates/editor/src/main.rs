use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy_egui::EguiPlugin;
use bevy_mesh::{Indices, PrimitiveTopology};
use cube::{Cube, DefaultMeshBuilder, generate_face_mesh};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Crossworld Voxel Editor".to_string(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_plugins(EditorPlugin)
        .run();
}

struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_scene);
    }
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a camera at (10, 10, 10) looking at origin
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(10.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Add a directional light
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Add an ambient light for better visibility
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 200.0,
        affects_lightmapped_meshes: false,
    });

    // Add a ground plane as a visual reference
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.5, 0.3),
            ..default()
        })),
        Transform::from_xyz(0.0, -0.5, 0.0),
    ));

    // Create a simple test voxel cube and render it
    spawn_test_voxel_cube(&mut commands, &mut meshes, &mut materials);

    info!("Crossworld Voxel Editor initialized");
    info!("Camera positioned at (10, 10, 10) looking at origin");
}

/// Create a simple test voxel cube to verify rendering
fn spawn_test_voxel_cube(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    use std::rc::Rc;

    // Create a simple 2x2x2 voxel cube (depth 1)
    // Material IDs: 1=red, 2=green, 3=blue, 4=yellow
    let cube = Cube::cubes([
        Rc::new(Cube::solid(1)), // Bottom-back-left (red)
        Rc::new(Cube::solid(2)), // Bottom-back-right (green)
        Rc::new(Cube::solid(3)), // Bottom-front-left (blue)
        Rc::new(Cube::solid(4)), // Bottom-front-right (yellow)
        Rc::new(Cube::solid(1)), // Top-back-left (red)
        Rc::new(Cube::solid(2)), // Top-back-right (green)
        Rc::new(Cube::solid(3)), // Top-front-left (blue)
        Rc::new(Cube::solid(4)), // Top-front-right (yellow)
    ]);

    // Generate mesh using cube crate
    let mut builder = DefaultMeshBuilder::new();

    // Simple color mapper based on material ID
    let color_fn = |material_id: i32| -> [f32; 3] {
        match material_id {
            1 => [1.0, 0.0, 0.0], // Red
            2 => [0.0, 1.0, 0.0], // Green
            3 => [0.0, 0.0, 1.0], // Blue
            4 => [1.0, 1.0, 0.0], // Yellow
            _ => [0.5, 0.5, 0.5], // Gray default
        }
    };

    // Generate faces with border materials (not used for simple cube)
    generate_face_mesh(
        &cube,
        &mut builder,
        color_fn,
        1, // max_depth
        [0, 0, 0, 0], // border_materials (not used)
        1, // base_depth
    );

    // Convert to Bevy mesh
    let bevy_mesh = convert_to_bevy_mesh(&builder);

    // Spawn the voxel mesh entity
    commands.spawn((
        Mesh3d(meshes.add(bevy_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            cull_mode: None, // Show both sides for debugging
            ..default()
        })),
        Transform::from_xyz(0.0, 1.0, 0.0), // Position above ground
    ));

    info!("Test voxel cube spawned: {} vertices, {} indices",
          builder.vertices.len() / 3, builder.indices.len());
}

/// Convert DefaultMeshBuilder to Bevy Mesh
fn convert_to_bevy_mesh(builder: &DefaultMeshBuilder) -> Mesh {
    // Create positions, normals, and colors from builder data
    let positions: Vec<[f32; 3]> = builder.vertices
        .chunks(3)
        .map(|v| [v[0], v[1], v[2]])
        .collect();

    let normals: Vec<[f32; 3]> = builder.normals
        .chunks(3)
        .map(|n| [n[0], n[1], n[2]])
        .collect();

    let colors: Vec<[f32; 4]> = builder.colors
        .chunks(3)
        .map(|c| [c[0], c[1], c[2], 1.0])
        .collect();

    info!("Converting voxel mesh: {} vertices, {} triangles",
          positions.len(),
          builder.indices.len() / 3);

    // Create mesh using Bevy's API
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(builder.indices.clone()))
}
