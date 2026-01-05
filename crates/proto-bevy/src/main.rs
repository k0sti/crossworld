//! 3D shapes with physics - shapes fall onto a ground plane using Rapier physics.
//!
//! Based on Bevy's 3d_shapes.rs example but with physics simulation.
//!
//! ## Debug Mode
//!
//! Use `--debug N` to enable debug mode:
//! - Enables debug logging and collider visualization
//! - Runs N frames after physics settles
//! - Saves last rendered frame to `output/exit_frame.png`
//!
//! Example: `cargo run --bin proto -- --debug 5`

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

/// Minimum frames needed for render pipeline to initialize (shader compilation, etc.)
/// Bevy's pipelined rendering needs several frames for the render world to catch up
const RENDER_WARMUP_FRAMES: u32 = 10;

/// Resource to track debug mode state
#[derive(Resource)]
struct DebugMode {
    /// Whether debug mode is enabled
    enabled: bool,
    /// Number of frames to run after warmup before exiting
    frames_after_warmup: u32,
    /// Current frame count
    frame_count: u32,
    /// Whether screenshot has been triggered
    screenshot_triggered: bool,
}

impl DebugMode {
    fn new(frames: Option<u32>) -> Self {
        Self {
            enabled: frames.is_some(),
            frames_after_warmup: frames.unwrap_or(0),
            frame_count: 0,
            screenshot_triggered: false,
        }
    }

    /// Total frames needed: warmup + user-specified frames
    fn total_frames(&self) -> u32 {
        RENDER_WARMUP_FRAMES + self.frames_after_warmup
    }
}

/// A generic object that contains mesh, physics body, and collider data.
/// This is the primary building block for entities in the world.
#[derive(Bundle)]
struct BevyObject {
    /// The 3D mesh for rendering
    mesh: Mesh3d,
    /// The material for rendering
    material: MeshMaterial3d<StandardMaterial>,
    /// Transform for position/rotation/scale
    transform: Transform,
    /// Marker component for querying
    shape: Shape,
    /// Physics rigid body type
    rigid_body: RigidBody,
    /// Physics collider shape
    collider: Collider,
    /// Bounciness
    restitution: Restitution,
    /// Friction coefficient
    friction: Friction,
}

impl BevyObject {
    /// Create a new BevyObject with the given mesh, material, transform, and collider
    fn new(
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        transform: Transform,
        collider: Collider,
    ) -> Self {
        Self {
            mesh: Mesh3d(mesh),
            material: MeshMaterial3d(material),
            transform,
            shape: Shape,
            rigid_body: RigidBody::Dynamic,
            collider,
            restitution: Restitution::coefficient(0.3),
            friction: Friction::coefficient(0.5),
        }
    }

    /// Create a static (fixed) version of the object
    fn fixed(
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        transform: Transform,
        collider: Collider,
    ) -> (Mesh3d, MeshMaterial3d<StandardMaterial>, Transform, RigidBody, Collider) {
        (
            Mesh3d(mesh),
            MeshMaterial3d(material),
            transform,
            RigidBody::Fixed,
            collider,
        )
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

    // Insert debug mode resource
    let debug_mode = DebugMode::new(args.debug);
    if debug_mode.enabled {
        info!(
            "[DEBUG] Debug mode enabled, will run {} frame(s) ({} warmup + {} user) and save to output/exit_frame.png",
            debug_mode.total_frames(),
            RENDER_WARMUP_FRAMES,
            debug_mode.frames_after_warmup
        );
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Enable wireframe globally in debug mode
            app.insert_resource(WireframeConfig {
                global: true,
                default_color: Color::srgb(0.0, 1.0, 0.0).into(),
            });
        }
    }
    app.insert_resource(debug_mode);

    app.add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                debug_frame_counter.run_if(|dm: Res<DebugMode>| dm.enabled),
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
    let total = debug_mode.total_frames();
    debug!(
        "[DEBUG] Frame {}/{}",
        debug_mode.frame_count, total
    );

    // On the last frame, trigger screenshot
    if debug_mode.frame_count >= total && !debug_mode.screenshot_triggered {
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
    if debug_mode.frame_count > debug_mode.total_frames() + 10 {
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
    debug_mode: Res<DebugMode>,
) {
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    // Bevy's default primitive dimensions:
    // - Cuboid::default() = half_size (0.5, 0.5, 0.5) -> full size 1x1x1
    // - Capsule3d::default() = radius 0.5, half_length 0.5 -> total height 1.5
    // - Cylinder::default() = radius 0.5, half_height 0.5 -> total height 1.0
    // - Cone::default() = radius 0.5, height 1.0
    // - ConicalFrustum::default() = radius_top 0.25, radius_bottom 0.5, height 0.5
    // - Sphere::default() = radius 0.5
    // - Torus::default() = minor_radius 0.25, major_radius 0.75
    // - Tetrahedron::default() = vertices at unit positions

    // Define shapes with their corresponding accurate colliders
    let shape_data: Vec<(Handle<Mesh>, Collider, &str)> = vec![
        // Cuboid: half_size (0.5, 0.5, 0.5)
        (
            meshes.add(Cuboid::default()),
            Collider::cuboid(0.5, 0.5, 0.5),
            "Cuboid",
        ),
        // Tetrahedron: vertices form a regular tetrahedron
        // Default vertices are approximately at unit positions
        (
            meshes.add(Tetrahedron::default()),
            Collider::convex_hull(&[
                Vec3::new(-0.5, -0.289, -0.289),
                Vec3::new(0.5, -0.289, -0.289),
                Vec3::new(0.0, -0.289, 0.577),
                Vec3::new(0.0, 0.577, 0.0),
            ])
            .unwrap_or_else(|| Collider::ball(0.5)),
            "Tetrahedron",
        ),
        // Capsule3d: radius 0.5, half_length 0.5 (total length along Y = 1.5)
        // Rapier capsule_y takes (half_height, radius) where half_height is the cylinder part
        (
            meshes.add(Capsule3d::default()),
            Collider::capsule_y(0.5, 0.5),
            "Capsule3d",
        ),
        // Torus: major_radius 0.75, minor_radius 0.25
        // Use convex hull approximation since Rapier doesn't have native torus
        (
            meshes.add(Torus::default()),
            create_torus_collider(0.75, 0.25, 16),
            "Torus",
        ),
        // Cylinder: radius 0.5, half_height 0.5
        (
            meshes.add(Cylinder::default()),
            Collider::cylinder(0.5, 0.5),
            "Cylinder",
        ),
        // Cone: radius 0.5, height 1.0 (half_height 0.5)
        (
            meshes.add(Cone::default()),
            Collider::cone(0.5, 0.5),
            "Cone",
        ),
        // ConicalFrustum: radius_top 0.25, radius_bottom 0.5, height 0.5
        // Approximate with convex hull
        (
            meshes.add(ConicalFrustum::default()),
            create_frustum_collider(0.5, 0.25, 0.5, 12),
            "ConicalFrustum",
        ),
        // Icosphere: radius 0.5
        (
            meshes.add(Sphere::default().mesh().ico(5).unwrap()),
            Collider::ball(0.5),
            "Icosphere",
        ),
        // UV Sphere: radius 0.5
        (
            meshes.add(Sphere::default().mesh().uv(32, 18)),
            Collider::ball(0.5),
            "UVSphere",
        ),
    ];

    // Extrusion dimensions:
    // - Rectangle::default() = half_size (0.5, 0.5), extruded 1.0
    // - Capsule2d::default() = radius 0.5, half_length 0.5, extruded 1.0
    // - Annulus::default() = inner 0.25, outer 0.5, extruded 1.0
    // - Circle::default() = radius 0.5, extruded 1.0
    // - Ellipse::default() = half_size (0.5, 0.25), extruded 1.0
    // - RegularPolygon::default() = radius 0.5, 6 sides, extruded 1.0
    // - Triangle2d::default() = base ~0.866, height ~0.75, extruded 1.0

    let extrusion_data: Vec<(Handle<Mesh>, Collider, &str)> = vec![
        // Rectangle extrusion: 1.0 x 1.0 x 1.0
        (
            meshes.add(Extrusion::new(Rectangle::default(), 1.)),
            Collider::cuboid(0.5, 0.5, 0.5),
            "RectExtrusion",
        ),
        // Capsule2d extrusion: radius 0.5, half_length 0.5, depth 1.0
        // Approximate as capsule rotated to Z axis
        (
            meshes.add(Extrusion::new(Capsule2d::default(), 1.)),
            Collider::capsule_z(0.5, 0.5),
            "CapsuleExtrusion",
        ),
        // Annulus extrusion: outer radius 0.5, depth 1.0 (approximate as cylinder)
        (
            meshes.add(Extrusion::new(Annulus::default(), 1.)),
            Collider::cylinder(0.5, 0.5),
            "AnnulusExtrusion",
        ),
        // Circle extrusion: radius 0.5, depth 1.0
        (
            meshes.add(Extrusion::new(Circle::default(), 1.)),
            Collider::cylinder(0.5, 0.5),
            "CircleExtrusion",
        ),
        // Ellipse extrusion: half_size (0.5, 0.25), depth 1.0
        // Approximate with convex hull
        (
            meshes.add(Extrusion::new(Ellipse::default(), 1.)),
            create_ellipse_extrusion_collider(0.5, 0.25, 0.5, 12),
            "EllipseExtrusion",
        ),
        // RegularPolygon extrusion: radius 0.5, 6 sides, depth 1.0
        (
            meshes.add(Extrusion::new(RegularPolygon::default(), 1.)),
            create_polygon_extrusion_collider(0.5, 6, 0.5),
            "PolygonExtrusion",
        ),
        // Triangle2d extrusion
        (
            meshes.add(Extrusion::new(Triangle2d::default(), 1.)),
            Collider::convex_hull(&[
                Vec3::new(-0.5, -0.289, -0.5),
                Vec3::new(0.5, -0.289, -0.5),
                Vec3::new(0.0, 0.577, -0.5),
                Vec3::new(-0.5, -0.289, 0.5),
                Vec3::new(0.5, -0.289, 0.5),
                Vec3::new(0.0, 0.577, 0.5),
            ])
            .unwrap_or_else(|| Collider::cuboid(0.5, 0.5, 0.5)),
            "TriangleExtrusion",
        ),
    ];

    let num_shapes = shape_data.len();

    // Spawn standard shapes using BevyObject
    for (i, (mesh, collider, name)) in shape_data.into_iter().enumerate() {
        let transform = Transform::from_xyz(
            -SHAPES_X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * SHAPES_X_EXTENT,
            8.0 + (i as f32 * 0.5),
            Z_EXTENT / 2.,
        )
        .with_rotation(Quat::from_rotation_x(-PI / 4.));

        commands.spawn(BevyObject::new(
            mesh,
            debug_material.clone(),
            transform,
            collider,
        ));

        if debug_mode.enabled {
            info!("Spawned {} at {:?}", name, transform.translation);
        }
    }

    let num_extrusions = extrusion_data.len();

    // Spawn extrusions using BevyObject
    for (i, (mesh, collider, name)) in extrusion_data.into_iter().enumerate() {
        let transform = Transform::from_xyz(
            -EXTRUSION_X_EXTENT / 2. + i as f32 / (num_extrusions - 1) as f32 * EXTRUSION_X_EXTENT,
            12.0 + (i as f32 * 0.5),
            0.,
        )
        .with_rotation(Quat::from_rotation_x(-PI / 4.));

        commands.spawn(BevyObject::new(
            mesh,
            debug_material.clone(),
            transform,
            collider,
        ));

        if debug_mode.enabled {
            info!("Spawned {} at {:?}", name, transform.translation);
        }
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
    commands.spawn(BevyObject::fixed(
        meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10)),
        materials.add(Color::from(SILVER)),
        Transform::default(),
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
        Text::new(if debug_mode.enabled {
            "DEBUG MODE: Press space to toggle wireframes\nColliders shown in green"
        } else {
            "Normal mode - run with --debug for collider visualization"
        }),
        Node {
            position_type: PositionType::Absolute,
            top: px(12.),
            left: px(12.),
            ..default()
        },
    ));
}

/// Create a convex hull approximation of a torus collider
fn create_torus_collider(major_radius: f32, minor_radius: f32, segments: usize) -> Collider {
    let mut points = Vec::with_capacity(segments * 4);
    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * 2.0 * PI;
        let (sin_a, cos_a) = angle.sin_cos();

        // Outer ring
        let outer = major_radius + minor_radius;
        points.push(Vec3::new(cos_a * outer, minor_radius, sin_a * outer));
        points.push(Vec3::new(cos_a * outer, -minor_radius, sin_a * outer));

        // Inner ring
        let inner = major_radius - minor_radius;
        points.push(Vec3::new(cos_a * inner, minor_radius, sin_a * inner));
        points.push(Vec3::new(cos_a * inner, -minor_radius, sin_a * inner));
    }
    Collider::convex_hull(&points).unwrap_or_else(|| Collider::ball(major_radius))
}

/// Create a convex hull approximation of a conical frustum collider
fn create_frustum_collider(
    radius_bottom: f32,
    radius_top: f32,
    half_height: f32,
    segments: usize,
) -> Collider {
    let mut points = Vec::with_capacity(segments * 2);
    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * 2.0 * PI;
        let (sin_a, cos_a) = angle.sin_cos();

        // Bottom circle
        points.push(Vec3::new(cos_a * radius_bottom, -half_height, sin_a * radius_bottom));
        // Top circle
        points.push(Vec3::new(cos_a * radius_top, half_height, sin_a * radius_top));
    }
    Collider::convex_hull(&points).unwrap_or_else(|| Collider::cylinder(half_height, radius_bottom))
}

/// Create a convex hull approximation of an ellipse extrusion collider
fn create_ellipse_extrusion_collider(
    half_width: f32,
    half_height: f32,
    half_depth: f32,
    segments: usize,
) -> Collider {
    let mut points = Vec::with_capacity(segments * 2);
    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * 2.0 * PI;
        let (sin_a, cos_a) = angle.sin_cos();

        let x = cos_a * half_width;
        let y = sin_a * half_height;

        // Front face
        points.push(Vec3::new(x, y, -half_depth));
        // Back face
        points.push(Vec3::new(x, y, half_depth));
    }
    Collider::convex_hull(&points).unwrap_or_else(|| Collider::cuboid(half_width, half_height, half_depth))
}

/// Create a convex hull approximation of a regular polygon extrusion collider
fn create_polygon_extrusion_collider(radius: f32, sides: usize, half_depth: f32) -> Collider {
    let mut points = Vec::with_capacity(sides * 2);
    for i in 0..sides {
        let angle = (i as f32 / sides as f32) * 2.0 * PI;
        let (sin_a, cos_a) = angle.sin_cos();

        let x = cos_a * radius;
        let y = sin_a * radius;

        // Front face
        points.push(Vec3::new(x, y, -half_depth));
        // Back face
        points.push(Vec3::new(x, y, half_depth));
    }
    Collider::convex_hull(&points).unwrap_or_else(|| Collider::cylinder(half_depth, radius))
}

/// Rotate shapes that are still falling
#[allow(dead_code)]
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
