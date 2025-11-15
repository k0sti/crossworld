//! Octa cube scene - 2x2x2 octree with 6 solid voxels and 2 empty spaces

use cube::Cube;
use std::rc::Rc;

/// Create an octa cube scene
///
/// The octree has depth 1 (one level of subdivision) with:
/// - 6 voxels set to value 1 (solid)
/// - 2 voxels set to value 0 (empty) at positions 3 and 7
///
/// Octant ordering (standard octree convention):
/// - 0: (-,-,-) bottom-left-back
/// - 1: (+,-,-) bottom-right-back
/// - 2: (-,+,-) top-left-back
/// - 3: (+,+,-) top-right-back [EMPTY]
/// - 4: (-,-,+) bottom-left-front
/// - 5: (+,-,+) bottom-right-front
/// - 6: (-,+,+) top-left-front
/// - 7: (+,+,+) top-right-front [EMPTY]
pub fn create_octa_cube() -> Rc<Cube<i32>> {
    let children: [Rc<Cube<i32>>; 8] = [
        Rc::new(Cube::Solid(1)), // 0: solid
        Rc::new(Cube::Solid(1)), // 1: solid
        Rc::new(Cube::Solid(1)), // 2: solid
        Rc::new(Cube::Solid(0)), // 3: empty
        Rc::new(Cube::Solid(1)), // 4: solid
        Rc::new(Cube::Solid(1)), // 5: solid
        Rc::new(Cube::Solid(1)), // 6: solid
        Rc::new(Cube::Solid(0)), // 7: empty
    ];

    Rc::new(Cube::Cubes(Box::new(children)))
}
