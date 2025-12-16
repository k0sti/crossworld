use glam::{Mat4, Vec3};

/// Orbit camera for viewing the scene
pub struct OrbitCamera {
    pub focus: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub dragging: bool,
    pub last_mouse_pos: Option<(f32, f32)>,
}

impl OrbitCamera {
    pub fn new(distance: f32) -> Self {
        Self {
            // World cube is in [0, 1] space, focus on center
            focus: Vec3::splat(0.5),
            yaw: 0.0,
            pitch: 0.5,
            distance,
            dragging: false,
            last_mouse_pos: None,
        }
    }

    #[allow(dead_code)]
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

    pub fn position(&self) -> Vec3 {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        self.focus + Vec3::new(x, y, z)
    }

    pub fn rotation(&self) -> glam::Quat {
        // Calculate the direction from camera to focus
        let pos = self.position();
        let dir = (self.focus - pos).normalize();

        // Create a rotation that looks at the focus point
        glam::Quat::from_rotation_arc(Vec3::NEG_Z, dir)
    }

    pub fn handle_mouse_drag(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw -= delta_x * 0.003;
        self.pitch -= delta_y * 0.003;
        self.pitch = self.pitch.clamp(-1.5, 1.5);
    }

    pub fn handle_scroll(&mut self, delta: f32) {
        self.distance -= delta * 0.1;
        self.distance = self.distance.clamp(0.5, 10.0);
    }
}
