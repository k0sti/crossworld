//! Physics Component Testbed
//!
//! Compares physics behavior between 4 different collider strategies:
//! - Scene 1: Cuboid collider - Simple box collider from ground cube
//! - Scene 2: Mesh collider - Triangle mesh from Cube using VoxelColliderBuilder
//! - Scene 3: Terrain collider - VoxelTerrainCollider with lazy triangle generation
//! - Scene 4: Empty - Reserved for future implementation
//!
//! Scene configuration can be loaded from Lua files when the
//! `lua` feature is enabled. See `config/scene.lua` for an example.

#![allow(clippy::too_many_arguments)]

use app::{App, FrameContext, InputState};
use crossworld_physics::{
    create_box_collider,
    rapier3d::{
        self,
        prelude::{ColliderBuilder, RigidBodyBuilder},
    },
    terrain::VoxelTerrainCollider,
    CubeObject, PhysicsWorld, VoxelColliderBuilder,
};
use cube::{io::csm::parse_csm, Cube};
use glam::{Quat, Vec3};
use glow::{Context, HasContext, COLOR_BUFFER_BIT, DEPTH_BUFFER_BIT, SCISSOR_TEST};
use renderer::{Camera, MeshRenderer, OrbitController, OrbitControllerConfig};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

pub mod lua_scene;
pub mod ui;

use lua_scene::{CameraConfig, GroundConfig, TestbedConfig};

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

/// Collider type for each scene
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColliderType {
    /// Simple cuboid collider
    Cuboid,
    /// Triangle mesh from VoxelColliderBuilder
    Mesh,
    /// Terrain collider with lazy triangle generation
    Terrain,
    /// Empty scene (no collider)
    Empty,
}

impl ColliderType {
    pub fn label(&self) -> &'static str {
        match self {
            ColliderType::Cuboid => "Cuboid",
            ColliderType::Mesh => "Mesh",
            ColliderType::Terrain => "Terrain",
            ColliderType::Empty => "Empty",
        }
    }

    pub fn color(&self) -> egui::Color32 {
        match self {
            ColliderType::Cuboid => egui::Color32::LIGHT_GREEN,
            ColliderType::Mesh => egui::Color32::LIGHT_BLUE,
            ColliderType::Terrain => egui::Color32::LIGHT_YELLOW,
            ColliderType::Empty => egui::Color32::GRAY,
        }
    }
}

/// Physics testbed state for one scene
pub struct PhysicsScene {
    pub world: PhysicsWorld,
    pub falling_objects: Vec<CubeObject>,
    pub ground_mesh_index: Option<usize>,
    pub falling_mesh_indices: Vec<usize>,
    pub collider_type: ColliderType,
    pub states: Vec<PhysicsState>,
    /// Terrain collider for scene type Terrain (optional)
    pub terrain_collider: Option<VoxelTerrainCollider>,
}

/// Ground configuration for scene creation
#[derive(Clone)]
pub struct GroundSettings {
    /// Size of ground cube edge (default 8.0)
    pub size: f32,
    /// Material/color index for ground (used when csm_cube is None)
    pub material: u8,
    /// Center position of the ground (default: origin-centered with top at Y=0)
    pub center: Vec3,
    /// Optional CSM-loaded cube (takes precedence over material)
    pub csm_cube: Option<Rc<Cube<u8>>>,
}

impl Default for GroundSettings {
    fn default() -> Self {
        let size = 8.0;
        Self {
            size,
            material: 32,
            center: Vec3::new(0.0, -size / 2.0, 0.0),
            csm_cube: None,
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

    // Physics scenes (4 quadrants)
    scenes: [Option<PhysicsScene>; 4],

    // Cubes for rendering (one for each scene)
    ground_cubes: [Option<Rc<Cube<u8>>>; 4],
    #[allow(dead_code)]
    falling_cube: Option<Rc<Cube<u8>>>,

    // Ground configuration (shared for all scenes)
    ground_settings: GroundSettings,

    // Object configurations (multiple objects supported)
    object_settings: Vec<ObjectSettings>,

    // UI state
    reset_requested: bool,

    // Timing
    frame_count: u64,
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
        let camera_position = Vec3::new(15.0, 12.0, 15.0);

        // Configure orbit controller with appropriate limits
        let orbit_config = OrbitControllerConfig {
            mouse_sensitivity: 0.005,
            zoom_sensitivity: 0.5,
            min_distance: 2.0,
            max_distance: 50.0,
        };

        Self {
            mesh_renderer: MeshRenderer::new(),
            camera: Camera::look_at(camera_position, camera_target, Vec3::Y),
            orbit_controller: OrbitController::new(camera_target, orbit_config),
            scenes: [None, None, None, None],
            ground_cubes: [None, None, None, None],
            falling_cube: None,
            ground_settings: GroundSettings::default(),
            object_settings: vec![ObjectSettings::default()],
            reset_requested: false,
            frame_count: 0,
            physics_dt: 1.0 / 60.0,
            last_physics_update: Instant::now(),
        }
    }

    /// Load configuration from a Lua file
    ///
    /// Returns a new PhysicsTestbed configured from the Lua scene configuration.
    /// Falls back to default configuration if the file doesn't exist or fails to parse.
    pub fn from_config_file(path: &Path) -> Self {
        match Self::try_from_config_file(path) {
            Ok(testbed) => testbed,
            Err(e) => {
                eprintln!(
                    "[Testbed] Warning: Failed to load config from {:?}: {}",
                    path, e
                );
                eprintln!("[Testbed] Using default configuration");
                Self::new()
            }
        }
    }

    /// Try to load configuration from a Lua file
    pub fn try_from_config_file(path: &Path) -> Result<Self, String> {
        let mut config =
            TestbedConfig::new().map_err(|e| format!("Failed to create Lua config: {}", e))?;
        config.load_file(path)?;

        // Extract camera configuration
        let camera_config = config.extract_camera("scene_camera")?;

        // Extract single ground configuration (used for all scenes)
        let ground_config = config.extract_ground("ground")?;

        // Extract object configurations from scene_objects
        let objects = config.extract_objects("scene_objects")?;
        if objects.is_empty() {
            return Err("scene_objects must contain at least one object".to_string());
        }

        Self::from_config(path, &camera_config, &ground_config, &objects)
    }

    /// Convert a GroundConfig to GroundSettings (Lua version)
    fn ground_config_to_settings(
        ground_config: &GroundConfig,
        config_dir: Option<&Path>,
    ) -> GroundSettings {
        match ground_config {
            GroundConfig::SolidCube {
                material,
                size_shift,
                center,
            } => GroundSettings {
                size: (1 << size_shift) as f32,
                material: *material,
                center: center.to_vec3(),
                csm_cube: None,
            },
            GroundConfig::CsmFile {
                path,
                size_shift,
                center,
            } => {
                // Resolve path relative to config directory
                let csm_path = if let Some(dir) = config_dir {
                    dir.join(path)
                } else {
                    std::path::PathBuf::from(path)
                };

                // Try to load CSM file
                let csm_cube = match std::fs::read_to_string(&csm_path) {
                    Ok(content) => match parse_csm(&content) {
                        Ok(cube) => Some(Rc::new(cube)),
                        Err(e) => {
                            eprintln!("[Testbed] Failed to parse CSM file {:?}: {}", csm_path, e);
                            None
                        }
                    },
                    Err(e) => {
                        eprintln!("[Testbed] Failed to read CSM file {:?}: {}", csm_path, e);
                        None
                    }
                };

                GroundSettings {
                    size: (1 << size_shift) as f32,
                    material: 32, // Default material if CSM loading fails
                    center: center.to_vec3(),
                    csm_cube,
                }
            }
        }
    }

    /// Convert an ObjectConfig to ObjectSettings (Lua version)
    fn object_config_to_settings(object_config: &lua_scene::ObjectConfig) -> ObjectSettings {
        ObjectSettings {
            position: object_config.position.to_vec3(),
            rotation: object_config.rotation.to_quat(),
            size: object_config.size.to_vec3(),
            mass: object_config.mass,
            material: object_config.material,
        }
    }

    /// Create testbed from camera and ground configurations (Lua version)
    fn from_config(
        path: &Path,
        camera_config: &CameraConfig,
        ground_config: &GroundConfig,
        object_configs: &[lua_scene::ObjectConfig],
    ) -> Result<Self, String> {
        let camera_position = camera_config.position.to_vec3();
        let camera_target = camera_config.look_at.to_vec3();

        // Get config directory for resolving relative paths
        let config_dir = path.parent();

        // Convert ground config to settings
        let ground_settings = Self::ground_config_to_settings(ground_config, config_dir);

        // Convert all object configs to settings
        let object_settings: Vec<ObjectSettings> = object_configs
            .iter()
            .map(Self::object_config_to_settings)
            .collect();

        let orbit_config = OrbitControllerConfig {
            mouse_sensitivity: 0.005,
            zoom_sensitivity: 0.5,
            min_distance: 2.0,
            max_distance: 50.0,
        };

        Ok(Self {
            mesh_renderer: MeshRenderer::new(),
            camera: Camera::look_at(camera_position, camera_target, Vec3::Y),
            orbit_controller: OrbitController::new(camera_target, orbit_config),
            scenes: [None, None, None, None],
            ground_cubes: [None, None, None, None],
            falling_cube: None,
            ground_settings,
            object_settings,
            reset_requested: false,
            frame_count: 0,
            physics_dt: 1.0 / 60.0,
            last_physics_update: Instant::now(),
        })
    }

    /// Get the ground cube (from CSM or solid material)
    fn get_ground_cube(&self) -> Rc<Cube<u8>> {
        self.ground_settings
            .csm_cube
            .clone()
            .unwrap_or_else(|| Rc::new(Cube::Solid(self.ground_settings.material)))
    }

    /// Create falling objects for a scene
    fn create_falling_objects(
        &self,
        world: &mut PhysicsWorld,
    ) -> (Vec<CubeObject>, Vec<PhysicsState>) {
        let mut falling_objects = Vec::with_capacity(self.object_settings.len());
        let mut states = Vec::with_capacity(self.object_settings.len());

        for obj_settings in &self.object_settings {
            let mut falling_object =
                CubeObject::new_dynamic(world, obj_settings.position, obj_settings.mass);
            falling_object.set_rotation(world, obj_settings.rotation);
            let falling_collider = create_box_collider(obj_settings.size);
            falling_object.attach_collider(world, falling_collider);
            falling_objects.push(falling_object);
            states.push(PhysicsState::default());
        }

        (falling_objects, states)
    }

    /// Scene 1: Create physics scene with simple cuboid ground collider
    fn create_cuboid_scene(&self) -> PhysicsScene {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        let half_size = self.ground_settings.size / 2.0;
        let center = self.ground_settings.center;
        let ground_collider = ColliderBuilder::cuboid(half_size, half_size, half_size)
            .translation([center.x, center.y, center.z].into())
            .build();
        world.add_static_collider(ground_collider);

        let (falling_objects, states) = self.create_falling_objects(&mut world);

        PhysicsScene {
            world,
            falling_objects,
            ground_mesh_index: None,
            falling_mesh_indices: Vec::new(),
            collider_type: ColliderType::Cuboid,
            states,
            terrain_collider: None,
        }
    }

    /// Scene 2: Create physics scene with mesh collider from Cube (VoxelColliderBuilder)
    fn create_mesh_scene(&self, ground_cube: &Rc<Cube<u8>>) -> PhysicsScene {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        let scale = self.ground_settings.size;
        let center = self.ground_settings.center;

        let mesh_collider = VoxelColliderBuilder::from_cube_scaled(ground_cube, 0, scale);

        let ground_body = RigidBodyBuilder::fixed()
            .translation([center.x, center.y, center.z].into())
            .build();
        let ground_body_handle = world.add_rigid_body(ground_body);
        world.add_collider(mesh_collider, ground_body_handle);

        let (falling_objects, states) = self.create_falling_objects(&mut world);

        PhysicsScene {
            world,
            falling_objects,
            ground_mesh_index: None,
            falling_mesh_indices: Vec::new(),
            collider_type: ColliderType::Mesh,
            states,
            terrain_collider: None,
        }
    }

    /// Scene 3: Create physics scene with terrain collider (VoxelTerrainCollider)
    fn create_terrain_scene(&self, ground_cube: &Rc<Cube<u8>>) -> PhysicsScene {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        let scale = self.ground_settings.size;
        let center = self.ground_settings.center;

        // Create VoxelTerrainCollider
        #[allow(clippy::arc_with_non_send_sync)]
        let arc_cube = Arc::new((**ground_cube).clone());
        let mut terrain_collider = VoxelTerrainCollider::new(
            arc_cube,
            scale,
            2,            // region_depth
            [1, 1, 0, 0], // border_materials
        );

        // Update the triangle BVH for the active area
        let half_world = scale / 2.0;
        let active_aabb = rapier3d::parry::bounding_volume::Aabb::new(
            [-half_world, -half_world, -half_world].into(),
            [half_world, half_world, half_world].into(),
        );
        terrain_collider.update_triangle_bvh(&active_aabb);

        // Create a trimesh collider from the terrain if we have triangles
        if let Some(shape) = terrain_collider.to_trimesh() {
            let ground_collider = ColliderBuilder::new(shape)
                .translation([center.x, center.y, center.z].into())
                .build();
            let ground_body = RigidBodyBuilder::fixed()
                .translation([center.x, center.y, center.z].into())
                .build();
            let ground_body_handle = world.add_rigid_body(ground_body);
            world.add_collider(ground_collider, ground_body_handle);
        }

        let (falling_objects, states) = self.create_falling_objects(&mut world);

        PhysicsScene {
            world,
            falling_objects,
            ground_mesh_index: None,
            falling_mesh_indices: Vec::new(),
            collider_type: ColliderType::Terrain,
            states,
            terrain_collider: Some(terrain_collider),
        }
    }

    /// Scene 4: Create empty physics scene (placeholder for future)
    fn create_empty_scene(&self) -> PhysicsScene {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let (falling_objects, states) = self.create_falling_objects(&mut world);

        PhysicsScene {
            world,
            falling_objects,
            ground_mesh_index: None,
            falling_mesh_indices: Vec::new(),
            collider_type: ColliderType::Empty,
            states,
            terrain_collider: None,
        }
    }

    /// Reset physics scenes to initial state
    ///
    /// # Safety
    /// The GL context must be current when this is called.
    unsafe fn reset_scenes(&mut self, gl: &Context) {
        // Get ground cube (from CSM or solid material)
        let ground_cube = self.get_ground_cube();

        // Create all 4 ground cubes (same cube for all scenes)
        for i in 0..4 {
            self.ground_cubes[i] = Some(ground_cube.clone());
        }

        // Recreate all 4 physics scenes
        self.scenes[0] = Some(self.create_cuboid_scene());
        self.scenes[1] = Some(self.create_mesh_scene(&ground_cube));
        self.scenes[2] = Some(self.create_terrain_scene(&ground_cube));
        self.scenes[3] = Some(self.create_empty_scene());

        // Re-upload meshes
        self.upload_meshes(gl);
    }

    /// Upload meshes for rendering
    ///
    /// # Safety
    /// The GL context must be current when this is called.
    unsafe fn upload_meshes(&mut self, gl: &Context) {
        // Get ground cube (from CSM or solid material)
        let ground_cube = self.get_ground_cube();

        // Upload ground mesh for all 4 scenes
        for i in 0..4 {
            self.ground_cubes[i] = Some(ground_cube.clone());

            match self.mesh_renderer.upload_mesh(gl, &ground_cube, 1) {
                Ok(idx) => {
                    if let Some(scene) = &mut self.scenes[i] {
                        scene.ground_mesh_index = Some(idx);
                    }
                }
                Err(e) => eprintln!("Failed to upload ground mesh for scene {}: {}", i, e),
            }
        }

        // Upload falling cube meshes for each scene
        for scene in self.scenes.iter_mut().flatten() {
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

    /// Step physics for all scenes
    fn step_physics(&mut self) {
        let now = Instant::now();
        let elapsed = (now - self.last_physics_update).as_secs_f32();

        // Fixed timestep physics
        if elapsed >= self.physics_dt {
            self.last_physics_update = now;

            for scene in self.scenes.iter_mut().flatten() {
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

    /// Render a scene to the given viewport
    fn render_scene(
        gl: &Context,
        scene: &PhysicsScene,
        mesh_renderer: &MeshRenderer,
        camera: &Camera,
        viewport_x: i32,
        viewport_y: i32,
        viewport_width: i32,
        viewport_height: i32,
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

        // Only render ground if not empty scene
        if scene.collider_type != ColliderType::Empty {
            if let Some(ground_idx) = scene.ground_mesh_index {
                unsafe {
                    mesh_renderer.render_mesh_with_options(
                        gl,
                        ground_idx,
                        ground_center,
                        Quat::IDENTITY,
                        Vec3::splat(ground_size),
                        Vec3::ONE,
                        false,
                        camera,
                        viewport_width,
                        viewport_height,
                    );
                }
            }
        }

        // Render all falling cubes with configured sizes and collision wireframes
        for (i, falling_object) in scene.falling_objects.iter().enumerate() {
            if let Some(&falling_idx) = scene.falling_mesh_indices.get(i) {
                let falling_pos = falling_object.position(&scene.world);
                let falling_rot = falling_object.rotation(&scene.world);
                let object_size = object_sizes.get(i).copied().unwrap_or(Vec3::splat(0.4));
                let normalized_size = object_size * 2.0;
                unsafe {
                    mesh_renderer.render_mesh_with_normalized_size(
                        gl,
                        falling_idx,
                        falling_pos,
                        falling_rot,
                        1.0,
                        normalized_size,
                        camera,
                        viewport_width,
                        viewport_height,
                    );
                }

                // Render debug wireframe based on collision state
                let is_colliding = falling_object
                    .collider_handle()
                    .map(|h| scene.world.is_colliding(h))
                    .unwrap_or(false);
                let is_sleeping = falling_object.is_sleeping(&scene.world);

                // Gray when sleeping, red when colliding, green when active
                let wireframe_color = if is_sleeping {
                    [0.5, 0.5, 0.5] // Gray
                } else if is_colliding {
                    [1.0, 0.2, 0.2] // Red
                } else {
                    [0.2, 1.0, 0.2] // Green
                };

                let box_size = object_size * 2.0;

                unsafe {
                    mesh_renderer.render_cubebox_wireframe_colored(
                        gl,
                        falling_pos,
                        falling_rot,
                        box_size,
                        1.0,
                        wireframe_color,
                        camera,
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

    /// Handle orbit camera input (right mouse button for orbit, scroll for zoom)
    fn handle_orbit_input(&mut self, input: &InputState) {
        // Handle camera orbit with RIGHT mouse drag
        if input.mouse_buttons.right && input.mouse_delta.length() > 0.0 {
            let delta = input.mouse_delta * self.orbit_controller.config.mouse_sensitivity;
            self.orbit_controller
                .rotate(delta.x, delta.y, &mut self.camera);
        }

        // Handle zoom with scroll
        if input.scroll_delta.y.abs() > 0.0 {
            self.orbit_controller.zoom(
                input.scroll_delta.y * self.orbit_controller.config.zoom_sensitivity,
                &mut self.camera,
            );
        }
    }
}

impl App for PhysicsTestbed {
    fn init(&mut self, ctx: &FrameContext) {
        println!("[Testbed] Initializing physics testbed with 4 scenes");

        // Initialize mesh renderer
        if let Err(e) = unsafe { self.mesh_renderer.init_gl(ctx.gl) } {
            eprintln!("[Testbed] Failed to initialize mesh renderer: {}", e);
        }

        // Get ground cube (from CSM or solid material)
        let ground_cube = self.get_ground_cube();

        // Create all 4 ground cubes (same cube for all scenes)
        for i in 0..4 {
            self.ground_cubes[i] = Some(ground_cube.clone());
        }

        // Create all 4 physics scenes
        self.scenes[0] = Some(self.create_cuboid_scene());
        self.scenes[1] = Some(self.create_mesh_scene(&ground_cube));
        self.scenes[2] = Some(self.create_terrain_scene(&ground_cube));
        self.scenes[3] = Some(self.create_empty_scene());

        // Upload meshes
        unsafe { self.upload_meshes(ctx.gl) };

        self.last_physics_update = Instant::now();

        println!("[Testbed] Physics scenes initialized:");
        println!("  Scene 1: Cuboid collider");
        println!("  Scene 2: Mesh collider (VoxelColliderBuilder)");
        println!("  Scene 3: Terrain collider (VoxelTerrainCollider)");
        println!("  Scene 4: Empty (future implementation)");
        println!("  Ground size: {}", self.ground_settings.size);
    }

    fn shutdown(&mut self, ctx: &FrameContext) {
        println!("[Testbed] Cleaning up");
        unsafe { self.mesh_renderer.destroy_gl(ctx.gl) };
    }

    fn update(&mut self, ctx: &FrameContext, input: &InputState) {
        // Handle orbit camera (right mouse button)
        self.handle_orbit_input(input);

        // Step physics
        self.step_physics();

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

        // Reserve space for top bar only (status is now overlay in each view)
        let top_bar_height = ui::TOP_BAR_HEIGHT as i32;
        let render_height = height - top_bar_height;
        let half_width = width / 2;
        let half_height = render_height / 2;

        // Clear the entire window first
        unsafe {
            ctx.gl.viewport(0, 0, width, height);
            ctx.gl.clear_color(0.1, 0.1, 0.1, 1.0);
            ctx.gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
        }

        // Collect object sizes for rendering
        let object_sizes: Vec<Vec3> = self.object_settings.iter().map(|s| s.size).collect();

        // Quadrant layout (0=top-left, 1=top-right, 2=bottom-left, 3=bottom-right)
        let quadrants = [
            (0, half_height),          // Scene 0: Top-left (Cuboid)
            (half_width, half_height), // Scene 1: Top-right (Mesh)
            (0, 0),                    // Scene 2: Bottom-left (Terrain)
            (half_width, 0),           // Scene 3: Bottom-right (Empty)
        ];

        // Render all 4 scenes with shared camera
        for (i, scene_opt) in self.scenes.iter().enumerate() {
            if let Some(scene) = scene_opt {
                let (vp_x, vp_y) = quadrants[i];
                Self::render_scene(
                    ctx.gl,
                    scene,
                    &self.mesh_renderer,
                    &self.camera,
                    vp_x,
                    vp_y,
                    half_width,
                    half_height,
                    self.ground_settings.size,
                    self.ground_settings.center,
                    &object_sizes,
                );
            }
        }

        // Draw divider lines between quadrants
        unsafe {
            ctx.gl.viewport(0, 0, width, height);

            // Vertical divider
            ctx.gl.scissor(half_width - 1, 0, 2, render_height);
            ctx.gl.enable(SCISSOR_TEST);
            ctx.gl.clear_color(0.4, 0.4, 0.4, 1.0);
            ctx.gl.clear(COLOR_BUFFER_BIT);

            // Horizontal divider
            ctx.gl.scissor(0, half_height - 1, width, 2);
            ctx.gl.clear(COLOR_BUFFER_BIT);

            ctx.gl.disable(SCISSOR_TEST);
        }
    }

    fn ui(&mut self, ctx: &FrameContext, egui_ctx: &egui::Context) {
        let frame_count = self.frame_count;

        // Top bar
        egui::TopBottomPanel::top("top_panel").show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Physics Testbed");
                ui.separator();
                ui.label("4-Scene Collider Comparison");
                ui.separator();

                if ui.button("Reset").clicked() {
                    self.reset_requested = true;
                }

                ui.separator();
                ui.label(format!("Frame: {}", frame_count));
            });
        });

        // Central panel for scene overlays (titles and status)
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(egui_ctx, |ui| {
                let painter = ui.painter();
                let width = ctx.size.0 as f32;
                let height = ctx.size.1 as f32;

                // Calculate quadrant rects using UI module
                let quadrant_rects = ui::calculate_quadrant_rects(width, height);

                // Render all scene overlays (titles + status)
                ui::render_scene_overlays(painter, &self.scenes, &quadrant_rects);
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
