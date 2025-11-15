## ADDED Requirements

### Requirement: Octa Cube Test Scene
The renderer SHALL provide a test scene consisting of an octa cube (2x2x2 subdivision) with exactly 2 empty spaces and 6 solid voxels.

#### Scenario: Octree structure definition
- **WHEN** the octa cube test scene is created
- **THEN** the octree has depth 1 (8 child voxels)
- **AND** 6 voxels have non-zero values (solid)
- **AND** 2 voxels have value 0 (empty)
- **AND** voxel positions are deterministic (e.g., corners at indices 0 and 7)

#### Scenario: Scene positioning
- **WHEN** the octa cube is placed in world space
- **THEN** it is positioned at a known coordinate (e.g., world origin)
- **AND** camera is positioned to view all 6 solid voxels
- **AND** empty spaces are visible as gaps in the rendered output

### Requirement: Dual Rendering
The test SHALL render the octa cube scene with both CPU and GPU raytracers using identical parameters.

#### Scenario: CPU rendering
- **WHEN** the octa cube scene is rendered with the CPU raytracer
- **THEN** an image is produced showing 6 visible voxel cubes
- **AND** 2 empty spaces are correctly skipped (show background)
- **AND** lighting and normals are correctly calculated
- **AND** output is saved to file for comparison

#### Scenario: GPU rendering
- **WHEN** the octa cube scene is rendered with the GPU raytracer
- **THEN** an image is produced with the same resolution as CPU render
- **AND** uses the same camera position and orientation
- **AND** uses the same lighting parameters
- **AND** output is saved to file for comparison

#### Scenario: Rendering parameters match
- **WHEN** both CPU and GPU renders are performed
- **THEN** resolution is identical (e.g., 512x512)
- **AND** camera position and orientation are identical
- **AND** max_depth parameter is identical
- **AND** lighting configuration is identical

### Requirement: Pixel Difference Comparison
The test SHALL generate a pixel-by-pixel difference image comparing CPU and GPU raytracer outputs.

#### Scenario: Diff image generation
- **WHEN** CPU and GPU renders are compared
- **THEN** a diff image is generated showing per-pixel absolute differences
- **AND** diff image is saved to file for inspection
- **AND** differences are highlighted visually (e.g., red/white scale)

#### Scenario: Diff statistics calculation
- **WHEN** diff image is generated
- **THEN** maximum pixel difference is calculated
- **AND** mean pixel difference is calculated
- **AND** count of differing pixels is calculated
- **AND** statistics are logged to test output

#### Scenario: Zero difference validation
- **WHEN** GPU and CPU raytracers are correctly implemented
- **THEN** maximum pixel difference is 0
- **AND** all pixels match exactly (pixel-perfect equivalence)
- **AND** diff image is completely black (no differences)

### Requirement: Automated Test Validation
The test SHALL automatically fail if any pixel differences are detected between CPU and GPU renders.

#### Scenario: Test passes on exact match
- **WHEN** CPU and GPU renders are identical
- **THEN** the test passes with exit code 0
- **AND** success message is logged

#### Scenario: Test fails on mismatch
- **WHEN** any pixel difference is detected
- **THEN** the test fails with non-zero exit code
- **AND** diff statistics are logged
- **AND** paths to output images are provided for inspection
- **AND** error message indicates which pixels differ

#### Scenario: Tolerance threshold (optional)
- **WHEN** floating-point precision may cause minor differences
- **THEN** a configurable tolerance threshold can be set (e.g., 1/255)
- **AND** differences below threshold are ignored
- **AND** threshold is documented and justified

### Requirement: Empty Space Traversal Validation
The test SHALL verify that both CPU and GPU raytracers correctly skip empty voxels (value=0) during traversal.

#### Scenario: Ray passes through empty space
- **WHEN** a ray passes through one of the 2 empty voxel positions
- **THEN** both raytracers skip the empty voxel without stopping
- **AND** both raytracers continue to the background or next solid voxel
- **AND** empty space pixels show background color (not black or error color)

#### Scenario: Ray hits solid voxel after empty space
- **WHEN** a ray passes through empty space before hitting a solid voxel
- **THEN** both raytracers correctly detect the solid voxel hit
- **AND** hit position and normal are correct for the solid voxel
- **AND** empty space does not affect the final hit result

### Requirement: Visual Verification Support
The test SHALL produce output images suitable for manual visual inspection to confirm correctness.

#### Scenario: CPU output inspection
- **WHEN** the CPU render is saved to file
- **THEN** the image shows 6 distinct voxel cubes
- **AND** 2 gaps are visible at the empty voxel positions
- **AND** lighting and shadows (if any) are visually correct
- **AND** image filename indicates it is the CPU reference

#### Scenario: GPU output inspection
- **WHEN** the GPU render is saved to file
- **THEN** the image appears visually identical to CPU render
- **AND** filename indicates it is the GPU output

#### Scenario: Diff output inspection
- **WHEN** the diff image is saved to file
- **THEN** differences (if any) are visually highlighted
- **AND** filename indicates it is the diff image
- **AND** scale/legend explains diff visualization (if applicable)

### Requirement: Test Integration
The octa cube validation test SHALL be integrated into the project's test suite and run automatically.

#### Scenario: Cargo test execution
- **WHEN** `cargo test --workspace` is run
- **THEN** the octa cube test is executed
- **AND** test results are included in test output
- **AND** test passes or fails with clear messaging

#### Scenario: CI/CD integration
- **WHEN** tests run in continuous integration environment
- **THEN** the octa cube test executes successfully
- **AND** test artifacts (images) are available for inspection
- **AND** test failure blocks PR/merge if differences detected

#### Scenario: Test isolation
- **WHEN** the octa cube test runs
- **THEN** it does not depend on other tests passing
- **AND** it does not modify shared state
- **AND** it can be run independently with `cargo test octa_cube`
