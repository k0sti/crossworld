//! Octa cube test scene - 2x2x2 octree with 6 solid voxels and 2 empty spaces

use cube::Cube;
use std::rc::Rc;

/// Create an octa cube test scene
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_octa_cube_structure() {
        let cube = create_octa_cube();

        // Verify it's a Cubes node
        match cube.as_ref() {
            Cube::Cubes(children) => {
                // Verify child count
                assert_eq!(children.len(), 8);

                // Verify solid children
                for (i, child) in children.iter().enumerate() {
                    match child.as_ref() {
                        Cube::Solid(val) => {
                            if i == 3 || i == 7 {
                                assert_eq!(*val, 0, "Octant {} should be empty", i);
                            } else {
                                assert_eq!(*val, 1, "Octant {} should be solid", i);
                            }
                        }
                        _ => panic!("Expected all children to be Solid nodes"),
                    }
                }
            }
            _ => panic!("Expected root to be a Cubes node"),
        }
    }
}
