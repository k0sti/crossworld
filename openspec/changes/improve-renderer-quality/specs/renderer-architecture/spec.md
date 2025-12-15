# renderer-architecture Specification Delta

## MODIFIED Requirements

### Requirement: Renderer Application Naming
The renderer application SHALL be named ~~"DualRendererApp"~~ **"CubeRendererApp"** to accurately reflect its multi-renderer architecture.

**Rationale**: System has expanded from 2 renderers (dual) to 5 renderers, making "dual" a misnomer.

#### Scenario: Struct naming
- **WHEN** renderer application struct is defined
- **THEN** it is named `CubeRendererApp` (not `DualRendererApp`)
- **AND** all impl blocks use `CubeRendererApp`
- **AND** constructor functions use consistent naming

#### Scenario: Function naming
- **WHEN** renderer entry point functions are defined in main.rs
- **THEN** functions are named `run_cube_renderer*` (not `run_dual_renderer*`)
- **AND** variants include `run_cube_renderer()`, `run_cube_renderer_sync()`, `run_cube_renderer_with_mode()`

#### Scenario: Variable naming
- **WHEN** renderer application instances are created
- **THEN** variables are named `cube_renderer` (not `dual_renderer`)
- **AND** naming is consistent across all files

#### Scenario: Comment and documentation updates
- **WHEN** code references the renderer system
- **THEN** comments use "cube renderer" terminology
- **AND** documentation reflects five-renderer architecture
- **AND** outdated "dual" references are removed

## ADDED Requirements

### Requirement: Five-Renderer Architecture
The renderer crate SHALL support five distinct rendering implementations for cube comparison.

#### Scenario: Renderer enumeration
- **WHEN** renderer system is described
- **THEN** it includes five renderers: CPU, GL, BCF CPU, GPU, Mesh
- **AND** each renderer implements consistent interface
- **AND** all renderers can render the same voxel cubes

#### Scenario: Synchronized rendering mode
- **WHEN** sync mode is enabled
- **THEN** all five renderers use identical camera configuration
- **AND** all five renderers use identical timestamp
- **AND** outputs are comparable for validation

#### Scenario: Individual renderer control
- **WHEN** user interacts with renderer UI
- **THEN** each renderer can be independently configured
- **AND** each renderer displays separate output
- **AND** any two renderers can be compared via diff tool

### Requirement: Renderer Documentation
The renderer crate SHALL maintain comprehensive documentation reflecting current capabilities.

#### Scenario: README accuracy
- **WHEN** README.md is read
- **THEN** it describes five-renderer architecture (not dual)
- **AND** mesh renderer is documented with features and limitations
- **AND** mesh caching option is documented
- **AND** code structure includes mesh renderer files

#### Scenario: Inline documentation
- **WHEN** code modules are read
- **THEN** module docstrings describe purpose and capabilities accurately
- **AND** public APIs have doc comments
- **AND** complex algorithms have explanatory comments

#### Scenario: Test documentation
- **WHEN** TEST_SUMMARY.md or test docs are read
- **THEN** they document mesh renderer tests
- **AND** camera sync tests are documented
- **AND** test coverage statistics are up-to-date
