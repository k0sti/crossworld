//! Default test models for renderer validation
//!
//! This module provides a collection of test models at various depths and configurations
//! to validate rendering correctness across different cube types and octree structures.
//!
//! Models are defined using CSM (CubeScript Model) format for easy modification and validation.

use cube::Cube;
use cube::io::csm::parse_csm;
use std::rc::Rc;

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse CSM string into Cube<u8>
///
/// CSM parser returns Cube<i32>, but we convert to u8 for material indices
fn parse_csm_u8(csm: &str) -> Rc<Cube<u8>> {
    let octree = parse_csm(csm).expect("Failed to parse CSM");
    // Convert Cube<i32> to Cube<u8>
    fn convert_cube(cube: &Cube<i32>) -> Cube<u8> {
        match cube {
            Cube::Solid(v) => Cube::Solid(*v as u8),
            Cube::Cubes(children) => {
                let converted: Vec<Rc<Cube<u8>>> =
                    children.iter().map(|c| Rc::new(convert_cube(c))).collect();
                let array: [Rc<Cube<u8>>; 8] = converted.try_into().unwrap();
                Cube::Cubes(Box::new(array))
            }
            Cube::Planes { axis: _, quad: _ } => {
                // Planes not fully implemented, fallback to solid
                Cube::Solid(0)
            }
            Cube::Slices { axis, layers } => {
                let converted: Vec<Rc<Cube<u8>>> =
                    layers.iter().map(|c| Rc::new(convert_cube(c))).collect();
                Cube::Slices {
                    axis: *axis,
                    layers: Rc::new(converted),
                }
            }
        }
    }
    Rc::new(convert_cube(&octree.root))
}

// ============================================================================
// Depth 0: Single Voxel (Size 1)
// ============================================================================

/// Single red voxel (depth 0)
///
/// Structure: Solid(224) where 224 = R2G3B2(r=3, g=0, b=0) = Red
///
/// CSM: `> 224`
pub fn create_single_red_voxel() -> Rc<Cube<u8>> {
    const CSM: &str = "> 224";
    parse_csm_u8(CSM)
}

// ============================================================================
// Depth 1: Octa Cube (Size 2)
// ============================================================================

/// Octa cube - 2x2x2 octree with 6 colored voxels and 2 empty spaces (depth 1)
///
/// This is the current test configuration used for validation.
///
/// R2G3B2 encoding: value = 128 + (r << 5) | (g << 2) | b
/// where r ∈ [0,3], g ∈ [0,7], b ∈ [0,3]
///
/// Material values:
/// - 224: Red    (r=3, g=0, b=0)
/// - 252: Yellow (r=3, g=7, b=0)
/// - 156: Green  (r=0, g=7, b=0)
/// - 255: White  (r=3, g=7, b=3)
/// - 131: Blue   (r=0, g=0, b=3)
///
/// Octant layout:
/// ```text
/// Y+ (top)
///   2---3      6---7
///  /|  /|     /|  /|
/// 0---1 |    4---5 |
/// | 6-|-7    | 2-|-3
/// |/  |/     |/  |/
/// 4---5      0---1
/// Z- (back)  Z+ (front)
/// ```
///
/// Values:
/// - 0: Red (224)
/// - 1: Yellow (252)
/// - 2: Green (156)
/// - 3: Empty (0)
/// - 4: Blue (131)
/// - 5: White (255)
/// - 6: Empty (0)
/// - 7: Empty (0)
///
/// CSM: `> [224 252 156 0 131 255 0 0]`
pub fn create_octa_cube() -> Rc<Cube<u8>> {
    const CSM: &str = "> [224 252 156 0 131 255 0 0]";
    parse_csm_u8(CSM)
}

// ============================================================================
// Depth 2: Extended Octa Cube (Size 4)
// ============================================================================

/// Extended octa cube - depth 2 with sparse and packed subdivisions
///
/// Structure:
/// - Base: Same as octa_cube (depth 1)
/// - Subdivisions at leaf positions:
///   - Octant 1 (Yellow): Sparse octa (mostly empty, one colored at top Y+)
///   - Octant 5 (White): Packed octa (all 8 filled with rainbow colors)
///
/// Sparse octa colors (R2G3B2):
/// - 159: Cyan (r=0, g=7, b=3) at position 6 (top-left-front)
/// - All other positions: Empty (0)
///
/// Packed octa colors (rainbow):
/// - 0: Red    (224)
/// - 1: Orange (248) = (r=3, g=5, b=0)
/// - 2: Yellow (252)
/// - 3: Green  (156)
/// - 4: Cyan   (159)
/// - 5: Blue   (131)
/// - 6: Purple (227) = (r=3, g=0, b=3)
/// - 7: White  (255)
///
/// CSM:
/// ```
/// > [
///   224                                    # 0: Red (unchanged)
///   [0 0 0 0 0 0 159 0]                    # 1: Sparse (cyan at octant 6)
///   156                                    # 2: Green (unchanged)
///   0                                      # 3: Empty
///   131                                    # 4: Blue (unchanged)
///   [224 248 252 156 159 131 227 255]     # 5: Packed (rainbow)
///   0                                      # 6: Empty
///   0                                      # 7: Empty
/// ]
/// ```
pub fn create_extended_octa_cube() -> Rc<Cube<u8>> {
    const CSM: &str = "> [224 [0 0 0 0 0 0 159 0] 156 0 131 [224 248 252 156 159 131 227 255] 0 0]";
    parse_csm_u8(CSM)
}

// ============================================================================
// Depth 3: Random Cubes (Size 8)
// ============================================================================

/// Depth 3 cube with random subdivisions and scattered cubes
///
/// Structure:
/// - Depth 0: Root octree (8 children)
/// - Depth 1: Some octants subdivided (depth 2 extended octa + new subdivisions)
/// - Depth 2: Further subdivisions with random solid voxels
/// - Depth 3: Deepest level with scattered colored cubes
///
/// This creates a complex structure with cubes at multiple depths:
/// - Octant 0: Depth 2 subdivision with random pattern
/// - Octant 1: Depth 3 subdivision (deepest, most complex)
/// - Octant 2: Simple solid (green)
/// - Octant 3: Depth 2 subdivision with sparse cubes
/// - Octant 4: Simple solid (blue)
/// - Octant 5: Depth 2 subdivision (packed rainbow from extended octa)
/// - Octant 6: Depth 3 subdivision with random scattered cubes
/// - Octant 7: Empty
///
/// Colors used (R2G3B2):
/// - 224: Red    (r=3, g=0, b=0)
/// - 248: Orange (r=3, g=5, b=0)
/// - 252: Yellow (r=3, g=7, b=0)
/// - 156: Green  (r=0, g=7, b=0)
/// - 159: Cyan   (r=0, g=7, b=3)
/// - 131: Blue   (r=0, g=0, b=3)
/// - 227: Purple (r=3, g=0, b=3)
/// - 255: White  (r=3, g=7, b=3)
///
/// CSM:
/// ```
/// > [
///   # Octant 0: Depth 2 with random pattern
///   [224 0 0 252 0 131 0 255]
///
///   # Octant 1: Depth 3 (deepest subdivision)
///   [[248 0 159 0 0 227 0 0] 0 [0 224 0 156 0 0 255 0] 0 0 [131 0 252 0 248 0 0 159] 0 0]
///
///   # Octant 2: Simple solid green
///   156
///
///   # Octant 3: Depth 2 sparse
///   [0 248 0 0 159 0 227 0]
///
///   # Octant 4: Simple solid blue
///   131
///
///   # Octant 5: Depth 2 packed rainbow (from extended octa)
///   [224 248 252 156 159 131 227 255]
///
///   # Octant 6: Depth 3 with scattered cubes
///   [0 [0 0 224 0 252 0 0 131] 0 0 [248 0 0 159 0 227 0 0] 255 0 [0 156 0 0 0 255 0 224]]
///
///   # Octant 7: Empty
///   0
/// ]
/// ```
pub fn create_depth_3_cube() -> Rc<Cube<u8>> {
    const CSM: &str = "> [[224 0 0 252 0 131 0 255] [[248 0 159 0 0 227 0 0] 0 [0 224 0 156 0 0 255 0] 0 0 [131 0 252 0 248 0 0 159] 0 0] 156 [0 248 0 0 159 0 227 0] 131 [224 248 252 156 159 131 227 255] [0 [0 0 224 0 252 0 0 131] 0 0 [248 0 0 159 0 227 0 0] 255 0 [0 156 0 0 0 255 0 224]] 0]";
    parse_csm_u8(CSM)
}

/// Simple depth 3 cube for debugging - minimal structure
///
/// Structure:
/// - Octant 0: Red (solid)
/// - Octant 1: Depth 2 subdivision with one yellow voxel at octant 0
///   - This creates a depth 3 structure: root -> octant 1 -> octant 0 -> yellow
/// - All other octants: Empty
///
/// CSM: `> [224 [[252 0 0 0 0 0 0 0] 0 0 0 0 0 0 0] 0 0 0 0 0 0]`
pub fn create_simple_depth_3_cube() -> Rc<Cube<u8>> {
    const CSM: &str = "> [224 [[252 0 0 0 0 0 0 0] 0 0 0 0 0 0 0] 0 0 0 0 0 0]";
    parse_csm_u8(CSM)
}

// ============================================================================
// Depth 1: Alternative Cube Types (Size 2)
// ============================================================================

/// Quad-like cube - simulating quad structure with octree
///
/// Structure: Octree with horizontal split (bottom 4 octants vs top 4)
/// - Bottom (Y-): Red, Yellow, Red, Yellow
/// - Top (Y+): Green, Blue, Green, Blue
///
/// Creates a striped pattern along Y axis
///
/// CSM: `> [224 252 156 131 224 252 156 131]`
pub fn create_quad_cube() -> Rc<Cube<u8>> {
    const CSM: &str = "> [224 252 156 131 224 252 156 131]";
    parse_csm_u8(CSM)
}

/// Layer cube - using Slices structure with layers along Y axis
///
/// Structure: Uses Cube::Slices with 4 layers stacked vertically
/// - Layer 0 (bottom): Red
/// - Layer 1: Yellow
/// - Layer 2: Green
/// - Layer 3 (top): Blue
///
/// Note: CSM doesn't directly support Slices, so we use octree approximation
/// CSM: `> [224 224 252 252 156 156 131 131]`
pub fn create_layer_cube() -> Rc<Cube<u8>> {
    const CSM: &str = "> [224 224 252 252 156 156 131 131]";
    parse_csm_u8(CSM)
}

/// SDF-style cube - sparse octree with only corner voxels
///
/// Structure: Octree with voxels only at 8 corners (like an SDF shell)
/// - All 8 octants have voxels
/// - Creates a hollow appearance
///
/// Colors (R2G3B2):
/// - 0: Red    (224)
/// - 1: Orange (248)
/// - 2: Yellow (252)
/// - 3: Green  (156)
/// - 4: Cyan   (159)
/// - 5: Blue   (131)
/// - 6: Purple (227)
/// - 7: White  (255)
///
/// CSM: `> [224 248 252 156 159 131 227 255]`
pub fn create_sdf_cube() -> Rc<Cube<u8>> {
    const CSM: &str = "> [224 248 252 156 159 131 227 255]";
    parse_csm_u8(CSM)
}

/// Generated cube - procedurally generated checkerboard pattern
///
/// Structure: Depth 1 octree with alternating solid/empty pattern
/// Creates a 3D checkerboard effect
///
/// Pattern (odd sum = colored, even sum = empty):
/// - 0 (0+0+0=0): Empty
/// - 1 (1+0+0=1): Red
/// - 2 (0+1+0=1): Green
/// - 3 (1+1+0=2): Empty
/// - 4 (0+0+1=1): Blue
/// - 5 (1+0+1=2): Empty
/// - 6 (0+1+1=2): Empty
/// - 7 (1+1+1=3): Yellow
///
/// CSM: `> [0 224 156 0 131 0 0 252]`
pub fn create_generated_cube() -> Rc<Cube<u8>> {
    const CSM: &str = "> [0 224 156 0 131 0 0 252]";
    parse_csm_u8(CSM)
}

// ============================================================================
// Model Registry
// ============================================================================

/// Test model configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestModel {
    // Depth-based models
    SingleRedVoxel,   // Depth 0
    OctaCube,         // Depth 1
    ExtendedOctaCube, // Depth 2
    Depth3Cube,       // Depth 3

    // Alternative cube types (depth 1)
    QuadCube,
    LayerCube,
    SdfCube,
    GeneratedCube,
}

impl TestModel {
    /// Get all available test models
    pub fn all() -> &'static [TestModel] {
        &[
            TestModel::SingleRedVoxel,
            TestModel::OctaCube,
            TestModel::ExtendedOctaCube,
            TestModel::Depth3Cube,
            TestModel::QuadCube,
            TestModel::LayerCube,
            TestModel::SdfCube,
            TestModel::GeneratedCube,
        ]
    }

    /// Get the maximum depth of this model
    pub fn max_depth(&self) -> u32 {
        match self {
            TestModel::SingleRedVoxel => 0,
            TestModel::OctaCube => 1,
            TestModel::ExtendedOctaCube => 2,
            TestModel::Depth3Cube => 3,
            TestModel::QuadCube => 1,
            TestModel::LayerCube => 1,
            TestModel::SdfCube => 1,
            TestModel::GeneratedCube => 1,
        }
    }

    /// Get a human-readable name for this model
    pub fn name(&self) -> &'static str {
        match self {
            TestModel::SingleRedVoxel => "Single Red Voxel (Depth 0)",
            TestModel::OctaCube => "Octa Cube (Depth 1)",
            TestModel::ExtendedOctaCube => "Extended Octa Cube (Depth 2)",
            TestModel::Depth3Cube => "Depth 3 Cube (Random Cubes)",
            TestModel::QuadCube => "Quad Cube (Planes)",
            TestModel::LayerCube => "Layer Cube (Slices)",
            TestModel::SdfCube => "SDF Cube (Corners)",
            TestModel::GeneratedCube => "Generated Cube (Checkerboard)",
        }
    }

    /// Get a short identifier for this model
    pub fn id(&self) -> &'static str {
        match self {
            TestModel::SingleRedVoxel => "single",
            TestModel::OctaCube => "octa",
            TestModel::ExtendedOctaCube => "extended",
            TestModel::Depth3Cube => "depth3",
            TestModel::QuadCube => "quad",
            TestModel::LayerCube => "layer",
            TestModel::SdfCube => "sdf",
            TestModel::GeneratedCube => "generated",
        }
    }

    /// Create the cube for this model
    pub fn create(&self) -> Rc<Cube<u8>> {
        match self {
            TestModel::SingleRedVoxel => create_single_red_voxel(),
            TestModel::OctaCube => create_octa_cube(),
            TestModel::ExtendedOctaCube => create_extended_octa_cube(),
            TestModel::Depth3Cube => create_depth_3_cube(),
            TestModel::QuadCube => create_quad_cube(),
            TestModel::LayerCube => create_layer_cube(),
            TestModel::SdfCube => create_sdf_cube(),
            TestModel::GeneratedCube => create_generated_cube(),
        }
    }
}
