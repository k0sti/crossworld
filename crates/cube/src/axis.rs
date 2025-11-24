use glam::Vec3;

/// Axis-aligned normal direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl Axis {
    /// Convert to Vec3 normal
    pub fn as_vec3(&self) -> Vec3 {
        match self {
            Axis::PosX => Vec3::X,
            Axis::NegX => -Vec3::X,
            Axis::PosY => Vec3::Y,
            Axis::NegY => -Vec3::Y,
            Axis::PosZ => Vec3::Z,
            Axis::NegZ => -Vec3::Z,
        }
    }

    /// Get the opposite axis
    pub fn opposite(&self) -> Self {
        match self {
            Axis::PosX => Axis::NegX,
            Axis::NegX => Axis::PosX,
            Axis::PosY => Axis::NegY,
            Axis::NegY => Axis::PosY,
            Axis::PosZ => Axis::NegZ,
            Axis::NegZ => Axis::PosZ,
        }
    }

    /// Try to create from a Vec3 (must be close to axis-aligned)
    pub fn from_vec3(v: Vec3) -> Option<Self> {
        let abs = v.abs();
        if abs.x > abs.y && abs.x > abs.z {
            return Some(if v.x > 0.0 { Axis::PosX } else { Axis::NegX });
        }
        if abs.y > abs.x && abs.y > abs.z {
            return Some(if v.y > 0.0 { Axis::PosY } else { Axis::NegY });
        }
        if abs.z > abs.x && abs.z > abs.y {
            return Some(if v.z > 0.0 { Axis::PosZ } else { Axis::NegZ });
        }
        None
    }

    /// Create from char (x/X -> PosX, y/Y -> PosY, z/Z -> PosZ)
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'x' | 'X' => Some(Axis::PosX),
            'y' | 'Y' => Some(Axis::PosY),
            'z' | 'Z' => Some(Axis::PosZ),
            _ => None,
        }
    }

    /// Convert to char (x, y, z) - ignores sign
    pub fn to_char(self) -> char {
        match self {
            Axis::PosX | Axis::NegX => 'x',
            Axis::PosY | Axis::NegY => 'y',
            Axis::PosZ | Axis::NegZ => 'z',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_axis_vectors() {
        assert_eq!(Axis::PosX.as_vec3(), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(Axis::NegX.as_vec3(), Vec3::new(-1.0, 0.0, 0.0));
        assert_eq!(Axis::PosY.as_vec3(), Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(Axis::NegY.as_vec3(), Vec3::new(0.0, -1.0, 0.0));
        assert_eq!(Axis::PosZ.as_vec3(), Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(Axis::NegZ.as_vec3(), Vec3::new(0.0, 0.0, -1.0));
    }

    #[test]
    fn test_opposite() {
        assert_eq!(Axis::PosX.opposite(), Axis::NegX);
        assert_eq!(Axis::NegX.opposite(), Axis::PosX);
        assert_eq!(Axis::PosY.opposite(), Axis::NegY);
        assert_eq!(Axis::NegY.opposite(), Axis::PosY);
        assert_eq!(Axis::PosZ.opposite(), Axis::NegZ);
        assert_eq!(Axis::NegZ.opposite(), Axis::PosZ);
    }

    #[test]
    fn test_from_vec3() {
        assert_eq!(Axis::from_vec3(Vec3::X), Some(Axis::PosX));
        assert_eq!(Axis::from_vec3(-Vec3::X), Some(Axis::NegX));
        assert_eq!(Axis::from_vec3(Vec3::new(0.1, 1.0, 0.1)), Some(Axis::PosY));
        assert_eq!(Axis::from_vec3(Vec3::ZERO), None);
    }
}
