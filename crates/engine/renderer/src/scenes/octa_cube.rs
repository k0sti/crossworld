//! Octa cube scene - 2x2x2 octree with 6 colored voxels and 2 empty spaces

use cube::Cube;
use std::rc::Rc;

/// Create an octa cube scene with R2G3B2 encoded RGB colors
///
/// The octree has depth 1 (one level of subdivision) with:
/// - 6 colored voxels using R2G3B2 encoding (128-255)
/// - 2 empty voxels: value 0 at positions 3 and 7
///
/// R2G3B2 encoding: value = 128 + (r << 5) | (g << 2) | b
/// where r ∈ [0,3], g ∈ [0,7], b ∈ [0,3]
///
/// Material values (RGB colors):
/// - 224 (128 + 96): Red    (r=3, g=0, b=0)
/// - 156 (128 + 28): Green  (r=0, g=7, b=0)
/// - 131 (128 + 3):  Blue   (r=0, g=0, b=3)
/// - 252 (128 + 124): Yellow (r=3, g=7, b=0)
/// - 255 (128 + 127): White  (r=3, g=7, b=3)
/// - 159 (128 + 31): Cyan   (r=0, g=7, b=3)
///
/// Octant ordering (standard octree convention):
/// - 0: (-,-,-) bottom-left-back  → RED (224)
/// - 1: (+,-,-) bottom-right-back → CYAN (159)
/// - 2: (-,+,-) top-left-back     → GREEN (156)
/// - 3: (+,+,-) top-right-back    → EMPTY (0)
/// - 4: (-,-,+) bottom-left-front → BLUE (131)
/// - 5: (+,-,+) bottom-right-front → WHITE (255)
/// - 6: (-,+,+) top-left-front    → YELLOW (252)
/// - 7: (+,+,+) top-right-front   → EMPTY (0)
pub fn create_octa_cube() -> Rc<Cube<u8>> {
    let children: [Rc<Cube<u8>>; 8] = [
        Rc::new(Cube::Solid(224)), // Red (r=3, g=0, b=0)
        Rc::new(Cube::Solid(252)), // Yellow (r=3, g=7, b=0)
        Rc::new(Cube::Solid(156)), // Green (r=0, g=7, b=0)
        Rc::new(Cube::Solid(0)),   // Empty
        Rc::new(Cube::Solid(131)), // Blue (r=0, g=0, b=3)
        Rc::new(Cube::Solid(255)), // White (r=3, g=7, b=3)
        Rc::new(Cube::Solid(0)),   // Empty
        Rc::new(Cube::Solid(0)),   // Empty
    ];

    let cube = Cube::Cubes(Box::new(children));
    Rc::new(cube)
}
