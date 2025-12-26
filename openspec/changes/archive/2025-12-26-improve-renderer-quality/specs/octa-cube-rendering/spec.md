# octa-cube-rendering Specification Delta

## MODIFIED Requirements

### Requirement: ~~Dual~~ **Multi-Renderer** Rendering
The test SHALL render the octa cube scene with ~~both CPU and GPU~~ **all five** raytracers using identical parameters.

**Rationale**: Extended to cover mesh renderer in addition to existing CPU/GL/BCF CPU/GPU renderers.

#### Scenario: CPU rendering
*(unchanged from base spec)*

#### Scenario: GPU rendering
*(unchanged from base spec)*

#### Scenario: **Mesh rendering**
- **WHEN** the octa cube scene is rendered with the mesh renderer
- **THEN** an image is produced showing 6 visible voxel faces
- **AND** 2 empty spaces result in gaps (no faces generated)
- **AND** uses the same camera position and orientation as raytracers
- **AND** uses the same lighting parameters
- **AND** output is saved to file for comparison
- **AND** mesh is correctly culled (no inside-out faces visible)

#### Scenario: Rendering parameters match
- **WHEN** ~~both~~ **all five** CPU~~, GPU,~~ **GL, BCF CPU, GPU, and Mesh** renders are performed
- **THEN** resolution is identical (e.g., 800x600)
- **AND** camera position and orientation are identical across all renderers
- **AND** lighting configuration is identical across all renderers
- **AND** mesh renderer camera matrices match raytracer calculations

## ADDED Requirements

### Requirement: Mesh Renderer Diff Comparison
The test SHALL generate diff images comparing mesh renderer output to raytracer outputs.

**Note**: Mesh rendering uses rasterization which may produce slightly different results than raytracing (anti-aliasing, subpixel differences). Diffs should be informative but tolerance may be required.

#### Scenario: CPU-Mesh diff generation
- **WHEN** CPU and Mesh renders are compared
- **THEN** a diff image is generated showing per-pixel absolute differences
- **AND** diff highlights areas where rasterization differs from raytracing
- **AND** diff statistics include max, mean, and count of differing pixels

#### Scenario: Mesh diff tolerance
- **WHEN** mesh renderer diff is evaluated
- **THEN** minor differences (1-2 values per channel) are expected due to rasterization
- **AND** large differences indicate mesh/camera sync issues
- **AND** tolerance threshold is configurable for automated tests

#### Scenario: Mesh visual validation
- **WHEN** mesh renderer output is inspected manually
- **THEN** overall structure matches raytraced output
- **AND** voxel positions are correct
- **AND** lighting appears similar (accounting for rasterization differences)
