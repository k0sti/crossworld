//! Fabric cube generator - produces Cube<Quat> from configuration

use super::interpolation::{apply_additive_state, calculate_child_quaternion, octant_offset};
use super::types::FabricConfig;
use crate::Cube;
use glam::{Quat, Vec3};
use std::rc::Rc;

/// Generator for fabric cubes using quaternion fields
#[derive(Debug, Clone)]
pub struct FabricGenerator {
    config: FabricConfig,
}

impl FabricGenerator {
    /// Create a new fabric generator with the given configuration
    pub fn new(config: FabricConfig) -> Self {
        Self { config }
    }

    /// Create a fabric generator with default configuration
    pub fn default_generator() -> Self {
        Self::new(FabricConfig::default())
    }

    /// Get reference to the configuration
    pub fn config(&self) -> &FabricConfig {
        &self.config
    }

    /// Generate a fabric cube to the specified depth.
    ///
    /// # Arguments
    /// * `depth` - Maximum depth to generate (0 = single node, higher = more detail)
    ///
    /// # Returns
    /// `Cube<Quat>` representing the fabric field
    pub fn generate_cube(&self, depth: u32) -> Cube<Quat> {
        // Start at origin with identity rotation and root magnitude
        let root_quat = Quat::IDENTITY * self.config.root_magnitude;
        self.generate_recursive(depth, 0, root_quat, Vec3::ZERO, 1.0)
    }

    /// Recursive generation of fabric cube.
    ///
    /// # Arguments
    /// * `remaining_depth` - How many more levels to generate
    /// * `current_depth` - Current depth in the tree (for additive state lookup)
    /// * `parent_quat` - Parent's quaternion value
    /// * `world_pos` - World position of this node's center
    /// * `world_size` - Size of this node in world units (starts at 1.0, halves each level)
    fn generate_recursive(
        &self,
        remaining_depth: u32,
        current_depth: u32,
        parent_quat: Quat,
        world_pos: Vec3,
        world_size: f32,
    ) -> Cube<Quat> {
        if remaining_depth == 0 {
            // Leaf node - return solid with this quaternion
            return Cube::Solid(parent_quat);
        }

        // Generate 8 children
        let child_size = world_size / 2.0;
        let children: [Rc<Cube<Quat>>; 8] = std::array::from_fn(|i| {
            // Calculate child's world position
            let offset = octant_offset(i) * world_size; // Scale offset by current world size
            let child_world_pos = world_pos + offset;

            // Calculate child's quaternion
            let parent_rotation = parent_quat.normalize();
            let mut child_quat =
                calculate_child_quaternion(parent_rotation, i, child_world_pos, &self.config);

            // Apply additive state for this depth
            let child_depth = current_depth + 1;
            if let Some(additive) = self.config.additive_states.get(child_depth as usize) {
                child_quat = apply_additive_state(child_quat, additive, child_world_pos);
            }

            // Recurse
            Rc::new(self.generate_recursive(
                remaining_depth - 1,
                child_depth,
                child_quat,
                child_world_pos,
                child_size,
            ))
        });

        Cube::Cubes(Box::new(children))
    }

    /// Get quaternion value at a specific world position.
    ///
    /// This performs a traversal to find the leaf containing the position
    /// and returns its quaternion value.
    ///
    /// # Arguments
    /// * `cube` - The fabric cube to query
    /// * `position` - World position in [-1, 1] range
    /// * `max_depth` - Maximum traversal depth
    ///
    /// # Returns
    /// Quaternion value at the position
    pub fn get_quaternion_at(&self, cube: &Cube<Quat>, position: Vec3, max_depth: u32) -> Quat {
        self.get_quaternion_recursive(cube, position, Vec3::ZERO, 1.0, max_depth)
    }

    #[allow(clippy::only_used_in_recursion)]
    fn get_quaternion_recursive(
        &self,
        cube: &Cube<Quat>,
        target_pos: Vec3,
        node_center: Vec3,
        node_size: f32,
        remaining_depth: u32,
    ) -> Quat {
        match cube {
            Cube::Solid(q) => *q,
            Cube::Cubes(children) if remaining_depth > 0 => {
                // Find which octant contains the target position
                let relative = target_pos - node_center;
                let octant_index = (if relative.x >= 0.0 { 1 } else { 0 })
                    | (if relative.y >= 0.0 { 2 } else { 0 })
                    | (if relative.z >= 0.0 { 4 } else { 0 });

                // Calculate child's center
                let child_size = node_size / 2.0;
                let offset = octant_offset(octant_index) * node_size;
                let child_center = node_center + offset;

                self.get_quaternion_recursive(
                    &children[octant_index],
                    target_pos,
                    child_center,
                    child_size,
                    remaining_depth - 1,
                )
            }
            Cube::Cubes(children) => {
                // At max depth but not leaf - return first child's value as approximation
                self.get_quaternion_recursive(
                    &children[0],
                    target_pos,
                    node_center,
                    node_size,
                    0,
                )
            }
            // For other cube types, get value if available
            _ => cube.value().copied().unwrap_or(Quat::IDENTITY),
        }
    }

    /// Calculate magnitude at a world position (for normal calculation).
    pub fn get_magnitude_at(&self, cube: &Cube<Quat>, position: Vec3, max_depth: u32) -> f32 {
        self.get_quaternion_at(cube, position, max_depth).length()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fabric_generator_creation() {
        let generator = FabricGenerator::default_generator();
        assert_eq!(generator.config().root_magnitude, 0.5);
        assert_eq!(generator.config().boundary_magnitude, 2.0);
    }

    #[test]
    fn test_generate_cube_depth_0() {
        let generator = FabricGenerator::default_generator();
        let cube = generator.generate_cube(0);

        // Depth 0 should be a solid leaf
        assert!(matches!(cube, Cube::Solid(_)));

        if let Cube::Solid(q) = cube {
            // Root magnitude should be applied
            assert!((q.length() - generator.config().root_magnitude).abs() < 0.001);
        }
    }

    #[test]
    fn test_generate_cube_depth_1() {
        let generator = FabricGenerator::default_generator();
        let cube = generator.generate_cube(1);

        // Depth 1 should be a Cubes variant with 8 solid children
        assert!(matches!(cube, Cube::Cubes(_)));

        if let Cube::Cubes(children) = &cube {
            assert_eq!(children.len(), 8);
            for child in children.iter() {
                assert!(matches!(**child, Cube::Solid(_)));
            }
        }
    }

    #[test]
    fn test_generate_cube_spherical_surface() {
        // Use a larger surface radius so more of the cube is inside
        let config = FabricConfig {
            root_magnitude: 0.3,      // Inside at center (well below 1.0)
            boundary_magnitude: 3.0,  // Outside at edges (well above 1.0)
            surface_radius: 1.5,      // Surface at 1.5 units from origin (outer corners at ~1.73)
            additive_states: vec![],  // No noise for predictability
            max_depth: 3,
        };
        let generator = FabricGenerator::new(config);
        let cube = generator.generate_cube(3);

        // The root node has magnitude = root_magnitude
        // Children are at offset ±0.5 from parent, so first level children are at distance ~0.87
        // With surface_radius=1.5 and root_magnitude=0.3, boundary_magnitude=3.0:
        // At distance 0.87: t = 0.87/1.5 ≈ 0.58, magnitude ≈ 0.3 + 2.7*0.58 ≈ 1.87

        // Check that point very close to center has lower magnitude than far corner
        let near_center_quat = generator.get_quaternion_at(&cube, Vec3::new(0.1, 0.1, 0.1), 3);
        let far_corner_quat = generator.get_quaternion_at(&cube, Vec3::new(0.9, 0.9, 0.9), 3);

        // Far corner should have higher magnitude than near center
        assert!(
            far_corner_quat.length() > near_center_quat.length(),
            "Far corner ({}) should have higher magnitude than near center ({})",
            far_corner_quat.length(),
            near_center_quat.length()
        );
    }

    #[test]
    fn test_get_quaternion_at_positions() {
        let config = FabricConfig {
            root_magnitude: 0.5,
            boundary_magnitude: 2.0,
            surface_radius: 1.0,
            additive_states: vec![],
            max_depth: 3,
        };
        let generator = FabricGenerator::new(config);
        let cube = generator.generate_cube(3);

        // Different positions should give different quaternions
        let q1 = generator.get_quaternion_at(&cube, Vec3::new(0.0, 0.0, 0.0), 3);
        let q2 = generator.get_quaternion_at(&cube, Vec3::new(0.5, 0.0, 0.0), 3);
        let q3 = generator.get_quaternion_at(&cube, Vec3::new(0.0, 0.5, 0.0), 3);

        // Magnitudes should increase with distance from center
        assert!(q1.length() < q2.length());
        assert!(q1.length() < q3.length());
    }

    #[test]
    fn test_magnitude_increases_with_distance() {
        let config = FabricConfig {
            root_magnitude: 0.5,
            boundary_magnitude: 2.0,
            surface_radius: 1.0,
            additive_states: vec![],
            max_depth: 4,
        };
        let generator = FabricGenerator::new(config);
        let cube = generator.generate_cube(4);

        // Sample magnitudes at increasing distances
        let distances = [0.0, 0.25, 0.5, 0.75];
        let magnitudes: Vec<f32> = distances
            .iter()
            .map(|d| generator.get_magnitude_at(&cube, Vec3::new(*d, 0.0, 0.0), 4))
            .collect();

        // Magnitudes should be monotonically increasing
        for i in 1..magnitudes.len() {
            assert!(
                magnitudes[i] >= magnitudes[i - 1],
                "Magnitude should increase with distance: {:?}",
                magnitudes
            );
        }
    }

    #[test]
    fn test_additive_states_affect_output() {
        // Generator without additive states
        let config_no_noise = FabricConfig {
            additive_states: vec![],
            ..Default::default()
        };
        let gen_no_noise = FabricGenerator::new(config_no_noise);
        let cube_no_noise = gen_no_noise.generate_cube(2);

        // Generator with additive states
        let config_with_noise = FabricConfig {
            additive_states: vec![
                super::super::types::AdditiveState::new(0.0, 0.0),
                super::super::types::AdditiveState::new(0.5, 0.2),
                super::super::types::AdditiveState::new(0.5, 0.2),
            ],
            ..Default::default()
        };
        let gen_with_noise = FabricGenerator::new(config_with_noise);
        let cube_with_noise = gen_with_noise.generate_cube(2);

        // Get quaternions at same position
        let pos = Vec3::new(0.3, 0.3, 0.3);
        let q_no_noise = gen_no_noise.get_quaternion_at(&cube_no_noise, pos, 2);
        let q_with_noise = gen_with_noise.get_quaternion_at(&cube_with_noise, pos, 2);

        // They should be different (noise affects the result)
        let diff = (q_no_noise - q_with_noise).length();
        assert!(
            diff > 0.001,
            "Additive states should affect quaternion values"
        );
    }
}
