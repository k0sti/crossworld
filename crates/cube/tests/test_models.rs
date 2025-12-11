//! Test models for cube mesh generation validation
//!
//! This module provides CSM test models at various depths for validating
//! mesh generation, face culling, and rendering correctness.

use cube::io::csm::parse_csm;
use cube::Cube;
use std::rc::Rc;

/// Parse CSM string and return as Cube<u8>
fn parse_test_csm(csm: &str) -> Rc<Cube<u8>> {
    let octree = parse_csm(csm).expect("Failed to parse CSM");
    // octree.root is already Cube<u8>, just wrap in Rc
    Rc::new(octree.root)
}

// ============================================================================
// Depth 0: Single Leaf Voxel (Size 1)
// ============================================================================

/// Single red voxel (depth 0) - simplest possible model
///
/// This is a leaf node with material value 1 (red).
/// Expected mesh: 6 faces (one per side of cube) if borders are empty.
///
/// CSM: `> 1`
pub fn single_leaf_cube() -> Rc<Cube<u8>> {
    const CSM: &str = "> 1";
    parse_test_csm(CSM)
}

// ============================================================================
// Depth 1: Octa Cube (Size 2)
// ============================================================================

/// Octa cube - 2x2x2 octree with mixed solid and empty voxels (depth 1)
///
/// Structure: 8 octants with different materials
/// - Some octants are solid (colored)
/// - Some octants are empty (0)
///
/// This tests neighbor-aware face culling at depth 1.
///
/// Octant layout (Z-order):
/// ```text
/// Octant indices:
///   6---7
///  /|  /|
/// 4---5 |
/// | 2-|-3
/// |/  |/
/// 0---1
/// ```
///
/// Values:
/// - 0: Red (1)
/// - 1: Yellow (2)
/// - 2: Green (3)
/// - 3: Empty (0)
/// - 4: Blue (4)
/// - 5: White (5)
/// - 6: Empty (0)
/// - 7: Empty (0)
///
/// CSM: `> [1 2 3 0 4 5 0 0]`
pub fn octa_cube_depth1() -> Rc<Cube<u8>> {
    const CSM: &str = "> [1 2 3 0 4 5 0 0]";
    parse_test_csm(CSM)
}

// ============================================================================
// Depth 2: Extended Octa Cube (Size 4)
// ============================================================================

/// Extended octa cube - depth 2 with sparse and packed subdivisions
///
/// Structure:
/// - Base: Same as octa_cube (depth 1)
/// - Subdivisions at select leaf positions:
///   - Octant 1 (Yellow): Subdivided with sparse pattern
///   - Octant 5 (White): Subdivided with all 8 filled
///
/// This tests mesh generation at depth 2 with mixed leaf depths.
///
/// CSM:
/// ```
/// > [
///   1                      # 0: Red (unchanged leaf)
///   [0 0 0 0 0 0 6 0]      # 1: Sparse subdivision (cyan at octant 6)
///   3                      # 2: Green (unchanged leaf)
///   0                      # 3: Empty
///   4                      # 4: Blue (unchanged leaf)
///   [1 7 2 3 6 4 8 5]      # 5: Packed subdivision (rainbow colors)
///   0                      # 6: Empty
///   0                      # 7: Empty
/// ]
/// ```
pub fn extended_octa_cube_depth2() -> Rc<Cube<u8>> {
    const CSM: &str = "> [1 [0 0 0 0 0 0 6 0] 3 0 4 [1 7 2 3 6 4 8 5] 0 0]";
    parse_test_csm(CSM)
}

// ============================================================================
// Depth 3: Deep Octree (Size 8)
// ============================================================================

/// Deep octree - depth 3 with nested subdivisions
///
/// Structure:
/// - Depth 0-1: Base octree with some subdivisions
/// - Depth 2: Further subdivisions at select octants
/// - Depth 3: Deepest leaves
///
/// This tests mesh generation at maximum typical depth.
///
/// CSM:
/// ```
/// > [
///   1                                      # 0: Red leaf
///   [[0 0 0 0 0 0 9 0] 10 0 0 0 0 0 0]   # 1: Depth 3 subdivision
///   3                                      # 2: Green leaf
///   0                                      # 3: Empty
///   4                                      # 4: Blue leaf
///   [1 7 2 3 6 4 8 5]                     # 5: Depth 2 subdivision
///   0                                      # 6: Empty
///   0                                      # 7: Empty
/// ]
/// ```
pub fn deep_octree_depth3() -> Rc<Cube<u8>> {
    const CSM: &str = "> [1 [[0 0 0 0 0 0 9 0] 10 0 0 0 0 0 0] 3 0 4 [1 7 2 3 6 4 8 5] 0 0]";
    parse_test_csm(CSM)
}

// ============================================================================
// Special Cases
// ============================================================================

/// All solid cube - no internal faces should be generated
///
/// Structure: 8 octants all filled with same material
///
/// Expected: Only boundary faces (6 total) if borders are empty
///
/// CSM: `> [1 1 1 1 1 1 1 1]`
pub fn all_solid_cube() -> Rc<Cube<u8>> {
    const CSM: &str = "> [1 1 1 1 1 1 1 1]";
    parse_test_csm(CSM)
}

/// All empty cube - no faces should be generated (unless borders are solid)
///
/// Structure: 8 octants all empty
///
/// Expected: 0 faces if borders are empty, some faces if borders are solid
///
/// CSM: `> [0 0 0 0 0 0 0 0]`
pub fn all_empty_cube() -> Rc<Cube<u8>> {
    const CSM: &str = "> [0 0 0 0 0 0 0 0]";
    parse_test_csm(CSM)
}

/// Checkerboard cube - alternating solid/empty pattern
///
/// Structure: 3D checkerboard with maximum internal faces
///
/// Expected: Many internal faces between solid and empty voxels
///
/// CSM: `> [1 0 1 0 0 1 0 1]`
pub fn checkerboard_cube() -> Rc<Cube<u8>> {
    const CSM: &str = "> [1 0 1 0 0 1 0 1]";
    parse_test_csm(CSM)
}

/// Single solid in empty - one voxel surrounded by empty space
///
/// Structure: Only octant 0 is solid, rest empty
///
/// Expected: 3 faces (only faces visible to empty neighbors)
///
/// CSM: `> [1 0 0 0 0 0 0 0]`
pub fn single_solid_in_empty() -> Rc<Cube<u8>> {
    const CSM: &str = "> [1 0 0 0 0 0 0 0]";
    parse_test_csm(CSM)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_models_parse() {
        // Verify all models parse successfully
        assert!(matches!(*single_leaf_cube(), Cube::Solid(_)));
        assert!(matches!(*octa_cube_depth1(), Cube::Cubes(_)));
        assert!(matches!(*extended_octa_cube_depth2(), Cube::Cubes(_)));
        assert!(matches!(*deep_octree_depth3(), Cube::Cubes(_)));
        assert!(matches!(*all_solid_cube(), Cube::Cubes(_)));
        assert!(matches!(*all_empty_cube(), Cube::Cubes(_)));
        assert!(matches!(*checkerboard_cube(), Cube::Cubes(_)));
        assert!(matches!(*single_solid_in_empty(), Cube::Cubes(_)));
    }

    #[test]
    fn test_single_leaf_is_solid() {
        let cube = single_leaf_cube();
        assert_eq!(*cube, Cube::Solid(1), "Single leaf should be Solid(1)");
    }

    #[test]
    fn test_octa_cube_structure() {
        let cube = octa_cube_depth1();
        if let Cube::Cubes(children) = &*cube {
            // Verify expected material values
            assert_eq!(*children[0], Cube::Solid(1)); // Red
            assert_eq!(*children[1], Cube::Solid(2)); // Yellow
            assert_eq!(*children[2], Cube::Solid(3)); // Green
            assert_eq!(*children[3], Cube::Solid(0)); // Empty
            assert_eq!(*children[4], Cube::Solid(4)); // Blue
            assert_eq!(*children[5], Cube::Solid(5)); // White
            assert_eq!(*children[6], Cube::Solid(0)); // Empty
            assert_eq!(*children[7], Cube::Solid(0)); // Empty
        } else {
            panic!("Expected Cubes variant");
        }
    }
}
