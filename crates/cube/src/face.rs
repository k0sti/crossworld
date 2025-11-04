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

    /// Iterator over all faces
    #[inline]
    pub fn iter() -> impl Iterator<Item = Face> {
        Self::ALL.iter().copied()
    }
}
