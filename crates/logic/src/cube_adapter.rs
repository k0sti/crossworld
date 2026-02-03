//! Cube crate integration for the logic system
//!
//! This module provides adapters to use the rule engine with the `cube` crate's
//! octree data structures.

use crate::{RuleContext, RuleExecutor};
use cube::{Cube, CubeCoord};
use glam::IVec3;
use std::cell::RefCell;
use std::rc::Rc;

/// Adapter for using a `Cube<u8>` with the rule engine
///
/// This wraps a reference-counted Cube and provides the `RuleContext` and
/// `RuleExecutor` traits for rule evaluation and execution.
///
/// # Note on Coordinate Systems
///
/// The Cube crate uses corner-based coordinates in the range `[0, 2^depth)`.
/// World positions passed to the rule engine are converted to cube coordinates
/// internally.
pub struct CubeAdapter {
    /// The wrapped cube (mutable via RefCell for interior mutability)
    cube: Rc<RefCell<Cube<u8>>>,

    /// Maximum depth for voxel resolution
    depth: u32,
}

impl CubeAdapter {
    /// Create a new adapter wrapping a cube
    pub fn new(cube: Cube<u8>, depth: u32) -> Self {
        CubeAdapter {
            cube: Rc::new(RefCell::new(cube)),
            depth,
        }
    }

    /// Create a new adapter with a shared reference
    pub fn from_shared(cube: Rc<RefCell<Cube<u8>>>, depth: u32) -> Self {
        CubeAdapter { cube, depth }
    }

    /// Get the wrapped cube
    pub fn cube(&self) -> std::cell::Ref<Cube<u8>> {
        self.cube.borrow()
    }

    /// Get mutable access to the wrapped cube
    pub fn cube_mut(&self) -> std::cell::RefMut<Cube<u8>> {
        self.cube.borrow_mut()
    }

    /// Take ownership of the cube
    pub fn into_cube(self) -> Cube<u8> {
        Rc::try_unwrap(self.cube)
            .ok()
            .expect("Cannot take cube - other references exist")
            .into_inner()
    }

    /// Get the depth setting
    pub fn depth(&self) -> u32 {
        self.depth
    }

    /// Convert world position to cube coordinate
    ///
    /// World positions are expected to be in the range `[0, 2^depth)`.
    fn to_cube_coord(&self, pos: IVec3) -> CubeCoord {
        CubeCoord::new(pos, self.depth)
    }

    /// Check if a position is within bounds
    fn in_bounds(&self, pos: IVec3) -> bool {
        let max = 1 << self.depth;
        pos.x >= 0 && pos.y >= 0 && pos.z >= 0 && pos.x < max && pos.y < max && pos.z < max
    }
}

impl RuleContext for CubeAdapter {
    fn get_material(&self, position: IVec3) -> u8 {
        if !self.in_bounds(position) {
            return 0; // Out of bounds = empty
        }

        let coord = self.to_cube_coord(position);
        let cube = self.cube.borrow();
        cube.get(coord).id()
    }

    fn get_depth(&self, _position: IVec3) -> u32 {
        self.depth
    }
}

impl RuleExecutor for CubeAdapter {
    fn set_material(&mut self, position: IVec3, material: u8) -> u8 {
        if !self.in_bounds(position) {
            return 0; // Out of bounds, no change
        }

        let coord = self.to_cube_coord(position);
        let old_material;

        {
            let cube = self.cube.borrow();
            old_material = cube.get(coord).id();
        }

        {
            let mut cube = self.cube.borrow_mut();
            let new_cube = cube.update(coord, Cube::Solid(material));
            *cube = new_cube;
        }

        old_material
    }

    fn get_material(&self, position: IVec3) -> u8 {
        RuleContext::get_material(self, position)
    }
}

/// Builder for creating cubes with rule-based generation
pub struct CubeBuilder {
    depth: u32,
    cube: Cube<u8>,
}

impl CubeBuilder {
    /// Create a new builder with empty cube at given depth
    pub fn new(depth: u32) -> Self {
        CubeBuilder {
            depth,
            cube: Cube::Solid(0),
        }
    }

    /// Create a builder from an existing cube
    pub fn from_cube(cube: Cube<u8>, depth: u32) -> Self {
        CubeBuilder { depth, cube }
    }

    /// Set a voxel at a position
    pub fn set(mut self, pos: IVec3, material: u8) -> Self {
        let coord = CubeCoord::new(pos, self.depth);
        self.cube = self.cube.update(coord, Cube::Solid(material));
        self
    }

    /// Fill a region with a material
    pub fn fill(mut self, min: IVec3, max: IVec3, material: u8) -> Self {
        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    let coord = CubeCoord::new(IVec3::new(x, y, z), self.depth);
                    self.cube = self.cube.update(coord, Cube::Solid(material));
                }
            }
        }
        self
    }

    /// Build the final cube
    pub fn build(self) -> Cube<u8> {
        self.cube.simplified()
    }

    /// Build into a CubeAdapter for use with the rule engine
    pub fn into_adapter(self) -> CubeAdapter {
        let depth = self.depth;
        CubeAdapter::new(self.build(), depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, Condition, Rule, RuleEngine, RuleTx};

    #[test]
    fn test_cube_adapter_get_material() {
        let cube = CubeBuilder::new(3)
            .set(IVec3::new(1, 2, 3), 5)
            .set(IVec3::new(0, 0, 0), 10)
            .build();

        let adapter = CubeAdapter::new(cube, 3);

        assert_eq!(adapter.get_material(IVec3::new(1, 2, 3)), 5);
        assert_eq!(adapter.get_material(IVec3::new(0, 0, 0)), 10);
        assert_eq!(adapter.get_material(IVec3::new(7, 7, 7)), 0); // Empty
    }

    #[test]
    fn test_cube_adapter_set_material() {
        let cube = Cube::Solid(0);
        let mut adapter = CubeAdapter::new(cube, 3);

        let old = adapter.set_material(IVec3::new(2, 2, 2), 7);
        assert_eq!(old, 0);
        assert_eq!(adapter.get_material(IVec3::new(2, 2, 2)), 7);
    }

    #[test]
    fn test_cube_adapter_with_rule_engine() {
        let mut engine = RuleEngine::new();

        // Rule: Turn material 1 into material 2
        engine
            .add_rule(
                Rule::new("transform")
                    .when(Condition::material(1))
                    .then(Action::set(2)),
            )
            .unwrap();

        let cube = CubeBuilder::new(3)
            .set(IVec3::new(4, 4, 4), 1) // This should match
            .set(IVec3::new(0, 0, 0), 3) // This shouldn't match
            .build();

        let mut adapter = CubeAdapter::new(cube, 3);

        // Evaluate at position with material 1
        let mut tx = RuleTx::new();
        engine.evaluate(IVec3::new(4, 4, 4), &adapter, &mut tx);

        // Execute the transaction
        engine.execute(&mut tx, &mut adapter);

        // Verify transformation
        assert_eq!(adapter.get_material(IVec3::new(4, 4, 4)), 2);
        assert_eq!(adapter.get_material(IVec3::new(0, 0, 0)), 3); // Unchanged
    }

    #[test]
    fn test_cube_builder() {
        let cube = CubeBuilder::new(2)
            .fill(IVec3::ZERO, IVec3::ONE, 1)
            .set(IVec3::new(0, 0, 0), 5)
            .build();

        // Access directly using CubeCoord
        assert_eq!(cube.get(CubeCoord::new(IVec3::new(0, 0, 0), 2)).id(), 5);
        assert_eq!(cube.get(CubeCoord::new(IVec3::new(1, 1, 1), 2)).id(), 1);
    }

    #[test]
    fn test_out_of_bounds() {
        let adapter = CubeAdapter::new(Cube::Solid(1), 2);

        // depth=2 means range [0, 4)
        assert_eq!(adapter.get_material(IVec3::new(0, 0, 0)), 1);
        assert_eq!(adapter.get_material(IVec3::new(3, 3, 3)), 1);
        assert_eq!(adapter.get_material(IVec3::new(4, 0, 0)), 0); // Out of bounds
        assert_eq!(adapter.get_material(IVec3::new(-1, 0, 0)), 0); // Out of bounds
    }
}
