# Design: Proto-GL Physics Viewer

## Architecture Overview

The proto-gl viewer is a lightweight native application for validating voxel physics without Bevy's build overhead. It combines the GL rendering stack from `crates/renderer` with physics simulation from `crates/physics`, and references patterns from `crates/proto` for physics setup and object spawning.

**Crate usage:**
- `cube` - Voxel data structures, CSM parsing, mesh generation
- `crossworld-physics` - Rapier3D wrapper, voxel collider builder
- `renderer` - OpenGL rendering, GlCubeTracer, egui integration
- `proto` - Reference for code examples (physics setup, spawning patterns, collision handling)

```
Proto-GL Application
├── Main Loop (winit event loop)
│   ├── Window/GL Context (glutin)
│   ├── egui Integration (egui-glow)
│   └── Physics Simulation (Rapier3D)
├── Rendering
│   ├── GlCubeTracer (voxel rendering)
│   ├── egui UI (controls + info)
│   └── Camera Controls (orbit)
└── Physics World
    ├── Static World (VoxelColliderBuilder)
    ├── Dynamic CubeObjects (falling cubes)
    └── Collision Detection (Rapier3D)
```

## Component Design

### CubeObject Component

```rust
pub struct CubeObject {
    /// Voxel data
    pub cube: Rc<Cube<u8>>,

    /// Rapier rigid body handle
    pub body_handle: RigidBodyHandle,

    /// Rapier collider handle
    pub collider_handle: ColliderHandle,

    /// Model source (for identification)
    pub model_name: String,

    /// Octree depth for rendering
    pub depth: u32,
}
```

### ProtoGlConfig

```rust
#[derive(Debug, Deserialize)]
pub struct ProtoGlConfig {
    pub world: WorldConfig,
    pub physics: PhysicsConfig,
    pub spawning: SpawningConfig,
    pub rendering: RenderConfig,
}

#[derive(Debug, Deserialize)]
pub struct WorldConfig {
    pub macro_depth: u32,
    pub micro_depth: u32,
    pub border_depth: u32,
    pub border_materials: [u8; 4],
    pub root_cube: String,  // CSM format
}

#[derive(Debug, Deserialize)]
pub struct PhysicsConfig {
    pub gravity: f32,
    pub timestep: f32,
}

#[derive(Debug, Deserialize)]
pub struct SpawningConfig {
    pub spawn_count: u32,
    pub models_path: String,
    pub min_height: f32,
    pub max_height: f32,
    pub spawn_radius: f32,
}

#[derive(Debug, Deserialize)]
pub struct RenderConfig {
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub camera_distance: f32,
}
```

## Rendering Pipeline

### Integration with crates/renderer

Reuse the established GL rendering patterns:

```rust
use renderer::{GlCubeTracer, Renderer};

pub struct ProtoGlRenderer {
    gl_tracer: GlCubeTracer,
    camera_yaw: f32,
    camera_pitch: f32,
    camera_distance: f32,
}

impl ProtoGlRenderer {
    pub fn render_scene(
        &mut self,
        world_cube: &Cube<u8>,
        objects: &[CubeObject],
        rigid_bodies: &RigidBodySet,
    ) {
        // Render static world
        self.gl_tracer.render(world_cube, depth, cam_transform);

        // Render dynamic objects at their physics positions
        for obj in objects {
            let rb = &rigid_bodies[obj.body_handle];
            let position = rb.translation();
            let rotation = rb.rotation();

            // Transform cube to world space and render
            self.gl_tracer.render_transformed(
                &obj.cube,
                obj.depth,
                position,
                rotation,
            );
        }
    }
}
```

### Camera System

Simplified orbit camera (no free-fly or follow mode):

```rust
pub struct OrbitCamera {
    pub focus: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl OrbitCamera {
    pub fn view_matrix(&self) -> Mat4 {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();

        Mat4::look_at_rh(
            self.focus + Vec3::new(x, y, z),
            self.focus,
            Vec3::Y,
        )
    }

    pub fn handle_mouse_drag(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw -= delta_x * 0.003;
        self.pitch -= delta_y * 0.003;
        self.pitch = self.pitch.clamp(-1.5, 1.5);
    }

    pub fn handle_scroll(&mut self, delta: f32) {
        self.distance -= delta;
        self.distance = self.distance.clamp(5.0, 100.0);
    }
}
```

## Physics Integration

### World Setup

```rust
fn setup_world_physics(
    config: &WorldConfig,
    rigid_bodies: &mut RigidBodySet,
    colliders: &mut ColliderSet,
) -> Cube<u8> {
    // Parse CSM
    let cube = cube::parse_csm(&config.root_cube)?.root;

    // Apply border layers
    let cube = if config.border_depth > 0 {
        add_border_layers(cube, config.border_depth, config.border_materials)
    } else {
        cube
    };

    // Create world collider
    let collider = VoxelColliderBuilder::from_cube(
        &Rc::new(cube.clone()),
        config.macro_depth + config.micro_depth + config.border_depth,
    );

    // Add to physics world
    let rb = RigidBodyBuilder::fixed().build();
    let rb_handle = rigid_bodies.insert(rb);
    colliders.insert_with_parent(collider, rb_handle, rigid_bodies);

    cube
}
```

### Dynamic Object Spawning

```rust
fn spawn_cube_objects(
    config: &SpawningConfig,
    rigid_bodies: &mut RigidBodySet,
    colliders: &mut ColliderSet,
) -> Vec<CubeObject> {
    let mut objects = Vec::new();
    let models = load_vox_models(&config.models_path);

    for i in 0..config.spawn_count {
        // Random position
        let x = rand::random::<f32>() * config.spawn_radius * 2.0 - config.spawn_radius;
        let y = rand::random::<f32>() * (config.max_height - config.min_height) + config.min_height;
        let z = rand::random::<f32>() * config.spawn_radius * 2.0 - config.spawn_radius;

        // Random model
        let model = &models[i % models.len()];

        // Create physics body
        let rb = RigidBodyBuilder::dynamic()
            .translation(vector![x, y, z])
            .build();
        let rb_handle = rigid_bodies.insert(rb);

        // Create collider
        let collider = VoxelColliderBuilder::from_cube(&model.cube, model.depth);
        let coll_handle = colliders.insert_with_parent(
            collider,
            rb_handle,
            rigid_bodies,
        );

        objects.push(CubeObject {
            cube: model.cube.clone(),
            body_handle: rb_handle,
            collider_handle: coll_handle,
            model_name: model.name.clone(),
            depth: model.depth,
        });
    }

    objects
}
```

## UI Design (egui)

### Control Panel

```rust
fn render_ui(
    ctx: &egui::Context,
    objects: &[CubeObject],
    fps: f32,
    config: &ProtoGlConfig,
) {
    egui::SidePanel::right("controls").show(ctx, |ui| {
        ui.heading("Proto-GL Viewer");

        ui.separator();
        ui.label(format!("FPS: {:.1}", fps));
        ui.label(format!("Objects: {}", objects.len()));

        ui.separator();
        ui.heading("Configuration");
        ui.label(format!("World depth: {}", config.world.macro_depth));
        ui.label(format!("Gravity: {:.2}", config.physics.gravity));
        ui.label(format!("Timestep: {:.4}", config.physics.timestep));

        if ui.button("Reset Scene").clicked() {
            // Reset physics simulation
        }
    });
}
```

## Configuration File

```toml
[world]
macro_depth = 3
micro_depth = 4
border_depth = 1
border_materials = [32, 32, 0, 0]
root_cube = ">a [5 5 4 9 5 5 0 0]"

[physics]
gravity = -9.81
timestep = 0.016666  # 60 Hz

[spawning]
spawn_count = 10
models_path = "assets/models/"
min_height = 10.0
max_height = 30.0
spawn_radius = 20.0

[rendering]
viewport_width = 800
viewport_height = 600
camera_distance = 30.0
```

## Main Loop Structure

```rust
fn main() -> Result<(), Box<dyn Error>> {
    let config = load_config()?;

    let event_loop = EventLoop::new()?;
    let mut app = ProtoGlApp::new(config);

    event_loop.run_app(&mut app)?;
    Ok(())
}

struct ProtoGlApp {
    // GL/Window state
    window: Option<Window>,
    gl: Option<Arc<Context>>,
    egui_ctx: Option<egui::Context>,
    painter: Option<egui_glow::Painter>,

    // Physics state
    physics_world: PhysicsWorld,
    objects: Vec<CubeObject>,
    world_cube: Cube<u8>,

    // Rendering state
    renderer: ProtoGlRenderer,
    camera: OrbitCamera,

    // Timing
    last_frame: Instant,
    accumulator: f32,

    // Config
    config: ProtoGlConfig,
}

impl ApplicationHandler for ProtoGlApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Setup GL context, egui, renderer
        // Initialize physics world
        // Spawn objects
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::RedrawRequested => {
                // Physics step (fixed timestep)
                self.physics_step();

                // Render scene
                self.render();

                // Render UI
                self.render_ui();

                // Swap buffers
                self.swap_buffers();

                // Request next frame
                self.window.request_redraw();
            }
            WindowEvent::MouseInput { .. } => { /* Camera controls */ }
            WindowEvent::CursorMoved { .. } => { /* Camera drag */ }
            WindowEvent::MouseWheel { .. } => { /* Camera zoom */ }
            _ => {}
        }
    }
}
```

## Performance Targets

- **Build time**: < 15 seconds (clean build)
- **Frame rate**: 60 FPS with 10 cubes
- **Memory**: < 50 MB with 20 cubes
- **Physics step**: < 2ms per step

## Testing Strategy

1. **Visual validation**: Cubes fall and collide correctly
2. **Performance tests**: Measure build time, FPS, memory
3. **Config loading**: Verify TOML parsing and defaults
4. **Physics accuracy**: Compare with Bevy proto results

## Future Extensions

Potential enhancements (not in initial scope):

1. **Collision visualization**: Render AABBs, contact points
2. **Time controls**: Pause, slow-motion, step-through
3. **Object spawning**: Runtime add/remove cubes
4. **Camera presets**: Top-down, side view, etc.
5. **Export**: Save screenshots or physics recordings
