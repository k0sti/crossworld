//! Octa cube scene - 2x2x2 octree with 5 colored voxels and 2 empty spaces

use cube::Cube;
use std::rc::Rc;

/// Create an octa cube scene with colored materials
///
/// The octree has depth 1 (one level of subdivision) with:
/// - 5 colored voxels: Red (1), Green (2), Blue (3), Yellow (4), White (5)
/// - 2 empty voxels: value 0 at positions 3 and 7
///
/// Material palette:
/// - 0: Empty (transparent)
/// - 1: Red
/// - 2: Green
/// - 3: Blue
/// - 4: Yellow
/// - 5: White
///
/// Octant ordering (standard octree convention):
/// - 0: (-,-,-) bottom-left-back  → RED (1)
/// - 1: (+,-,-) bottom-right-back → WHITE (5)
/// - 2: (-,+,-) top-left-back     → GREEN (2)
/// - 3: (+,+,-) top-right-back    → EMPTY (0)
/// - 4: (-,-,+) bottom-left-front → BLUE (3)
/// - 5: (+,-,+) bottom-right-front → WHITE (5)
/// - 6: (-,+,+) top-left-front    → YELLOW (4)
/// - 7: (+,+,+) top-right-front   → EMPTY (0)
pub fn create_octa_cube() -> Rc<Cube<i32>> {
    let children: [Rc<Cube<i32>>; 8] = [
        Rc::new(Cube::Solid(1)), // 0: Red
        Rc::new(Cube::Solid(5)), // 1: White
        Rc::new(Cube::Solid(2)), // 2: Green
        Rc::new(Cube::Solid(0)), // 3: Empty
        Rc::new(Cube::Solid(3)), // 4: Blue
        Rc::new(Cube::Solid(5)), // 5: White
        Rc::new(Cube::Solid(4)), // 6: Yellow
        Rc::new(Cube::Solid(0)), // 7: Empty
    ];

    Rc::new(Cube::Cubes(Box::new(children)))
}
