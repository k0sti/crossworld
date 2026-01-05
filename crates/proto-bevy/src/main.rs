//! 3D shapes with physics - shapes fall onto a ground plane using Rapier physics.
//!
//! Based on Bevy's 3d_shapes.rs example but with physics simulation.
//!
//! ## Debug Mode
//!
//! Use `--debug N` to enable debug mode:
//! - Enables debug logging
//! - Runs N frames after physics settles
//! - Saves last rendered frame to `output/exit_frame.png`
//!
//! Example: `cargo run --bin proto-bevy -- --debug 5`

use std::f32::consts::PI;
use std::path::Path;

use clap::Parser;

#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::{
    app::AppExit,
    asset::RenderAssetUsages,
    color::palettes::basic::SILVER,
    log::LogPlugin,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        view::screenshot::{save_to_disk, Screenshot},
    },
};
use bevy_rapier3d::prelude::*;

/// Proto-Bevy: 3D shapes with physics simulation
#[derive(Parser, Debug)]
#[command(name = "proto-bevy")]
#[command(about = "3D shapes with physics simulation using Bevy and Rapier")]
struct Args {
    /// Debug mode: run N frames after startup, save final frame to output/exit_frame.png
    /// Default is 1 frame when --debug is specified without a value
    #[arg(long, num_args = 0..=1, default_missing_value = "1")]
    debug: Option<u32>,
}

/// Resource to track debug mode state
#[derive(Resource)]
struct DebugMode {
    /// Number of frames to run before exiting
    frames_to_run: u32,
    /// Current frame count
    frame_count: u32,
    /// Whether screenshot has been triggered
    screenshot_triggered: bool,
}

impl DebugMode {
    fn new(frames: u32) -> Self {
        Self {
            frames_to_run: frames,
            frame_count: 0,
            screenshot_triggered: false,
        }
    }
}

fn main() {
    let args = Args::parse();

    let mut app = App::new();

    // Configure logging based on debug mode
    let log_level = if args.debug.is_some() {
        bevy::log::Level::DEBUG
    } else {
        bevy::log::Level::INFO
    };

    // Build plugins with appropriate log level
    let default_plugins = DefaultPlugins
        .set(ImagePlugin::default_nearest())
        .set(LogPlugin {
            level: log_level,
            filter: "wgpu=error,naga=warn".to_string(),
            ..default()
        });

    app.add_plugins((
        default_plugins,
        RapierPhysicsPlugin::<NoUserData>::default(),
        RapierDebugRenderPlugin::default(),
        #[cfg(not(target_arch = "wasm32"))]
        WireframePlugin::default(),
    ));

    // Insert debug mode resource if enabled
    if let Some(frames) = args.debug {
        info!(
            "[DEBUG] Debug mode enabled, will run {} frame(s) and save to output/exit_frame.png",
            frames
        );
        app.insert_resource(DebugMode::new(frames));
        app.add_systems(Update, debug_frame_counter);
    }

    app.add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                //rotate,
                #[cfg(not(target_arch = "wasm32"))]
                toggle_wireframe,
            ),
        )
        .run();
}

/// System to count frames and exit in debug mode
fn debug_frame_counter(
    mut commands: Commands,
    mut debug_mode: ResMut<DebugMode>,
    mut exit_writer: MessageWriter<AppExit>,
) {
    debug_mode.frame_count += 1;
    debug!(
        "[DEBUG] Frame {}/{}",
        debug_mode.frame_count, debug_mode.frames_to_run
    );

    // On the last frame, trigger screenshot
    if debug_mode.frame_count >= debug_mode.frames_to_run && !debug_mode.screenshot_triggered {
        debug_mode.screenshot_triggered = true;

        // Ensure output directory exists
        let output_dir = Path::new("output");
        if !output_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(output_dir) {
                error!("[DEBUG] Failed to create output directory: {}", e);
            }
        }

        let output_path = "output/exit_frame.png".to_string();
        info!("[DEBUG] Capturing screenshot to: {}", output_path);

        // Spawn screenshot entity with save observer
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(output_path.clone()))
            .observe(
                move |_trigger: On<bevy::render::view::screenshot::ScreenshotCaptured>,
                      mut exit: MessageWriter<AppExit>| {
                    info!("[DEBUG] Screenshot saved, exiting application");
                    exit.write(AppExit::Success);
                },
            );
    }

    // Safety timeout - if screenshot hasn't completed after extra frames, exit anyway
    if debug_mode.frame_count > debug_mode.frames_to_run + 10 {
        warn!("[DEBUG] Screenshot timeout, forcing exit");
        exit_writer.write(AppExit::Success);
    }
}

/// A marker component for our shapes so we can query them separately from the ground plane
#[derive(Component)]
struct Shape;

const SHAPES_X_EXTENT: f32 = 14.0;
const EXTRUSION_X_EXTENT: f32 = 16.0;
const Z_EXTENT: f32 = 5.0;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    // Standard 3D shapes
    let shapes = [
        meshes.add(Cuboid::default()),
        meshes.add(Tetrahedron::default()),
        meshes.add(Capsule3d::default()),
        meshes.add(Torus::default()),
        meshes.add(Cylinder::default()),
        meshes.add(Cone::default()),
        meshes.add(ConicalFrustum::default()),
        meshes.add(Sphere::default().mesh().ico(5).unwrap()),
        meshes.add(Sphere::default().mesh().uv(32, 18)),
        meshes.add(Segment3d::default()),
        meshes.add(Polyline3d::new(vec![
            Vec3::new(-0.5, 0.0, 0.0),
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(0.0, 0.5, 0.0),
        ])),
    ];

    // Colliders for each shape (approximations)
    let shape_colliders = [
        Collider::cuboid(0.5, 0.5, 0.5), // Cuboid
        Collider::convex_hull(&[
            // Tetrahedron
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::new(-0.5, -0.5, 0.5),
            Vec3::new(0.5, -0.5, 0.5),
            Vec3::new(0.0, -0.5, -0.5),
        ])
        .unwrap_or_else(|| Collider::ball(0.5)),
        Collider::capsule_y(0.25, 0.25),   // Capsule3d
        Collider::ball(0.5),               // Torus (approximation)
        Collider::cylinder(0.5, 0.25),     // Cylinder
        Collider::cone(0.5, 0.25),         // Cone
        Collider::cone(0.5, 0.4),          // ConicalFrustum (approximation)
        Collider::ball(0.5),               // Icosphere
        Collider::ball(0.5),               // UV Sphere
        Collider::cuboid(0.5, 0.05, 0.05), // Segment3d (line approximation)
        Collider::cuboid(0.5, 0.25, 0.05), // Polyline3d (approximation)
    ];

    // Extruded 2D shapes
    let extrusions = [
        meshes.add(Extrusion::new(Rectangle::default(), 1.)),
        meshes.add(Extrusion::new(Capsule2d::default(), 1.)),
        meshes.add(Extrusion::new(Annulus::default(), 1.)),
        meshes.add(Extrusion::new(Circle::default(), 1.)),
        meshes.add(Extrusion::new(Ellipse::default(), 1.)),
        meshes.add(Extrusion::new(RegularPolygon::default(), 1.)),
        meshes.add(Extrusion::new(Triangle2d::default(), 1.)),
    ];

    // Colliders for extrusions (approximations)
    let extrusion_colliders = [
        Collider::cuboid(0.5, 0.5, 0.5), // Rectangle extrusion
        Collider::capsule_y(0.25, 0.25), // Capsule2d extrusion
        Collider::cylinder(0.5, 0.5),    // Annulus extrusion
        Collider::cylinder(0.5, 0.5),    // Circle extrusion
        Collider::cylinder(0.5, 0.4),    // Ellipse extrusion
        Collider::cylinder(0.5, 0.5),    // RegularPolygon extrusion
        Collider::convex_hull(&[
            // Triangle extrusion
            Vec3::new(-0.5, -0.5, -0.5),
            Vec3::new(0.5, -0.5, -0.5),
            Vec3::new(0.0, 0.5, -0.5),
            Vec3::new(-0.5, -0.5, 0.5),
            Vec3::new(0.5, -0.5, 0.5),
            Vec3::new(0.0, 0.5, 0.5),
        ])
        .unwrap_or_else(|| Collider::ball(0.5)),
    ];

    let num_shapes = shapes.len();

    // Spawn standard shapes with physics
    for (i, (shape, collider)) in shapes.into_iter().zip(shape_colliders).enumerate() {
        commands.spawn((
            Mesh3d(shape),
            MeshMaterial3d(debug_material.clone()),
            Transform::from_xyz(
                -SHAPES_X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * SHAPES_X_EXTENT,
                8.0 + (i as f32 * 0.5), // Stagger heights for interesting collision
                Z_EXTENT / 2.,
            )
            .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            Shape,
            RigidBody::Dynamic,
            collider,
            Restitution::coefficient(0.3),
            Friction::coefficient(0.5),
        ));
    }

    let num_extrusions = extrusions.len();

    // Spawn extrusions with physics
    for (i, (shape, collider)) in extrusions.into_iter().zip(extrusion_colliders).enumerate() {
        commands.spawn((
            Mesh3d(shape),
            MeshMaterial3d(debug_material.clone()),
            Transform::from_xyz(
                -EXTRUSION_X_EXTENT / 2.
                    + i as f32 / (num_extrusions - 1) as f32 * EXTRUSION_X_EXTENT,
                12.0 + (i as f32 * 0.5), // Higher starting point
                0.,
            )
            .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            Shape,
            RigidBody::Dynamic,
            collider,
            Restitution::coefficient(0.3),
            Friction::coefficient(0.5),
        ));
    }

    // Point light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // Ground plane with physics collider
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
        MeshMaterial3d(materials.add(Color::from(SILVER))),
        RigidBody::Fixed,
        Collider::cuboid(25.0, 0.01, 25.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 12., 24.0).looking_at(Vec3::new(0., 4., 0.), Vec3::Y),
    ));

    // Instructions text
    #[cfg(not(target_arch = "wasm32"))]
    commands.spawn((
        Text::new("Press space to toggle wireframes"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12.),
            left: px(12.),
            ..default()
        },
    ));
}

/// Rotate shapes that are still falling
fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() / 2.);
    }
}

/// Creates a colorful test pattern
fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn toggle_wireframe(
    mut wireframe_config: ResMut<WireframeConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        wireframe_config.global = !wireframe_config.global;
    }
}
