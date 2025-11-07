use crate::traversal::{
    OFFSET_BACK, OFFSET_DOWN, OFFSET_FRONT, OFFSET_LEFT, OFFSET_RIGHT, OFFSET_UP,
};
use glam::Vec3;

/// Face direction for cube faces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Top,    // +Y
    Bottom, // -Y
    Left,   // -X
    Right,  // +X
    Front,  // +Z
    Back,   // -Z
}

impl Face {
    /// All six faces in order
    pub const ALL: [Face; 6] = [
        Face::Top,
        Face::Bottom,
        Face::Left,
        Face::Right,
        Face::Front,
        Face::Back,
    ];

    /// Face directions with their corresponding neighbor offsets and direction vectors
    ///
    /// Each entry is (Face to render, Neighbor offset to check, Direction vector from viewer to face)
    /// The face is rendered when looking from an empty voxel toward a solid neighbor.
    pub const DIRECTIONS: [(Face, i32, Vec3); 6] = [
        (Face::Right, OFFSET_LEFT, Vec3::new(-1.0, 0.0, 0.0)), // Left neighbor: render RIGHT face
        (Face::Left, OFFSET_RIGHT, Vec3::new(1.0, 0.0, 0.0)),  // Right neighbor: render LEFT face
        (Face::Top, OFFSET_DOWN, Vec3::new(0.0, -1.0, 0.0)),   // Down neighbor: render TOP face
        (Face::Bottom, OFFSET_UP, Vec3::new(0.0, 1.0, 0.0)),   // Up neighbor: render BOTTOM face
        (Face::Front, OFFSET_BACK, Vec3::new(0.0, 0.0, -1.0)), // Back neighbor: render FRONT face
        (Face::Back, OFFSET_FRONT, Vec3::new(0.0, 0.0, 1.0)),  // Front neighbor: render BACK face
    ];

    /// Get the normal vector for this face
    #[inline]
    pub fn normal(self) -> [f32; 3] {
        match self {
            Face::Top => [0.0, 1.0, 0.0],
            Face::Bottom => [0.0, -1.0, 0.0],
            Face::Left => [-1.0, 0.0, 0.0],
            Face::Right => [1.0, 0.0, 0.0],
            Face::Front => [0.0, 0.0, 1.0],
            Face::Back => [0.0, 0.0, -1.0],
        }
    }

    /// Get the four vertices for this face in counter-clockwise order when viewed from outside
    #[inline]
    pub fn vertices(self, x: f32, y: f32, z: f32, size: f32) -> [[f32; 3]; 4] {
        match self {
            Face::Top => [
                [x, y + size, z],
                [x, y + size, z + size],
                [x + size, y + size, z + size],
                [x + size, y + size, z],
            ],
            Face::Bottom => [
                [x, y, z],
                [x + size, y, z],
                [x + size, y, z + size],
                [x, y, z + size],
            ],
            Face::Left => [
                [x, y, z + size],
                [x, y + size, z + size],
                [x, y + size, z],
                [x, y, z],
            ],
            Face::Right => [
                [x + size, y, z],
                [x + size, y + size, z],
                [x + size, y + size, z + size],
                [x + size, y, z + size],
            ],
            Face::Front => [
                [x + size, y, z + size],
                [x + size, y + size, z + size],
                [x, y + size, z + size],
                [x, y, z + size],
            ],
            Face::Back => [
                [x, y, z],
                [x, y + size, z],
                [x + size, y + size, z],
                [x + size, y, z],
            ],
        }
    }

    /// Get UV coordinates for this face with world-space tiling
    /// Returns 4 UV coords matching the vertex order (counter-clockwise from outside)
    ///
    /// # Arguments
    /// * `x, y, z` - World position of the face
    /// * `size` - Size of the face in world units
    /// * `scale` - UV scale factor for texture repetition
    #[inline]
    pub fn uvs(self, x: f32, y: f32, z: f32, size: f32, scale: f32) -> [[f32; 2]; 4] {
        // Map UVs based on world position for seamless tiling across adjacent voxels
        // Round to prevent floating-point precision errors from causing z-fighting
        // when voxels at different depths share boundaries
        let round = |v: f32| -> f32 {
            // Round to nearest 1/65536 to eliminate precision errors while maintaining accuracy
            (v * 65536.0).round() / 65536.0
        };

        match self {
            Face::Top | Face::Bottom => {
                // Top/Bottom faces use X and Z coordinates
                let u0 = round(x * scale);
                let v0 = round(z * scale);
                let u1 = round((x + size) * scale);
                let v1 = round((z + size) * scale);
                [[u0, v0], [u0, v1], [u1, v1], [u1, v0]]
            }
            Face::Left | Face::Right => {
                // Left/Right faces use Z and Y coordinates
                let u0 = round(z * scale);
                let v0 = round(y * scale);
                let u1 = round((z + size) * scale);
                let v1 = round((y + size) * scale);
                [[u0, v0], [u0, v1], [u1, v1], [u1, v0]]
            }
            Face::Front | Face::Back => {
                // Front/Back faces use X and Y coordinates
                let u0 = round(x * scale);
                let v0 = round(y * scale);
                let u1 = round((x + size) * scale);
                let v1 = round((y + size) * scale);
                [[u0, v0], [u0, v1], [u1, v1], [u1, v0]]
            }
        }
    }

    /// Iterator over all faces
    #[inline]
    pub fn iter() -> impl Iterator<Item = Face> {
        Self::ALL.iter().copied()
    }
}
