//! Physics Component Testbed
//!
//! Compares physics behavior between:
//! - Left: Simple flat ground using cuboid collider
//! - Right: Flat ground constructed from Cube objects using terrain collider
//!
//! Scene configuration can be loaded from Steel (Scheme) files when the
//! `steel` feature is enabled. See `config/scene.scm` for an example.

#![allow(clippy::too_many_arguments)]

use app::{App, FrameContext, InputState};
use crossworld_physics::{
    create_box_collider,
    rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder},
    CubeObject, PhysicsWorld, VoxelColliderBuilder,
};
use cube::Cube;
use glam::{Quat, Vec3};
use glow::{Context, HasContext, COLOR_BUFFER_BIT, DEPTH_BUFFER_BIT, SCISSOR_TEST};
use renderer::{Camera, MeshRenderer, OrbitController, OrbitControllerConfig};
use std::rc::Rc;
use std::time::Instant;

#[cfg(feature = "steel")]
mod steel_scene;
#[cfg(feature = "steel")]
use steel_scene::{CameraConfig, GroundConfig, TestbedConfig};
#[cfg(feature = "steel")]
use std::path::Path;

/// Physics state for display
#[derive(Default, Clone)]
pub struct PhysicsState {
    pub falling_position: Vec3,
    pub falling_velocity: Vec3,
    pub is_on_ground: bool,
}

impl PhysicsState {
    pub fn format_compact(&self) -> String {
        let ground_status = if self.is_on_ground { "HIT" } else { "AIR" };
        format!(
            "Y: {:.2} | Vel: {:.2} | {}",
            self.falling_position.y, self.falling_velocity.y, ground_status
        )
    }
}

/// Physics testbed state for one side (left = cuboid, right = terrain)
pub struct PhysicsScene {
    pub world: PhysicsWorld,
    pub falling_objects: Vec<CubeObject>,
    pub ground_mesh_index: Option<usize>,
    pub falling_mesh_indices: Vec<usize>,
    #[allow(dead_code)]
    pub ground_collider_type: &'static str,
    pub states: Vec<PhysicsState>,
}

/// Ground configuration for scene creation
#[derive(Clone)]
pub struct GroundSettings {
    /// Size of ground cube edge (default 8.0)
    pub size: f32,
    /// Material/color index for ground
    pub material: u8,
    /// Center position of the ground (default: origin-centered with top at Y=0)
    pub center: Vec3,
}

impl Default for GroundSettings {
    fn default() -> Self {
        let size = 8.0;
        Self {
            size,
            material: 32,
            center: Vec3::new(0.0, -size / 2.0, 0.0),
        }
    }
}

/// Object configuration for scene creation
#[derive(Clone)]
pub struct ObjectSettings {
    /// Initial position of the object
    pub position: Vec3,
    /// Initial rotation of the object
    pub rotation: Quat,
    /// Size of the object (half-extents)
    pub size: Vec3,
    /// Mass of the object
    pub mass: f32,
    /// Material/color index for the object
    pub material: u8,
}

impl Default for ObjectSettings {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 6.0, 0.0),
            rotation: Quat::from_euler(glam::EulerRot::XYZ, 0.1, 0.2, 0.3),
            size: Vec3::new(0.4, 0.4, 0.4),
            mass: 1.0,
            material: 224,
        }
    }
}

/// Physics testbed application
pub struct PhysicsTestbed {
    // Rendering
    mesh_renderer: MeshRenderer,
    camera: Camera,
    orbit_controller: OrbitController,

    // Physics scenes
    left_scene: Option<PhysicsScene>,  // Cuboid ground
    right_scene: Option<PhysicsScene>, // Terrain collider ground

    // Cubes for rendering (one for each scene with different materials)
    left_ground_cube: Option<Rc<Cube<u8>>>,
    right_ground_cube: Option<Rc<Cube<u8>>>,
    #[allow(dead_code)]
    falling_cube: Option<Rc<Cube<u8>>>,

    // Ground configuration (separate for left and right scenes)
    left_ground_settings: GroundSettings,
    right_ground_settings: GroundSettings,

    // Object configurations (multiple objects supported)
    object_settings: Vec<ObjectSettings>,

    // Config file path for reloading
    #[cfg(feature = "steel")]
    config_path: Option<std::path::PathBuf>,

    // UI state
    reset_requested: bool,

    // Timing
    frame_count: u64,
    debug_frames: Option<u64>,
    physics_dt: f32,
    last_physics_update: Instant,
}

impl Default for PhysicsTestbed {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsTestbed {
    pub fn new() -> Self {
        // Camera target is the top of the ground cube (Y=0)
        let camera_target = Vec3::new(0.0, 0.0, 0.0);
        // Camera positioned above and to the side, looking at ground top
        let camera_position = Vec3::new(10.0, 8.0, 10.0);

        // Configure orbit controller with appropriate limits
        let orbit_config = OrbitControllerConfig {
            mouse_sensitivity: 0.005,
            zoom_sensitivity: 0.5,
            min_distance: 2.0,
            max_distance: 30.0,
        };

        Self {
            mesh_renderer: MeshRenderer::new(),
            camera: Camera::look_at(camera_position, camera_target, Vec3::Y),
            orbit_controller: OrbitController::new(camera_target, orbit_config),
            left_scene: None,
            right_scene: None,
            left_ground_cube: None,
            right_ground_cube: None,
            falling_cube: None,
            left_ground_settings: GroundSettings::default(),
            right_ground_settings: GroundSettings::default(),
            object_settings: vec![ObjectSettings::default()],
            #[cfg(feature = "steel")]
            config_path: None,
            reset_requested: false,
            frame_count: 0,
            debug_frames: None,
            physics_dt: 1.0 / 60.0,
            last_physics_update: Instant::now(),
        }
    }

    pub fn with_debug_frames(mut self, frames: Option<u64>) -> Self {
        self.debug_frames = frames;
        self
    }

    /// Load configuration from a Steel file
    ///
    /// Returns a new PhysicsTestbed configured from the Steel scene configuration.
    /// Falls back to default configuration if the file doesn't exist or fails to parse.
    #[cfg(feature = "steel")]
    pub fn from_config_file(path: &Path) -> Self {
        match Self::try_from_config_file(path) {
            Ok(testbed) => testbed,
            Err(e) => {
                eprintln!("[Testbed] Warning: Failed to load config from {:?}: {}", path, e);
                eprintln!("[Testbed] Using default configuration");
                Self::new()
            }
        }
    }

    /// Try to load configuration from a Steel file
    #[cfg(feature = "steel")]
    pub fn try_from_config_file(path: &Path) -> Result<Self, String> {
        let mut config = TestbedConfig::new();
        config.load_file(path)?;

        // Extract camera configuration
        let camera_config = config.extract_camera("scene-camera")?;

        // Extract ground configurations:
        // - Left scene uses cuboid collider, so it needs ground-2 (ground-cuboid)
        // - Right scene uses terrain collider, so it needs ground-1 (ground-cube)
        let left_ground_config = config.extract_ground("ground-2")?;
        let right_ground_config = config.extract_ground("ground-1")?;

        // Extract object configurations from scene-objects
        let objects = config.extract_objects("scene-objects")?;
        if objects.is_empty() {
            return Err("scene-objects must contain at least one object".to_string());
        }

        Self::from_config(path, &camera_config, &left_ground_config, &right_ground_config, &objects)
    }

    /// Convert a GroundConfig to GroundSettings
    #[cfg(feature = "steel")]
    fn ground_config_to_settings(ground_config: &GroundConfig) -> GroundSettings {
        match ground_config {
            GroundConfig::SolidCube {
                material,
                size_shift,
                center,
            } => GroundSettings {
                size: (1 << size_shift) as f32,
                material: *material,
                center: center.to_vec3(),
            },
            GroundConfig::Cuboid {
                width,
                center,
            } => GroundSettings {
                size: *width,
                material: 32,
                center: center.to_vec3(),
            },
        }
    }

    /// Convert an ObjectConfig to ObjectSettings
    #[cfg(feature = "steel")]
    fn object_config_to_settings(object_config: &steel_scene::ObjectConfig) -> ObjectSettings {
        ObjectSettings {
            position: object_config.position.to_vec3(),
            rotation: object_config.rotation.to_quat(),
            size: object_config.size.to_vec3(),
            mass: object_config.mass,
            material: object_config.material,
        }
    }

    /// Create testbed from camera and ground configurations
    #[cfg(feature = "steel")]
    fn from_config(
        path: &Path,
        camera_config: &CameraConfig,
        left_ground_config: &GroundConfig,
        right_ground_config: &GroundConfig,
        object_configs: &[steel_scene::ObjectConfig],
    ) -> Result<Self, String> {
        let camera_position = camera_config.position.to_vec3();
        let camera_target = camera_config.look_at.to_vec3();

        // Convert ground configs to settings
        let left_ground_settings = Self::ground_config_to_settings(left_ground_config);
        let right_ground_settings = Self::ground_config_to_settings(right_ground_config);

        // Convert all object configs to settings
        let object_settings: Vec<ObjectSettings> = object_configs
            .iter()
            .map(Self::object_config_to_settings)
            .collect();

        let orbit_config = OrbitControllerConfig {
            mouse_sensitivity: 0.005,
            zoom_sensitivity: 0.5,
            min_distance: 2.0,
            max_distance: 30.0,
        };

        Ok(Self {
            mesh_renderer: MeshRenderer::new(),
            camera: Camera::look_at(camera_position, camera_target, Vec3::Y),
            orbit_controller: OrbitController::new(camera_target, orbit_config),
            left_scene: None,
            right_scene: None,
            left_ground_cube: None,
            right_ground_cube: None,
            falling_cube: None,
            left_ground_settings,
            right_ground_settings,
            object_settings,
            config_path: Some(path.to_path_buf()),
            reset_requested: false,
            frame_count: 0,
            debug_frames: None,
            physics_dt: 1.0 / 60.0,
            last_physics_update: Instant::now(),
        })
    }

    /// Reload configuration from the config file (if available)
    #[cfg(feature = "steel")]
    fn reload_config(&mut self) {
        if let Some(path) = &self.config_path.clone() {
            let mut config = TestbedConfig::new();
            if let Err(e) = config.load_file(path) {
                eprintln!("[Testbed] Warning: Failed to reload config: {}", e);
                return;
            }

            // Reload camera configuration
            if let Ok(camera_config) = config.extract_camera("scene-camera") {
                let camera_position = camera_config.position.to_vec3();
                let camera_target = camera_config.look_at.to_vec3();
                self.camera = Camera::look_at(camera_position, camera_target, Vec3::Y);
                self.orbit_controller.target = camera_target;
            }

            // Reload ground configurations:
            // - Left scene uses cuboid collider, so it needs ground-2 (ground-cuboid)
            // - Right scene uses terrain collider, so it needs ground-1 (ground-cube)
            if let Ok(left_ground_config) = config.extract_ground("ground-2") {
                self.left_ground_settings = Self::ground_config_to_settings(&left_ground_config);
            }
            if let Ok(right_ground_config) = config.extract_ground("ground-1") {
                self.right_ground_settings = Self::ground_config_to_settings(&right_ground_config);
            }

            println!("[Testbed] Configuration reloaded from {:?}", path);
        }
    }

    /// Create physics scene with simple cuboid ground collider (left scene, uses ground-1)
    fn create_cuboid_ground_scene(&self) -> PhysicsScene {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        let half_size = self.left_ground_settings.size / 2.0;
        let center = self.left_ground_settings.center;
        let ground_collider = ColliderBuilder::cuboid(half_size, half_size, half_size)
            .translation([center.x, center.y, center.z].into())
            .build();
        world.add_static_collider(ground_collider);

        // Create all falling objects from config
        let mut falling_objects = Vec::with_capacity(self.object_settings.len());
        let mut states = Vec::with_capacity(self.object_settings.len());

        for obj_settings in &self.object_settings {
            let mut falling_object = CubeObject::new_dynamic(&mut world, obj_settings.position, obj_settings.mass);
            // Apply rotation from config
            falling_object.set_rotation(&mut world, obj_settings.rotation);
            let falling_collider = create_box_collider(obj_settings.size);
            falling_object.attach_collider(&mut world, falling_collider);
            falling_objects.push(falling_object);
            states.push(PhysicsState::default());
        }

        PhysicsScene {
            world,
            falling_objects,
            ground_mesh_index: None,
            falling_mesh_indices: Vec::new(),
            ground_collider_type: "Cuboid",
            states,
        }
    }

    /// Create physics scene with terrain collider from Cube objects (right scene, uses ground-2)
    fn create_terrain_ground_scene(&self, ground_cube: &Rc<Cube<u8>>) -> PhysicsScene {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        let scale = self.right_ground_settings.size;
        let center = self.right_ground_settings.center;

        if self.debug_frames.is_some() {
            println!("[DEBUG] Creating terrain collider:");
            println!("  Ground cube: {:?}", ground_cube);
            println!("  Scale: {}", scale);
            println!("  Center: {:?}", center);
        }

        let terrain_collider = VoxelColliderBuilder::from_cube_scaled(ground_cube, 0, scale);

        if self.debug_frames.is_some() {
            // Print collider info
            if let Some(trimesh) = terrain_collider.shape().as_trimesh() {
                println!("  Terrain collider: {} vertices, {} triangles",
                    trimesh.vertices().len(), trimesh.indices().len());
            } else {
                println!("  WARNING: Terrain collider is not a trimesh!");
            }
        }

        let terrain_body = RigidBodyBuilder::fixed()
            .translation([center.x, center.y, center.z].into())
            .build();
        let terrain_body_handle = world.add_rigid_body(terrain_body);
        world.add_collider(terrain_collider, terrain_body_handle);

        // Create all falling objects from config
        let mut falling_objects = Vec::with_capacity(self.object_settings.len());
        let mut states = Vec::with_capacity(self.object_settings.len());

        for obj_settings in &self.object_settings {
            let mut falling_object = CubeObject::new_dynamic(&mut world, obj_settings.position, obj_settings.mass);
            // Apply rotation from config
            falling_object.set_rotation(&mut world, obj_settings.rotation);
            let falling_collider = create_box_collider(obj_settings.size);
            falling_object.attach_collider(&mut world, falling_collider);
            falling_objects.push(falling_object);
            states.push(PhysicsState::default());
        }

        PhysicsScene {
            world,
            falling_objects,
            ground_mesh_index: None,
            falling_mesh_indices: Vec::new(),
            ground_collider_type: "Terrain (Cube)",
            states,
        }
    }

    /// Reset physics scenes to initial state
    ///
    /// # Safety
    /// The GL context must be current when this is called.
    unsafe fn reset_scenes(&mut self, gl: &Context) {
        // Reload configuration from file if available
        #[cfg(feature = "steel")]
        self.reload_config();

        // Create ground cubes with their respective materials
        let left_ground_cube = Rc::new(Cube::Solid(self.left_ground_settings.material));
        let right_ground_cube = Rc::new(Cube::Solid(self.right_ground_settings.material));
        self.left_ground_cube = Some(left_ground_cube);
        self.right_ground_cube = Some(right_ground_cube.clone());

        // Recreate physics scenes
        self.left_scene = Some(self.create_cuboid_ground_scene());
        self.right_scene = Some(self.create_terrain_ground_scene(&right_ground_cube));

        // Re-upload meshes
        self.upload_meshes(gl);
    }

    /// Upload meshes for rendering
    ///
    /// # Safety
    /// The GL context must be current when this is called.
    unsafe fn upload_meshes(&mut self, gl: &Context) {
        // Create ground cubes with configured material colors (separate for left and right)
        let left_ground_cube = Rc::new(Cube::Solid(self.left_ground_settings.material));
        let right_ground_cube = Rc::new(Cube::Solid(self.right_ground_settings.material));
        self.left_ground_cube = Some(left_ground_cube.clone());
        self.right_ground_cube = Some(right_ground_cube.clone());

        // Upload left ground mesh
        match self.mesh_renderer.upload_mesh(gl, &left_ground_cube, 1) {
            Ok(idx) => {
                if let Some(scene) = &mut self.left_scene {
                    scene.ground_mesh_index = Some(idx);
                }
            }
            Err(e) => eprintln!("Failed to upload left ground mesh: {}", e),
        }

        // Upload right ground mesh
        match self.mesh_renderer.upload_mesh(gl, &right_ground_cube, 1) {
            Ok(idx) => {
                if let Some(scene) = &mut self.right_scene {
                    scene.ground_mesh_index = Some(idx);
                }
            }
            Err(e) => eprintln!("Failed to upload right ground mesh: {}", e),
        }

        // Upload falling cube meshes for each object (for left scene)
        if let Some(scene) = &mut self.left_scene {
            scene.falling_mesh_indices.clear();
            for obj_settings in &self.object_settings {
                let falling_cube = Rc::new(Cube::Solid(obj_settings.material));
                match self.mesh_renderer.upload_mesh(gl, &falling_cube, 1) {
                    Ok(idx) => scene.falling_mesh_indices.push(idx),
                    Err(e) => eprintln!("Failed to upload falling mesh: {}", e),
                }
            }
        }

        // Upload falling cube meshes for each object (for right scene)
        if let Some(scene) = &mut self.right_scene {
            scene.falling_mesh_indices.clear();
            for obj_settings in &self.object_settings {
                let falling_cube = Rc::new(Cube::Solid(obj_settings.material));
                match self.mesh_renderer.upload_mesh(gl, &falling_cube, 1) {
                    Ok(idx) => scene.falling_mesh_indices.push(idx),
                    Err(e) => eprintln!("Failed to upload falling mesh: {}", e),
                }
            }
        }

        // Store first falling cube for backwards compatibility (if any)
        if let Some(first_obj) = self.object_settings.first() {
            self.falling_cube = Some(Rc::new(Cube::Solid(first_obj.material)));
        }
    }

    /// Step physics for both scenes
    fn step_physics(&mut self) {
        let now = Instant::now();
        let elapsed = (now - self.last_physics_update).as_secs_f32();

        // Fixed timestep physics
        if elapsed >= self.physics_dt {
            self.last_physics_update = now;

            if let Some(scene) = &mut self.left_scene {
                scene.world.step(self.physics_dt);

                // Update physics state for all objects
                for (i, falling_object) in scene.falling_objects.iter().enumerate() {
                    let pos = falling_object.position(&scene.world);
                    let vel = falling_object.velocity(&scene.world);
                    let is_on_ground = vel.y.abs() < 0.1 && pos.y < 1.0;

                    if i < scene.states.len() {
                        scene.states[i] = PhysicsState {
                            falling_position: pos,
                            falling_velocity: vel,
                            is_on_ground,
                        };
                    }
                }
            }
            if let Some(scene) = &mut self.right_scene {
                scene.world.step(self.physics_dt);

                // Update physics state for all objects
                for (i, falling_object) in scene.falling_objects.iter().enumerate() {
                    let pos = falling_object.position(&scene.world);
                    let vel = falling_object.velocity(&scene.world);
                    let is_on_ground = vel.y.abs() < 0.1 && pos.y < 1.0;

                    if i < scene.states.len() {
                        scene.states[i] = PhysicsState {
                            falling_position: pos,
                            falling_velocity: vel,
                            is_on_ground,
                        };
                    }
                }
            }
        }
    }

    /// Print debug information about physics state
    fn print_debug_info(&self) {
        if self.debug_frames.is_none() {
            return;
        }

        let num_objects = self.object_settings.len();

        for obj_idx in 0..num_objects {
            let left_pos = self
                .left_scene
                .as_ref()
                .and_then(|s| s.falling_objects.get(obj_idx).map(|o| o.position(&s.world)))
                .unwrap_or(Vec3::ZERO);

            let right_pos = self
                .right_scene
                .as_ref()
                .and_then(|s| s.falling_objects.get(obj_idx).map(|o| o.position(&s.world)))
                .unwrap_or(Vec3::ZERO);

            let left_vel = self
                .left_scene
                .as_ref()
                .and_then(|s| s.falling_objects.get(obj_idx).map(|o| o.velocity(&s.world)))
                .unwrap_or(Vec3::ZERO);

            let right_vel = self
                .right_scene
                .as_ref()
                .and_then(|s| s.falling_objects.get(obj_idx).map(|o| o.velocity(&s.world)))
                .unwrap_or(Vec3::ZERO);

            let left_rot = self
                .left_scene
                .as_ref()
                .and_then(|s| s.falling_objects.get(obj_idx).map(|o| o.rotation(&s.world)))
                .unwrap_or(Quat::IDENTITY);

            let right_rot = self
                .right_scene
                .as_ref()
                .and_then(|s| s.falling_objects.get(obj_idx).map(|o| o.rotation(&s.world)))
                .unwrap_or(Quat::IDENTITY);

            let pos_diff = left_pos - right_pos;
            let vel_diff = left_vel - right_vel;

            println!(
                "Frame {} Obj[{}]: Left(Cuboid) pos={:.4},{:.4},{:.4} rot=({:.4},{:.4},{:.4},{:.4}) vel={:.4},{:.4},{:.4}",
                self.frame_count, obj_idx,
                left_pos.x, left_pos.y, left_pos.z,
                left_rot.x, left_rot.y, left_rot.z, left_rot.w,
                left_vel.x, left_vel.y, left_vel.z,
            );
            println!(
                "               Right(Terrain) pos={:.4},{:.4},{:.4} rot=({:.4},{:.4},{:.4},{:.4}) vel={:.4},{:.4},{:.4}",
                right_pos.x, right_pos.y, right_pos.z,
                right_rot.x, right_rot.y, right_rot.z, right_rot.w,
                right_vel.x, right_vel.y, right_vel.z,
            );

            if pos_diff.length() > 0.001 || vel_diff.length() > 0.001 {
                println!(
                    "  DIFF: pos_delta={:.6},{:.6},{:.6} vel_delta={:.6},{:.6},{:.6}",
                    pos_diff.x, pos_diff.y, pos_diff.z, vel_diff.x, vel_diff.y, vel_diff.z,
                );
            }
        }
    }

    /// Render a scene to the given viewport
    fn render_scene(
        gl: &Context,
        scene: &PhysicsScene,
        mesh_renderer: &MeshRenderer,
        camera: &Camera,
        camera_target: Vec3,
        viewport_x: i32,
        viewport_y: i32,
        viewport_width: i32,
        viewport_height: i32,
        x_offset: f32,
        ground_size: f32,
        ground_center: Vec3,
        object_sizes: &[Vec3],
    ) {
        unsafe {
            // Set viewport and scissor
            gl.viewport(viewport_x, viewport_y, viewport_width, viewport_height);
            gl.scissor(viewport_x, viewport_y, viewport_width, viewport_height);
            gl.enable(SCISSOR_TEST);

            // Clear this viewport with dark background
            gl.clear_color(0.12, 0.12, 0.18, 1.0);
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
        }

        // Create camera for this scene with offset position
        let offset_position = camera.position + Vec3::new(x_offset, 0.0, 0.0);
        let offset_target = camera_target + Vec3::new(x_offset, 0.0, 0.0);
        let scene_camera = Camera::look_at(offset_position, offset_target, Vec3::Y);

        if let Some(ground_idx) = scene.ground_mesh_index {
            let ground_pos = ground_center + Vec3::new(x_offset, 0.0, 0.0);
            unsafe {
                mesh_renderer.render_mesh_with_options(
                    gl,
                    ground_idx,
                    ground_pos,
                    Quat::IDENTITY,
                    ground_size,
                    Vec3::ONE,
                    false,
                    &scene_camera,
                    viewport_width,
                    viewport_height,
                );
            }
        }

        // Render all falling cubes with configured sizes and collision wireframes
        for (i, falling_object) in scene.falling_objects.iter().enumerate() {
            if let Some(&falling_idx) = scene.falling_mesh_indices.get(i) {
                let falling_pos = falling_object.position(&scene.world) + Vec3::new(x_offset, 0.0, 0.0);
                let falling_rot = falling_object.rotation(&scene.world);
                // Use max dimension * 2 as uniform scale (size is half-extents)
                let object_size = object_sizes.get(i).copied().unwrap_or(Vec3::splat(0.4));
                let render_scale = object_size.max_element() * 2.0;
                unsafe {
                    mesh_renderer.render_mesh_with_scale(
                        gl,
                        falling_idx,
                        falling_pos,
                        falling_rot,
                        render_scale,
                        &scene_camera,
                        viewport_width,
                        viewport_height,
                    );
                }

                // Render debug wireframe based on collision state
                let is_colliding = falling_object
                    .collider_handle()
                    .map(|h| scene.world.is_colliding(h))
                    .unwrap_or(false);

                // Green when no collision, red when colliding
                let wireframe_color = if is_colliding {
                    [1.0, 0.2, 0.2] // Red
                } else {
                    [0.2, 1.0, 0.2] // Green
                };

                // Calculate AABB for wireframe (object_size is half-extents)
                let aabb_half = object_size;
                let aabb_min = falling_pos - aabb_half;
                let aabb_max = falling_pos + aabb_half;

                unsafe {
                    mesh_renderer.render_aabb_wireframe(
                        gl,
                        aabb_min,
                        aabb_max,
                        wireframe_color,
                        &scene_camera,
                        viewport_width,
                        viewport_height,
                    );
                }
            }
        }

        unsafe {
            gl.disable(SCISSOR_TEST);
        }
    }

    fn check_debug_exit(&mut self) {
        if let Some(max_frames) = self.debug_frames {
            if self.frame_count >= max_frames {
                println!("\n[Testbed] Reached {} frames, exiting", max_frames);

                // Print final comparison for all objects
                if let (Some(left), Some(right)) = (&self.left_scene, &self.right_scene) {
                    println!("\n=== Final Comparison ===");

                    let mut has_significant_diff = false;
                    for (i, (left_obj, right_obj)) in left.falling_objects.iter().zip(right.falling_objects.iter()).enumerate() {
                        let left_pos = left_obj.position(&left.world);
                        let right_pos = right_obj.position(&right.world);
                        let diff = left_pos - right_pos;

                        println!("Object[{}]:", i);
                        println!("  Left (Cuboid):   Y = {:.6}", left_pos.y);
                        println!("  Right (Terrain): Y = {:.6}", right_pos.y);
                        println!("  Difference:      Y = {:.6}", diff.y);

                        if diff.length() > 0.01 {
                            has_significant_diff = true;
                        }
                    }

                    if has_significant_diff {
                        println!("\nWARNING: Significant physics difference detected!");
                    } else {
                        println!("\nPhysics behavior is consistent between both collider types.");
                    }
                }

                std::process::exit(0);
            }
        }
    }

    /// Handle orbit camera input from egui response
    fn handle_orbit_input(&mut self, input: &InputState) {
        // Handle camera orbit with left mouse drag
        if input.mouse_buttons.left && input.mouse_delta.length() > 0.0 {
            let delta = input.mouse_delta * self.orbit_controller.config.mouse_sensitivity;
            self.orbit_controller.rotate(delta.x, delta.y, &mut self.camera);
        }

        // Handle zoom with scroll
        if input.scroll_delta.y.abs() > 0.0 {
            self.orbit_controller.zoom(input.scroll_delta.y * self.orbit_controller.config.zoom_sensitivity, &mut self.camera);
        }
    }
}

impl App for PhysicsTestbed {
    fn init(&mut self, ctx: &FrameContext) {
        println!("[Testbed] Initializing physics testbed");

        // Initialize mesh renderer
        if let Err(e) = unsafe { self.mesh_renderer.init_gl(ctx.gl) } {
            eprintln!("[Testbed] Failed to initialize mesh renderer: {}", e);
        }

        // Create ground cubes with their respective materials from config
        let left_ground_cube = Rc::new(Cube::Solid(self.left_ground_settings.material));
        let right_ground_cube = Rc::new(Cube::Solid(self.right_ground_settings.material));
        self.left_ground_cube = Some(left_ground_cube);
        self.right_ground_cube = Some(right_ground_cube.clone());

        // Create physics scenes
        self.left_scene = Some(self.create_cuboid_ground_scene());
        self.right_scene = Some(self.create_terrain_ground_scene(&right_ground_cube));

        // Upload meshes
        unsafe { self.upload_meshes(ctx.gl) };

        self.last_physics_update = Instant::now();

        println!("[Testbed] Physics scenes initialized");
        println!("  Left:  Cuboid ground (ground-2), size={}", self.left_ground_settings.size);
        println!("  Right: Terrain ground (ground-1), size={}", self.right_ground_settings.size);
        if let Some(frames) = self.debug_frames {
            println!("  Debug mode: running {} frames", frames);
        }
    }

    fn shutdown(&mut self, ctx: &FrameContext) {
        println!("[Testbed] Cleaning up");
        unsafe { self.mesh_renderer.destroy_gl(ctx.gl) };
    }

    fn update(&mut self, ctx: &FrameContext, input: &InputState) {
        // Check debug frame limit (exits if reached)
        self.check_debug_exit();

        // Handle orbit camera
        self.handle_orbit_input(input);

        // Step physics
        self.step_physics();

        // Print debug info
        self.print_debug_info();

        // Handle reset after the frame
        if self.reset_requested {
            self.reset_requested = false;
            unsafe { self.reset_scenes(ctx.gl) };
        }

        self.frame_count += 1;
    }

    fn render(&mut self, ctx: &FrameContext) {
        let width = ctx.size.0 as i32;
        let height = ctx.size.1 as i32;

        // Reserve space for top bar
        let top_bar_height = 40;
        let label_height = 25;
        let render_height = height - top_bar_height - label_height;
        let half_width = width / 2;

        // Clear the entire window first
        unsafe {
            ctx.gl.viewport(0, 0, width, height);
            ctx.gl.clear_color(0.1, 0.1, 0.1, 1.0);
            ctx.gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
        }

        // Get camera target for consistent rendering
        let camera_target = self.orbit_controller.target;

        // Collect object sizes for rendering
        let object_sizes: Vec<Vec3> = self.object_settings.iter().map(|s| s.size).collect();

        // Render left scene (cuboid ground, uses ground-1)
        if let Some(scene) = &self.left_scene {
            Self::render_scene(
                ctx.gl,
                scene,
                &self.mesh_renderer,
                &self.camera,
                camera_target,
                0,
                0,
                half_width,
                render_height,
                -6.0,
                self.left_ground_settings.size,
                self.left_ground_settings.center,
                &object_sizes,
            );
        }

        // Render right scene (terrain ground, uses ground-2)
        if let Some(scene) = &self.right_scene {
            Self::render_scene(
                ctx.gl,
                scene,
                &self.mesh_renderer,
                &self.camera,
                camera_target,
                half_width,
                0,
                half_width,
                render_height,
                6.0,
                self.right_ground_settings.size,
                self.right_ground_settings.center,
                &object_sizes,
            );
        }

        // Draw divider line between viewports
        unsafe {
            ctx.gl.viewport(0, 0, width, height);
            ctx.gl.scissor(half_width - 1, 0, 2, render_height);
            ctx.gl.enable(SCISSOR_TEST);
            ctx.gl.clear_color(0.4, 0.4, 0.4, 1.0);
            ctx.gl.clear(COLOR_BUFFER_BIT);
            ctx.gl.disable(SCISSOR_TEST);
        }
    }

    fn ui(&mut self, _ctx: &FrameContext, egui_ctx: &egui::Context) {
        // Show first object state for status display (could be expanded to show all)
        let left_state_text = self
            .left_scene
            .as_ref()
            .and_then(|s| s.states.first().map(|state| format!("Cuboid: {}", state.format_compact())))
            .unwrap_or_else(|| "Cuboid: N/A".to_string());
        let right_state_text = self
            .right_scene
            .as_ref()
            .and_then(|s| s.states.first().map(|state| format!("Terrain: {}", state.format_compact())))
            .unwrap_or_else(|| "Terrain: N/A".to_string());
        let frame_count = self.frame_count;
        let label_height_f = 25.0;

        // Top bar with dropdown
        egui::TopBottomPanel::top("top_panel").show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Physics Testbed");
                ui.separator();
                ui.label("Setup: Single Falling Cube");
                ui.separator();

                if ui.button("Reset").clicked() {
                    self.reset_requested = true;
                }

                ui.separator();
                ui.label(format!("Frame: {}", frame_count));
            });
        });

        // Physics state labels above viewports
        egui::TopBottomPanel::bottom("state_panel")
            .frame(egui::Frame::NONE.fill(egui::Color32::from_gray(30)))
            .show(egui_ctx, |ui| {
                ui.set_height(label_height_f);
                ui.horizontal(|ui| {
                    let available_width = ui.available_width();
                    let half = available_width / 2.0;

                    // Left scene state
                    ui.allocate_ui_with_layout(
                        egui::vec2(half - 5.0, label_height_f),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.label(
                                egui::RichText::new(&left_state_text)
                                    .monospace()
                                    .color(egui::Color32::LIGHT_GREEN),
                            );
                        },
                    );

                    ui.separator();

                    // Right scene state
                    ui.allocate_ui_with_layout(
                        egui::vec2(half - 5.0, label_height_f),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.label(
                                egui::RichText::new(&right_state_text)
                                    .monospace()
                                    .color(egui::Color32::LIGHT_BLUE),
                            );
                        },
                    );
                });
            });

        // Central panel for camera control hint
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(egui_ctx, |ui| {
                let rect = ui.available_rect_before_wrap();

                // Show hint at bottom
                let hint_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.min.x + 5.0, rect.max.y - 20.0),
                    egui::vec2(300.0, 20.0),
                );
                ui.painter().text(
                    hint_rect.min,
                    egui::Align2::LEFT_TOP,
                    "(Drag: orbit, Scroll: zoom)",
                    egui::FontId::proportional(12.0),
                    egui::Color32::from_gray(120),
                );
            });
    }
}

/// Export the create_app function for dynamic loading (optional)
///
/// Note: This uses `dyn App` which isn't strictly FFI-safe, but this is only used
/// for hot-reload between Rust code, not for interop with other languages.
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn create_app() -> *mut dyn App {
    Box::into_raw(Box::new(PhysicsTestbed::new()))
}
