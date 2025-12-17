//! Fabric module - Procedural voxel generation using quaternion fields
//!
//! The fabric system generates continuous voxel surfaces using quaternion fields.
//! Unlike discrete material IDs, quaternions enable smooth interpolation and
//! gradient-based normal calculation.
//!
//! Key concepts:
//! - Quaternions are NOT normalized, encoding two properties:
//!   - Rotation (direction): Encodes world position via accumulated octant rotations
//!   - Magnitude (length): Encodes field density for surface detection
//! - Surface detection: |Q| < 1 = inside (solid), |Q| > 1 = outside (air)
//! - Normals derived from magnitude gradient

mod generator;
mod interpolation;
mod surface;
mod types;

pub use generator::FabricGenerator;
pub use interpolation::{lerp_quaternion, magnitude_from_distance, octant_offset, octant_rotation};
pub use surface::{calculate_normal, is_surface, quaternion_to_color};
pub use types::{AdditiveState, FabricConfig};
