use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Result of a raycast operation
#[derive(Debug, Clone, Copy)]
pub struct RaycastResult {
    /// World position where ray hit voxel surface
    pub hit_position: Vec3,
    /// Normal vector of the face that was hit (axis-aligned)
    pub face_normal: Vec3,
    /// Distance from ray origin to hit point
    pub distance: f32,
    /// Voxel coordinate that was hit (integer coordinates)
    pub voxel_coord: IVec3,
}

impl RaycastResult {
    /// Calculate the position for placing a voxel (hit position + face normal)
    pub fn placement_position(&self) -> Vec3 {
        self.hit_position + self.face_normal
    }
}

/// Resource storing the current raycast result
#[derive(Resource, Default)]
pub struct EditorRaycast {
    pub result: Option<RaycastResult>,
}

/// System that performs raycasting from camera through mouse cursor
pub fn update_raycast(
    mut raycast: ResMut<EditorRaycast>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    // Get primary window
    let Ok(window) = window_query.single() else {
        return;
    };

    // Get cursor position
    let Some(cursor_pos) = window.cursor_position() else {
        raycast.result = None;
        return;
    };

    // Get camera
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    // Convert cursor position to world-space ray
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        raycast.result = None;
        return;
    };

    // Perform raycast against voxel world
    // For now, we'll do a simple plane intersection at y=0 as a placeholder
    // TODO: Replace with actual voxel raycast using cube crate
    if let Some(result) = raycast_plane(ray.origin, *ray.direction, Vec3::ZERO, Vec3::Y) {
        raycast.result = Some(result);
    } else {
        raycast.result = None;
    }
}

/// Temporary raycast against a plane (will be replaced with voxel raycast)
fn raycast_plane(origin: Vec3, direction: Vec3, plane_point: Vec3, plane_normal: Vec3) -> Option<RaycastResult> {
    let denom = plane_normal.dot(direction);

    // Check if ray is parallel to plane
    if denom.abs() < 1e-6 {
        return None;
    }

    let t = (plane_point - origin).dot(plane_normal) / denom;

    // Check if intersection is behind ray origin
    if t < 0.0 {
        return None;
    }

    let hit_position = origin + direction * t;

    // Calculate voxel coordinate (floor to integer grid)
    let voxel_coord = IVec3::new(
        hit_position.x.floor() as i32,
        0, // Plane intersection is always at y=0
        hit_position.z.floor() as i32,
    );

    Some(RaycastResult {
        hit_position,
        face_normal: if denom < 0.0 { plane_normal } else { -plane_normal },
        distance: t,
        voxel_coord,
    })
}

/// Plugin for raycast system
pub struct RaycastPlugin;

impl Plugin for RaycastPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<EditorRaycast>()
            .add_systems(Update, update_raycast);
    }
}
