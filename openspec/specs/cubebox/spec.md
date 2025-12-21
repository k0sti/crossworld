# cubebox Specification

## Purpose
TBD - created by archiving change add-cubebox-type. Update Purpose after archive.
## Requirements
### Requirement: CubeBox Type Definition
The system SHALL provide a `CubeBox<T>` type that combines a `Cube<T>` with its actual voxel dimensions.

A CubeBox MUST contain:
- `cube: Cube<T>` - The octree data with the model positioned at origin (0,0,0)
- `size: IVec3` - The original model dimensions in voxels (not power-of-2 aligned)
- `depth: u32` - The octree depth, where size is measured in units of 2^depth voxels

#### Scenario: CubeBox from 16x30x12 avatar model
- **GIVEN** a MagicaVoxel model with dimensions 16x30x12
- **WHEN** loaded as a CubeBox
- **THEN** the CubeBox has size=(16, 30, 12), depth=5 (32³ octree), and cube contains the voxel data starting at origin

#### Scenario: CubeBox from 8x8x8 model
- **GIVEN** a MagicaVoxel model with dimensions 8x8x8
- **WHEN** loaded as a CubeBox
- **THEN** the CubeBox has size=(8, 8, 8), depth=3 (8³ octree), and cube fills the entire octree

### Requirement: Load Vox to CubeBox
The system SHALL provide a function `load_vox_to_cubebox(bytes: &[u8]) -> Result<CubeBox<u8>, String>` that:
- Parses .vox file bytes
- Creates an octree with minimum required depth to contain the model
- Positions the model at origin (0,0,0) within the octree
- Returns the model size and depth along with the cube

#### Scenario: Load avatar vox file
- **GIVEN** a .vox file containing a 16x30x12 avatar model
- **WHEN** `load_vox_to_cubebox(bytes)` is called
- **THEN** a CubeBox is returned with size=(16, 30, 12) and depth=5

#### Scenario: Load structure vox file
- **GIVEN** a .vox file containing a 64x32x64 structure
- **WHEN** `load_vox_to_cubebox(bytes)` is called
- **THEN** a CubeBox is returned with size=(64, 32, 64) and depth=6

#### Scenario: Invalid vox file
- **GIVEN** corrupted or invalid bytes
- **WHEN** `load_vox_to_cubebox(bytes)` is called
- **THEN** an error is returned describing the failure

### Requirement: CubeBox Placement
The system SHALL provide a method `CubeBox::place_in()` that places the bounded model into a target cube at a specified position.

The method MUST:
- Accept target cube, target depth, position, and scale parameters
- Return a new cube with the model placed (immutable operation)
- Support positive, zero, and negative scale exponents

#### Scenario: Place avatar at ground level
- **GIVEN** a 16x30x12 avatar CubeBox with depth=5
- **AND** a world cube with depth=8
- **WHEN** `place_in(world, 8, position=(100, 0, 100), scale=0)` is called
- **THEN** the avatar is placed at (100, 0, 100) in world coordinates at 1:1 scale

#### Scenario: Place structure with scale
- **GIVEN** a 32x16x32 structure CubeBox with depth=5
- **AND** a world cube with depth=10
- **WHEN** `place_in(world, 10, position=(200, 0, 200), scale=2)` is called
- **THEN** the structure is placed at 4x scale (each model voxel becomes 4x4x4 world voxels)

#### Scenario: Place multiple models
- **GIVEN** two CubeBox models and an empty world cube
- **WHEN** `place_in()` is called for each model with different positions
- **THEN** both models appear in the resulting cube at their specified positions

### Requirement: CubeBox Convenience Methods
The system SHALL provide convenience methods on CubeBox for common operations.

#### Scenario: Get octree size
- **GIVEN** a CubeBox with depth=5
- **WHEN** `octree_size()` is called
- **THEN** 32 is returned (2^5)

#### Scenario: Get bounds as AABB
- **GIVEN** a CubeBox with size=(16, 30, 12) and depth=5
- **WHEN** `bounds()` is called
- **THEN** an AABB from (0,0,0) to (16,30,12) is returned

#### Scenario: Check if size fits octree
- **GIVEN** a CubeBox with size=(16, 30, 12) and depth=5
- **WHEN** `fits_octree()` is called
- **THEN** true is returned (all dimensions <= 32)

### Requirement: WASM CubeBox Bindings
The system SHALL expose CubeBox functionality to WebAssembly with JavaScript-friendly API.

The WASM bindings MUST include:
- `WasmCubeBox` class with `cube`, `sizeX`, `sizeY`, `sizeZ`, and `depth` properties
- `loadVoxBox(bytes: Uint8Array): WasmCubeBox` function
- `WasmCubeBox.placeIn(target, targetDepth, x, y, z, scale): WasmCube` method

#### Scenario: Load vox in TypeScript
- **GIVEN** a .vox file fetched as Uint8Array
- **WHEN** `loadVoxBox(bytes)` is called
- **THEN** a WasmCubeBox is returned with accessible size and depth properties

#### Scenario: Place model in TypeScript
- **GIVEN** a WasmCubeBox and a WasmCube world
- **WHEN** `cubeBox.placeIn(world, 8, 100, 0, 100, 0)` is called
- **THEN** a new WasmCube is returned with the model placed

### Requirement: Backward Compatible Vox Loading
The system SHALL maintain the existing `load_vox_to_cube(bytes, align)` function for backward compatibility.

The function MUST:
- Be marked as deprecated with a note to use `load_vox_to_cubebox()` instead
- Continue to work with existing code
- Internally use the new loading logic

#### Scenario: Existing code continues to work
- **GIVEN** code using `load_vox_to_cube(bytes, Vec3::splat(0.5))`
- **WHEN** the code is compiled and run
- **THEN** it produces the same result as before (centered model)
- **AND** a deprecation warning is shown during compilation

