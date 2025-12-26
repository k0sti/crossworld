//! Triangle generation from voxel faces
//!
//! Converts cube crate `FaceInfo` to Rapier `Triangle` shapes for collision detection.

use cube::{Face, FaceInfo};
use glam::Vec3;
use rapier3d::math::Point;
use rapier3d::parry::bounding_volume::Aabb;
use rapier3d::parry::shape::Triangle;

/// Generate two triangles from a voxel face
///
/// Each voxel face is a quad that gets split into 2 triangles.
/// Uses consistent winding order (counter-clockwise when viewed from outside)
/// for correct outward-facing normals.
///
/// # Arguments
/// * `face` - Face information from cube crate traversal
/// * `world_size` - Total world size in units
///
/// # Returns
/// Two triangles that together form the quad face
pub fn face_to_triangles(face: &FaceInfo, world_size: f32) -> [Triangle; 2] {
    let half_world = world_size / 2.0;

    // Face position is the voxel corner in [0,1] space
    // Convert to world space centered at origin
    let voxel_pos = face.position * world_size - Vec3::splat(half_world);
    let size = face.size * world_size;

    // Use Face::vertices from cube crate for consistent winding order
    // The cube crate defines vertices in CCW order when viewed from outside
    let local_verts = face.face.vertices(voxel_pos.x, voxel_pos.y, voxel_pos.z, size);
    let corners: [Vec3; 4] = [
        Vec3::from(local_verts[0]),
        Vec3::from(local_verts[1]),
        Vec3::from(local_verts[2]),
        Vec3::from(local_verts[3]),
    ];

    // Split quad into 2 triangles (0,1,2) and (0,2,3)
    let to_point = |v: Vec3| Point::new(v.x, v.y, v.z);
    [
        Triangle::new(to_point(corners[0]), to_point(corners[1]), to_point(corners[2])),
        Triangle::new(to_point(corners[0]), to_point(corners[2]), to_point(corners[3])),
    ]
}

/// Get a single triangle by index (0 or 1) from a face
///
/// # Arguments
/// * `face` - Face information from cube crate traversal
/// * `tri_idx` - Triangle index within the face (0 or 1)
/// * `world_size` - Total world size in units
pub fn face_to_triangle(face: &FaceInfo, tri_idx: u8, world_size: f32) -> Triangle {
    face_to_triangles(face, world_size)[tri_idx as usize]
}

/// Compute the AABB for a triangle
pub fn triangle_aabb(tri: &Triangle) -> Aabb {
    let min = tri.a.coords.inf(&tri.b.coords).inf(&tri.c.coords);
    let max = tri.a.coords.sup(&tri.b.coords).sup(&tri.c.coords);
    Aabb::new(
        Point::new(min.x, min.y, min.z),
        Point::new(max.x, max.y, max.z),
    )
}

/// Compute the AABB for a face (covers both triangles)
pub fn face_aabb(face: &FaceInfo, world_size: f32) -> Aabb {
    let half_world = world_size / 2.0;
    let voxel_pos = face.position * world_size - Vec3::splat(half_world);
    let size = face.size * world_size;

    // Face AABB depends on face direction
    let (face_min, face_max) = match face.face {
        Face::Left => {
            let x = voxel_pos.x;
            (
                Vec3::new(x, voxel_pos.y, voxel_pos.z),
                Vec3::new(x, voxel_pos.y + size, voxel_pos.z + size),
            )
        }
        Face::Right => {
            let x = voxel_pos.x + size;
            (
                Vec3::new(x, voxel_pos.y, voxel_pos.z),
                Vec3::new(x, voxel_pos.y + size, voxel_pos.z + size),
            )
        }
        Face::Bottom => {
            let y = voxel_pos.y;
            (
                Vec3::new(voxel_pos.x, y, voxel_pos.z),
                Vec3::new(voxel_pos.x + size, y, voxel_pos.z + size),
            )
        }
        Face::Top => {
            let y = voxel_pos.y + size;
            (
                Vec3::new(voxel_pos.x, y, voxel_pos.z),
                Vec3::new(voxel_pos.x + size, y, voxel_pos.z + size),
            )
        }
        Face::Back => {
            let z = voxel_pos.z;
            (
                Vec3::new(voxel_pos.x, voxel_pos.y, z),
                Vec3::new(voxel_pos.x + size, voxel_pos.y + size, z),
            )
        }
        Face::Front => {
            let z = voxel_pos.z + size;
            (
                Vec3::new(voxel_pos.x, voxel_pos.y, z),
                Vec3::new(voxel_pos.x + size, voxel_pos.y + size, z),
            )
        }
    };

    Aabb::new(face_min.to_array().into(), face_max.to_array().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cube::CubeCoord;
    use glam::IVec3;

    fn make_face(face: Face, position: Vec3, size: f32, material_id: u8) -> FaceInfo {
        FaceInfo {
            face,
            position,
            size,
            material_id,
            viewer_coord: CubeCoord::new(IVec3::ZERO, 1),
        }
    }

    #[test]
    fn test_face_to_triangles_top() {
        let face = make_face(Face::Top, Vec3::new(0.25, 0.25, 0.25), 0.25, 1);
        let [tri0, tri1] = face_to_triangles(&face, 100.0);

        // Top face at position (0.25, 0.25, 0.25) with size 0.25 in [0,1] space
        // World space: (-25, -25, -25) to (0, 0, 0) for voxel, top face at y = 0
        // Actually: voxel_pos = (0.25 * 100 - 50, 0.25 * 100 - 50, 0.25 * 100 - 50) = (-25, -25, -25)
        // size = 0.25 * 100 = 25

        // Verify normals point up (+Y)
        let normal0 = (tri0.b - tri0.a).cross(&(tri0.c - tri0.a));
        let normal1 = (tri1.b - tri1.a).cross(&(tri1.c - tri1.a));

        assert!(
            normal0.y > 0.0,
            "Triangle 0 normal should point up, got {:?}",
            normal0
        );
        assert!(
            normal1.y > 0.0,
            "Triangle 1 normal should point up, got {:?}",
            normal1
        );
    }

    #[test]
    fn test_face_to_triangles_bottom() {
        let face = make_face(Face::Bottom, Vec3::new(0.0, 0.0, 0.0), 0.5, 1);
        let [tri0, tri1] = face_to_triangles(&face, 100.0);

        // Verify normals point down (-Y)
        let normal0 = (tri0.b - tri0.a).cross(&(tri0.c - tri0.a));
        let normal1 = (tri1.b - tri1.a).cross(&(tri1.c - tri1.a));

        assert!(
            normal0.y < 0.0,
            "Triangle 0 normal should point down, got {:?}",
            normal0
        );
        assert!(
            normal1.y < 0.0,
            "Triangle 1 normal should point down, got {:?}",
            normal1
        );
    }

    #[test]
    fn test_face_to_triangles_left() {
        let face = make_face(Face::Left, Vec3::new(0.0, 0.0, 0.0), 0.5, 1);
        let [tri0, tri1] = face_to_triangles(&face, 100.0);

        // Verify normals point left (-X)
        let normal0 = (tri0.b - tri0.a).cross(&(tri0.c - tri0.a));
        let normal1 = (tri1.b - tri1.a).cross(&(tri1.c - tri1.a));

        assert!(
            normal0.x < 0.0,
            "Triangle 0 normal should point left, got {:?}",
            normal0
        );
        assert!(
            normal1.x < 0.0,
            "Triangle 1 normal should point left, got {:?}",
            normal1
        );
    }

    #[test]
    fn test_face_to_triangles_right() {
        let face = make_face(Face::Right, Vec3::new(0.0, 0.0, 0.0), 0.5, 1);
        let [tri0, tri1] = face_to_triangles(&face, 100.0);

        // Verify normals point right (+X)
        let normal0 = (tri0.b - tri0.a).cross(&(tri0.c - tri0.a));
        let normal1 = (tri1.b - tri1.a).cross(&(tri1.c - tri1.a));

        assert!(
            normal0.x > 0.0,
            "Triangle 0 normal should point right, got {:?}",
            normal0
        );
        assert!(
            normal1.x > 0.0,
            "Triangle 1 normal should point right, got {:?}",
            normal1
        );
    }

    #[test]
    fn test_face_to_triangles_front() {
        let face = make_face(Face::Front, Vec3::new(0.0, 0.0, 0.0), 0.5, 1);
        let [tri0, tri1] = face_to_triangles(&face, 100.0);

        // Verify normals point front (+Z)
        let normal0 = (tri0.b - tri0.a).cross(&(tri0.c - tri0.a));
        let normal1 = (tri1.b - tri1.a).cross(&(tri1.c - tri1.a));

        assert!(
            normal0.z > 0.0,
            "Triangle 0 normal should point front, got {:?}",
            normal0
        );
        assert!(
            normal1.z > 0.0,
            "Triangle 1 normal should point front, got {:?}",
            normal1
        );
    }

    #[test]
    fn test_face_to_triangles_back() {
        let face = make_face(Face::Back, Vec3::new(0.0, 0.0, 0.0), 0.5, 1);
        let [tri0, tri1] = face_to_triangles(&face, 100.0);

        // Verify normals point back (-Z)
        let normal0 = (tri0.b - tri0.a).cross(&(tri0.c - tri0.a));
        let normal1 = (tri1.b - tri1.a).cross(&(tri1.c - tri1.a));

        assert!(
            normal0.z < 0.0,
            "Triangle 0 normal should point back, got {:?}",
            normal0
        );
        assert!(
            normal1.z < 0.0,
            "Triangle 1 normal should point back, got {:?}",
            normal1
        );
    }

    #[test]
    fn test_face_to_triangle_single() {
        let face = make_face(Face::Top, Vec3::new(0.0, 0.0, 0.0), 0.5, 1);
        let [expected0, expected1] = face_to_triangles(&face, 100.0);

        let tri0 = face_to_triangle(&face, 0, 100.0);
        let tri1 = face_to_triangle(&face, 1, 100.0);

        // Compare vertices
        assert_eq!(tri0.a, expected0.a);
        assert_eq!(tri0.b, expected0.b);
        assert_eq!(tri0.c, expected0.c);

        assert_eq!(tri1.a, expected1.a);
        assert_eq!(tri1.b, expected1.b);
        assert_eq!(tri1.c, expected1.c);
    }

    #[test]
    fn test_triangle_aabb() {
        let tri = Triangle::new(
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
            Point::new(0.0, 1.0, 0.0),
        );

        let aabb = triangle_aabb(&tri);

        assert!((aabb.mins.x - 0.0).abs() < 1e-6);
        assert!((aabb.mins.y - 0.0).abs() < 1e-6);
        assert!((aabb.mins.z - 0.0).abs() < 1e-6);
        assert!((aabb.maxs.x - 1.0).abs() < 1e-6);
        assert!((aabb.maxs.y - 1.0).abs() < 1e-6);
        assert!((aabb.maxs.z - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_face_aabb() {
        // Top face at origin, size 0.5 in local space, world size 100
        let face = make_face(Face::Top, Vec3::new(0.0, 0.0, 0.0), 0.5, 1);
        let aabb = face_aabb(&face, 100.0);

        // voxel_pos = (0, 0, 0) * 100 - 50 = (-50, -50, -50)
        // size = 0.5 * 100 = 50
        // Top face is at y = voxel_pos.y + size = -50 + 50 = 0

        assert!((aabb.mins.x - (-50.0)).abs() < 1e-6);
        assert!((aabb.mins.y - 0.0).abs() < 1e-6); // Top face at y = 0
        assert!((aabb.mins.z - (-50.0)).abs() < 1e-6);
        assert!((aabb.maxs.x - 0.0).abs() < 1e-6);
        assert!((aabb.maxs.y - 0.0).abs() < 1e-6); // Flat face
        assert!((aabb.maxs.z - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_world_space_transformation() {
        // Test that face positions are correctly transformed to world space
        let world_size = 100.0;

        // Face at center of local space
        let face = make_face(Face::Top, Vec3::new(0.5, 0.5, 0.5), 0.25, 1);
        let [tri0, _] = face_to_triangles(&face, world_size);

        // Center of local space (0.5, 0.5, 0.5) should map to world (0, 0, 0)
        // With size 0.25, voxel goes from (0, 0, 0) to (25, 25, 25)
        // Top face y = 25

        // All vertices should be at y = 25 (top face)
        assert!((tri0.a.y - 25.0).abs() < 1e-6);
        assert!((tri0.b.y - 25.0).abs() < 1e-6);
        assert!((tri0.c.y - 25.0).abs() < 1e-6);
    }
}
