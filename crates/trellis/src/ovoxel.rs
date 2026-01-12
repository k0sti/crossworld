use glam::{IVec3, Vec3};
use serde::{Deserialize, Serialize};

/// Sparse voxel representation from Trellis.2 output
///
/// This format represents 3D voxel data as a collection of sparse coordinates
/// with associated color attributes. It's optimized for storage and transmission
/// of sparse voxel data from the Trellis.2 mesh-to-voxel pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OVoxel {
    /// Sparse voxel coordinates [N, 3] (integer positions)
    pub coords: Vec<IVec3>,

    /// RGB color attributes [N, 3] (float [0, 1])
    pub attrs: Vec<[f32; 3]>,

    /// Size of each voxel in world space
    pub voxel_size: f32,

    /// Axis-aligned bounding box [min, max]
    pub aabb: [Vec3; 2],
}

impl OVoxel {
    /// Create a new OVoxel from components
    pub fn new(coords: Vec<IVec3>, attrs: Vec<[f32; 3]>, voxel_size: f32, aabb: [Vec3; 2]) -> Self {
        Self {
            coords,
            attrs,
            voxel_size,
            aabb,
        }
    }

    /// Validate the OVoxel structure
    pub fn validate(&self) -> Result<(), OVoxelError> {
        // Check that coords and attrs have the same length
        if self.coords.len() != self.attrs.len() {
            return Err(OVoxelError::MismatchedLengths {
                coords: self.coords.len(),
                attrs: self.attrs.len(),
            });
        }

        // Check for empty data
        if self.coords.is_empty() {
            return Err(OVoxelError::EmptyData);
        }

        // Validate voxel size
        if self.voxel_size <= 0.0 || !self.voxel_size.is_finite() {
            return Err(OVoxelError::InvalidVoxelSize(self.voxel_size));
        }

        // Validate AABB
        let min = self.aabb[0];
        let max = self.aabb[1];
        if min.x >= max.x || min.y >= max.y || min.z >= max.z {
            return Err(OVoxelError::InvalidAABB { min, max });
        }

        // Validate color attributes (must be in [0, 1] range)
        for (i, attr) in self.attrs.iter().enumerate() {
            for (channel_idx, &value) in attr.iter().enumerate() {
                if !(0.0..=1.0).contains(&value) || !value.is_finite() {
                    return Err(OVoxelError::InvalidColorAttribute {
                        voxel_index: i,
                        channel: channel_idx,
                        value,
                    });
                }
            }
        }

        Ok(())
    }

    /// Get the dimensions of the bounding box
    pub fn dimensions(&self) -> Vec3 {
        self.aabb[1] - self.aabb[0]
    }

    /// Get the center of the bounding box
    pub fn center(&self) -> Vec3 {
        (self.aabb[0] + self.aabb[1]) * 0.5
    }

    /// Get the number of voxels
    pub fn len(&self) -> usize {
        self.coords.len()
    }

    /// Check if the OVoxel is empty
    pub fn is_empty(&self) -> bool {
        self.coords.is_empty()
    }

    /// Iterate over voxels (coordinate + color pairs)
    pub fn iter(&self) -> impl Iterator<Item = (&IVec3, &[f32; 3])> {
        self.coords.iter().zip(self.attrs.iter())
    }
}

/// Errors that can occur when working with OVoxel data
#[derive(Debug, Clone, thiserror::Error)]
pub enum OVoxelError {
    #[error("Mismatched lengths: coords={coords}, attrs={attrs}")]
    MismatchedLengths { coords: usize, attrs: usize },

    #[error("OVoxel data is empty")]
    EmptyData,

    #[error("Invalid voxel size: {0}")]
    InvalidVoxelSize(f32),

    #[error("Invalid AABB: min={min:?}, max={max:?}")]
    InvalidAABB { min: Vec3, max: Vec3 },

    #[error("Invalid color attribute at voxel {voxel_index}, channel {channel}: value={value}")]
    InvalidColorAttribute {
        voxel_index: usize,
        channel: usize,
        value: f32,
    },

    #[error("Failed to parse JSON: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ovoxel_creation() {
        let coords = vec![IVec3::new(0, 0, 0), IVec3::new(1, 1, 1)];
        let attrs = vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let voxel_size = 1.0;
        let aabb = [Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0)];

        let ovoxel = OVoxel::new(coords, attrs, voxel_size, aabb);
        assert_eq!(ovoxel.len(), 2);
        assert!(!ovoxel.is_empty());
    }

    #[test]
    fn test_ovoxel_validation() {
        let coords = vec![IVec3::new(0, 0, 0)];
        let attrs = vec![[0.5, 0.5, 0.5]];
        let voxel_size = 1.0;
        let aabb = [Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)];

        let ovoxel = OVoxel::new(coords, attrs, voxel_size, aabb);
        assert!(ovoxel.validate().is_ok());
    }

    #[test]
    fn test_mismatched_lengths() {
        let coords = vec![IVec3::new(0, 0, 0)];
        let attrs = vec![[0.5, 0.5, 0.5], [0.3, 0.3, 0.3]];
        let voxel_size = 1.0;
        let aabb = [Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)];

        let ovoxel = OVoxel::new(coords, attrs, voxel_size, aabb);
        assert!(matches!(
            ovoxel.validate(),
            Err(OVoxelError::MismatchedLengths { .. })
        ));
    }

    #[test]
    fn test_invalid_color() {
        let coords = vec![IVec3::new(0, 0, 0)];
        let attrs = vec![[1.5, 0.5, 0.5]]; // Invalid: > 1.0
        let voxel_size = 1.0;
        let aabb = [Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)];

        let ovoxel = OVoxel::new(coords, attrs, voxel_size, aabb);
        assert!(matches!(
            ovoxel.validate(),
            Err(OVoxelError::InvalidColorAttribute { .. })
        ));
    }

    #[test]
    fn test_invalid_aabb() {
        let coords = vec![IVec3::new(0, 0, 0)];
        let attrs = vec![[0.5, 0.5, 0.5]];
        let voxel_size = 1.0;
        let aabb = [Vec3::new(1.0, 1.0, 1.0), Vec3::new(0.0, 0.0, 0.0)]; // Invalid: min > max

        let ovoxel = OVoxel::new(coords, attrs, voxel_size, aabb);
        assert!(matches!(
            ovoxel.validate(),
            Err(OVoxelError::InvalidAABB { .. })
        ));
    }

    #[test]
    fn test_dimensions_and_center() {
        let coords = vec![IVec3::new(0, 0, 0)];
        let attrs = vec![[0.5, 0.5, 0.5]];
        let voxel_size = 1.0;
        let aabb = [Vec3::new(0.0, 0.0, 0.0), Vec3::new(10.0, 20.0, 30.0)];

        let ovoxel = OVoxel::new(coords, attrs, voxel_size, aabb);
        assert_eq!(ovoxel.dimensions(), Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(ovoxel.center(), Vec3::new(5.0, 10.0, 15.0));
    }
}
