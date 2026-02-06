use glam::IVec3;
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

    /// Component index: 0=X, 1=Y, 2=Z
    #[inline]
    pub fn index(self) -> usize {
        (self as usize) >> 1
    }

    /// Sign: 1 for Pos*, -1 for Neg*
    #[inline]
    pub fn sign(self) -> i32 {
        1 - ((self as i32) & 1) * 2
    }

    #[inline]
    pub fn sign_f32(self) -> f32 {
        self.sign() as f32
    }

    /// Construct from component index and sign
    #[inline]
    pub fn from_index_sign(index: usize, sign: i32) -> Self {
        const TABLE: [Axis; 6] = [
            Axis::NegX,
            Axis::PosX,
            Axis::NegY,
            Axis::PosY,
            Axis::NegZ,
            Axis::PosZ,
        ];
        TABLE[index * 2 + ((sign + 1) >> 1) as usize]
    }

    /// Flip direction: PosX <-> NegX, etc.
    #[inline]
    pub fn flip(self) -> Self {
        const TABLE: [Axis; 6] = [
            Axis::NegX,
            Axis::PosX,
            Axis::NegY,
            Axis::PosY,
            Axis::NegZ,
            Axis::PosZ,
        ];
        TABLE[self as usize]
    }

    /// Unit vector
    #[inline]
    pub fn to_vec3(self) -> Vec3 {
        const TABLE: [Vec3; 6] = [
            Vec3::X,
            Vec3::NEG_X,
            Vec3::Y,
            Vec3::NEG_Y,
            Vec3::Z,
            Vec3::NEG_Z,
        ];
        TABLE[self as usize]
    }

    /// Unit vector (integer)
    #[inline]
    pub fn to_ivec3(self) -> IVec3 {
        const TABLE: [IVec3; 6] = [
            IVec3::X,
            IVec3::NEG_X,
            IVec3::Y,
            IVec3::NEG_Y,
            IVec3::Z,
            IVec3::NEG_Z,
        ];
        TABLE[self as usize]
    }

    /// Get this axis component from vector
    #[inline]
    pub fn of(self, v: Vec3) -> f32 {
        v[self.index()]
    }

    /// Get this axis component from integer vector
    #[inline]
    pub fn of_i(self, v: IVec3) -> i32 {
        v[self.index()]
    }

    /// Return vector with this component set to value
    #[inline]
    pub fn set(self, mut v: Vec3, val: f32) -> Vec3 {
        v[self.index()] = val;
        v
    }

    /// Return integer vector with this component set to value
    #[inline]
    pub fn set_i(self, mut v: IVec3, val: i32) -> IVec3 {
        v[self.index()] = val;
        v
    }

    /// Return integer vector with this component incremented by sign
    #[inline]
    pub fn step(self, mut v: IVec3) -> IVec3 {
        let i = self.index();
        v[i] += self.sign();
        v
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
