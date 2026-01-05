//! 3D shapes with physics - shapes fall onto a ground plane using Rapier physics.
//!
//! Features:
//! - First-person camera controls (WASD + F/V for up/down)
//! - Mouse look when right mouse button is held
//! - Mesh picking with raycast hit display
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

use clap::Parser;

#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::{
    app::AppExit,
    asset::RenderAssetUsages,
    color::palettes::basic::SILVER,
    color::palettes::tailwind::{PINK_100, RED_500},
    input::mouse::AccumulatedMouseMotion,
    log::LogPlugin,
    picking::{mesh_picking::MeshPickingPlugin, pointer::PointerInteraction},
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        view::screenshot::{save_to_disk, Screenshot},
    },
};
use bevy_rapier3d::prelude::*;

#[derive(Parser, Debug)]
#[command(name = "proto-bevy")]
struct Args {
    #[arg(long, num_args = 0..=1, default_missing_value = "1")]
    debug: Option<u32>,
}

const RENDER_WARMUP_FRAMES: u32 = 10;
const SHAPES_X_EXTENT: f32 = 14.0;
const EXTRUSION_X_EXTENT: f32 = 16.0;
const Z_EXTENT: f32 = 5.0;

#[derive(Resource)]
struct DebugMode {
    enabled: bool,
    frames_after_warmup: u32,
    frame_count: u32,
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

    fn total_frames(&self) -> u32 {
        RENDER_WARMUP_FRAMES + self.frames_after_warmup
    }
}

#[derive(Bundle)]
struct BevyObject {
    mesh: Mesh3d,
    material: MeshMaterial3d<StandardMaterial>,
    transform: Transform,
    shape: Shape,
    rigid_body: RigidBody,
    collider: Collider,
    restitution: Restitution,
    friction: Friction,
}

impl BevyObject {
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

#[derive(Component)]
struct FirstPersonCamera {
    move_speed: f32,
    sensitivity: f32,
    pitch: f32,
    yaw: f32,
}

impl Default for FirstPersonCamera {
    fn default() -> Self {
        Self {
            move_speed: 10.0,
            sensitivity: 0.003,
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

#[derive(Resource, Default)]
struct RaycastHitInfo {
    position: Option<Vec3>,
    normal: Option<Vec3>,
}

#[derive(Component)]
struct HitInfoText;

#[derive(Component)]
struct Shape;

fn main() {
    let args = Args::parse();

    let mut app = App::new();

    let log_level = if args.debug.is_some() {
        bevy::log::Level::DEBUG
    } else {
        bevy::log::Level::INFO
    };

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
        MeshPickingPlugin,
        #[cfg(not(target_arch = "wasm32"))]
        WireframePlugin::default(),
    ));

    app.init_resource::<RaycastHitInfo>();

    let debug_mode = DebugMode::new(args.debug);
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.insert_resource(WireframeConfig {
            global: debug_mode.enabled,
            default_color: Color::srgb(0.0, 1.0, 0.0),
        });
    }
    if debug_mode.enabled {
        info!(
            "[DEBUG] Running {} frames ({} warmup + {})",
            debug_mode.total_frames(),
            RENDER_WARMUP_FRAMES,
            debug_mode.frames_after_warmup
        );
    }
    app.insert_resource(debug_mode);

    app.add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                debug_frame_counter.run_if(|dm: Res<DebugMode>| dm.enabled),
                first_person_movement,
                mouse_look,
                update_raycast_info,
                draw_mesh_intersections,
                update_hit_info_text,
                #[cfg(not(target_arch = "wasm32"))]
                toggle_wireframe,
            ),
        )
        .run();
}

fn debug_frame_counter(
    mut commands: Commands,
    mut debug_mode: ResMut<DebugMode>,
    mut exit_writer: MessageWriter<AppExit>,
) {
    debug_mode.frame_count += 1;
    let total = debug_mode.total_frames();
    debug!("[DEBUG] Frame {}/{}", debug_mode.frame_count, total);

    if debug_mode.frame_count >= total && !debug_mode.screenshot_triggered {
        debug_mode.screenshot_triggered = true;

        if let Err(e) = std::fs::create_dir_all("output") {
            error!("[DEBUG] Failed to create output directory: {}", e);
        }

        let output_path = std::path::PathBuf::from("output/exit_frame.png");
        info!("[DEBUG] Capturing screenshot to: {:?}", output_path);

        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(output_path))
            .observe(
                |_trigger: On<bevy::render::view::screenshot::ScreenshotCaptured>,
                 mut exit: MessageWriter<AppExit>| {
                    info!("[DEBUG] Screenshot saved, exiting");
                    exit.write(AppExit::Success);
                },
            );
    }

    if debug_mode.frame_count > debug_mode.total_frames() + 10 {
        warn!("[DEBUG] Screenshot timeout, forcing exit");
        exit_writer.write(AppExit::Success);
    }
}

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

    // Bevy default primitive sizes:
    // Cuboid: half_size = 0.5 (1x1x1 box)
    // Sphere: radius = 0.5
    // Capsule3d: radius = 0.25, half_length = 0.5 (total height = 1.5)
    // Cylinder: radius = 0.5, half_height = 0.5 (total height = 1.0)
    // Cone: radius = 0.5, height = 1.0
    // Torus: minor_radius = 0.25, major_radius = 0.75
    // Tetrahedron: edge length ~1.73 (vertices at distance 1.0 from center)
    // ConicalFrustum: radius_top = 0.25, radius_bottom = 0.5, height = 0.5
    let shape_data: Vec<(Handle<Mesh>, Collider, &str)> = vec![
        (meshes.add(Cuboid::default()), Collider::cuboid(0.5, 0.5, 0.5), "Cuboid"),
        (
            meshes.add(Tetrahedron::default()),
            create_tetrahedron_collider(),
            "Tetrahedron",
        ),
        // Capsule3d: radius=0.25, half_length=0.5 → capsule_y(half_segment, radius)
        (meshes.add(Capsule3d::default()), Collider::capsule_y(0.5, 0.25), "Capsule3d"),
        (meshes.add(Torus::default()), create_torus_collider(0.75, 0.25, 16), "Torus"),
        (meshes.add(Cylinder::default()), Collider::cylinder(0.5, 0.5), "Cylinder"),
        // Cone: height=1.0 means half_height=0.5
        (meshes.add(Cone::default()), Collider::cone(0.5, 0.5), "Cone"),
        (meshes.add(ConicalFrustum::default()), create_frustum_collider(0.5, 0.25, 0.25, 12), "ConicalFrustum"),
        (meshes.add(Sphere::default().mesh().ico(5).unwrap()), Collider::ball(0.5), "Icosphere"),
        (meshes.add(Sphere::default().mesh().uv(32, 18)), Collider::ball(0.5), "UVSphere"),
    ];

    // 2D shape defaults for extrusions:
    // Rectangle: half_size = (0.5, 0.5)
    // Capsule2d: radius = 0.25, half_length = 0.5
    // Annulus: inner_radius = 0.25, outer_radius = 0.5
    // Circle: radius = 0.5
    // Ellipse: half_size = (0.5, 0.25)
    // RegularPolygon: circumradius = 0.5, 6 sides
    // Triangle2d: default equilateral with circumradius ~0.58
    let extrusion_data: Vec<(Handle<Mesh>, Collider, &str)> = vec![
        (meshes.add(Extrusion::new(Rectangle::default(), 1.)), Collider::cuboid(0.5, 0.5, 0.5), "RectExtrusion"),
        // Capsule2d extruded: radius=0.25, half_length=0.5, depth=0.5
        (meshes.add(Extrusion::new(Capsule2d::default(), 1.)), Collider::capsule_z(0.5, 0.25), "CapsuleExtrusion"),
        // Annulus: approximate with cylinder at outer radius
        (meshes.add(Extrusion::new(Annulus::default(), 1.)), Collider::cylinder(0.5, 0.5), "AnnulusExtrusion"),
        (meshes.add(Extrusion::new(Circle::default(), 1.)), Collider::cylinder(0.5, 0.5), "CircleExtrusion"),
        // Ellipse: half_size = (0.5, 0.25)
        (meshes.add(Extrusion::new(Ellipse::default(), 1.)), create_ellipse_extrusion_collider(0.5, 0.25, 0.5, 12), "EllipseExtrusion"),
        (meshes.add(Extrusion::new(RegularPolygon::default(), 1.)), create_polygon_extrusion_collider(0.5, 6, 0.5), "PolygonExtrusion"),
        (
            meshes.add(Extrusion::new(Triangle2d::default(), 1.)),
            create_triangle_extrusion_collider(0.5),
            "TriangleExtrusion",
        ),
    ];

    let num_shapes = shape_data.len();
    for (i, (mesh, collider, name)) in shape_data.into_iter().enumerate() {
        let transform = Transform::from_xyz(
            -SHAPES_X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * SHAPES_X_EXTENT,
            8.0 + (i as f32 * 0.5),
            Z_EXTENT / 2.,
        )
        .with_rotation(Quat::from_rotation_x(-PI / 4.));

        commands.spawn(BevyObject::new(mesh, debug_material.clone(), transform, collider));

        if debug_mode.enabled {
            info!("Spawned {} at {:?}", name, transform.translation);
        }
    }

    let num_extrusions = extrusion_data.len();
    for (i, (mesh, collider, name)) in extrusion_data.into_iter().enumerate() {
        let transform = Transform::from_xyz(
            -EXTRUSION_X_EXTENT / 2. + i as f32 / (num_extrusions - 1) as f32 * EXTRUSION_X_EXTENT,
            12.0 + (i as f32 * 0.5),
            0.,
        )
        .with_rotation(Quat::from_rotation_x(-PI / 4.));

        commands.spawn(BevyObject::new(mesh, debug_material.clone(), transform, collider));

        if debug_mode.enabled {
            info!("Spawned {} at {:?}", name, transform.translation);
        }
    }

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

    commands.spawn(BevyObject::fixed(
        meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10)),
        materials.add(Color::from(SILVER)),
        Transform::default(),
        Collider::cuboid(25.0, 0.01, 25.0),
    ));

    let camera_pos = Vec3::new(0.0, 12., 24.0);
    let look_target = Vec3::new(0., 4., 0.);
    let direction = (look_target - camera_pos).normalize();
    let initial_yaw = direction.x.atan2(direction.z);
    let initial_pitch = (-direction.y).asin();

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(camera_pos).looking_at(look_target, Vec3::Y),
        FirstPersonCamera {
            pitch: initial_pitch,
            yaw: initial_yaw,
            ..default()
        },
    ));

    commands.spawn((
        Text::new("WASD - Move | F/V - Up/Down | RMB - Look | Space - Wireframe"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12.),
            left: px(12.),
            ..default()
        },
    ));

    commands.spawn((
        Text::new("Hit: None"),
        HitInfoText,
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12.),
            left: px(12.),
            ..default()
        },
    ));
}

fn create_tetrahedron_collider() -> Collider {
    // Bevy's Tetrahedron::default() vertices (regular tetrahedron)
    let a = 1.0 / 3.0_f32.sqrt();
    Collider::convex_hull(&[
        Vec3::new(0.0, a, 0.0),
        Vec3::new(-0.5, -a / 2.0, a * 0.866),
        Vec3::new(-0.5, -a / 2.0, -a * 0.866),
        Vec3::new(1.0, -a / 2.0, 0.0),
    ])
    .unwrap_or_else(|| Collider::ball(0.5))
}

fn create_torus_collider(major_radius: f32, minor_radius: f32, segments: usize) -> Collider {
    let mut points = Vec::with_capacity(segments * 4);
    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * 2.0 * PI;
        let (sin_a, cos_a) = angle.sin_cos();
        let outer = major_radius + minor_radius;
        points.push(Vec3::new(cos_a * outer, minor_radius, sin_a * outer));
        points.push(Vec3::new(cos_a * outer, -minor_radius, sin_a * outer));
        let inner = major_radius - minor_radius;
        points.push(Vec3::new(cos_a * inner, minor_radius, sin_a * inner));
        points.push(Vec3::new(cos_a * inner, -minor_radius, sin_a * inner));
    }
    Collider::convex_hull(&points).unwrap_or_else(|| Collider::ball(major_radius))
}

fn create_frustum_collider(radius_bottom: f32, radius_top: f32, half_height: f32, segments: usize) -> Collider {
    let mut points = Vec::with_capacity(segments * 2);
    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * 2.0 * PI;
        let (sin_a, cos_a) = angle.sin_cos();
        points.push(Vec3::new(cos_a * radius_bottom, -half_height, sin_a * radius_bottom));
        points.push(Vec3::new(cos_a * radius_top, half_height, sin_a * radius_top));
    }
    Collider::convex_hull(&points).unwrap_or_else(|| Collider::cylinder(half_height, radius_bottom))
}

fn create_ellipse_extrusion_collider(half_width: f32, half_height: f32, half_depth: f32, segments: usize) -> Collider {
    let mut points = Vec::with_capacity(segments * 2);
    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * 2.0 * PI;
        let (sin_a, cos_a) = angle.sin_cos();
        let x = cos_a * half_width;
        let y = sin_a * half_height;
        points.push(Vec3::new(x, y, -half_depth));
        points.push(Vec3::new(x, y, half_depth));
    }
    Collider::convex_hull(&points).unwrap_or_else(|| Collider::cuboid(half_width, half_height, half_depth))
}

fn create_polygon_extrusion_collider(radius: f32, sides: usize, half_depth: f32) -> Collider {
    let mut points = Vec::with_capacity(sides * 2);
    for i in 0..sides {
        let angle = (i as f32 / sides as f32) * 2.0 * PI;
        let (sin_a, cos_a) = angle.sin_cos();
        let x = cos_a * radius;
        let y = sin_a * radius;
        points.push(Vec3::new(x, y, -half_depth));
        points.push(Vec3::new(x, y, half_depth));
    }
    Collider::convex_hull(&points).unwrap_or_else(|| Collider::cylinder(half_depth, radius))
}

fn create_triangle_extrusion_collider(half_depth: f32) -> Collider {
    // Triangle2d::default() is equilateral with vertices at:
    // top: (0, 0.5), bottom-left: (-0.5, -0.5), bottom-right: (0.5, -0.5)
    Collider::convex_hull(&[
        Vec3::new(0.0, 0.5, -half_depth),
        Vec3::new(-0.5, -0.5, -half_depth),
        Vec3::new(0.5, -0.5, -half_depth),
        Vec3::new(0.0, 0.5, half_depth),
        Vec3::new(-0.5, -0.5, half_depth),
        Vec3::new(0.5, -0.5, half_depth),
    ])
    .unwrap_or_else(|| Collider::cuboid(0.5, 0.5, half_depth))
}

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
fn toggle_wireframe(mut wireframe_config: ResMut<WireframeConfig>, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(KeyCode::Space) {
        wireframe_config.global = !wireframe_config.global;
    }
}

fn first_person_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &FirstPersonCamera)>,
) {
    for (mut transform, controller) in &mut query {
        let mut velocity = Vec3::ZERO;
        let forward = transform.forward();
        let forward_flat = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right = transform.right();
        let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

        if keyboard.pressed(KeyCode::KeyW) { velocity += forward_flat; }
        if keyboard.pressed(KeyCode::KeyS) { velocity -= forward_flat; }
        if keyboard.pressed(KeyCode::KeyD) { velocity += right_flat; }
        if keyboard.pressed(KeyCode::KeyA) { velocity -= right_flat; }
        if keyboard.pressed(KeyCode::KeyF) { velocity += Vec3::Y; }
        if keyboard.pressed(KeyCode::KeyV) { velocity -= Vec3::Y; }

        if velocity.length_squared() > 0.0 {
            velocity = velocity.normalize() * controller.move_speed * time.delta_secs();
            transform.translation += velocity;
        }
    }
}

fn mouse_look(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut query: Query<(&mut Transform, &mut FirstPersonCamera)>,
) {
    // Skip the first frame when button is pressed to avoid jump from accumulated motion
    if !mouse_button.pressed(MouseButton::Right) || mouse_button.just_pressed(MouseButton::Right) {
        return;
    }
    let delta = mouse_motion.delta;
    if delta == Vec2::ZERO {
        return;
    }
    for (mut transform, mut controller) in &mut query {
        controller.yaw -= delta.x * controller.sensitivity;
        controller.pitch -= delta.y * controller.sensitivity;
        controller.pitch = controller.pitch.clamp(-1.55, 1.55);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, controller.yaw, controller.pitch, 0.0);
    }
}

fn update_raycast_info(pointers: Query<&PointerInteraction>, mut hit_info: ResMut<RaycastHitInfo>) {
    if let Some((position, normal)) = pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(_entity, hit)| hit.position.zip(hit.normal))
        .next()
    {
        hit_info.position = Some(position);
        hit_info.normal = Some(normal);
    } else {
        hit_info.position = None;
        hit_info.normal = None;
    }
}

fn draw_mesh_intersections(pointers: Query<&PointerInteraction>, mut gizmos: Gizmos) {
    for (point, normal) in pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(_entity, hit)| hit.position.zip(hit.normal))
    {
        gizmos.sphere(Isometry3d::from_translation(point), 0.05, RED_500);
        gizmos.arrow(point, point + normal.normalize() * 0.5, PINK_100);
    }
}

fn update_hit_info_text(hit_info: Res<RaycastHitInfo>, mut query: Query<&mut Text, With<HitInfoText>>) {
    for mut text in &mut query {
        if let (Some(pos), Some(normal)) = (hit_info.position, hit_info.normal) {
            **text = format!(
                "Hit: ({:.2}, {:.2}, {:.2}) N: ({:.2}, {:.2}, {:.2})",
                pos.x, pos.y, pos.z, normal.x, normal.y, normal.z
            );
        } else {
            **text = "Hit: None".to_string();
        }
    }
}
