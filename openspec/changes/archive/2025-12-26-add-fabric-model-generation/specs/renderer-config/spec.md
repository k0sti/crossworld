## ADDED Requirements

### Requirement: Unified Renderer Configuration File
The renderer SHALL use a single `config.ron` file that consolidates model definitions, single cube settings, fabric parameters, and rendering options.

#### Scenario: Config file loads successfully
- **GIVEN** a valid `config.ron` file exists at `crates/renderer/config.ron`
- **WHEN** the renderer application starts
- **THEN** the configuration is loaded without errors
- **AND** all sections (models, single_cube, fabric, rendering) are accessible

#### Scenario: Config file structure
- **GIVEN** a config.ron file
- **WHEN** examining its structure
- **THEN** it contains:
  - `models`: Array of model entries (id, name, csm/vox_path)
  - `single_cube`: Single cube configuration (default_material)
  - `fabric`: Fabric generation parameters (additive_states, default_max_depth)
  - `rendering`: General rendering settings (default_resolution)

### Requirement: Single Cube Configuration Section
The config file SHALL contain a `single_cube` section with material parameters for the single voxel model.

#### Scenario: Default material from config
- **GIVEN** config.ron contains `single_cube: (default_material: 224)`
- **WHEN** the "Single Cube" model is selected without user modification
- **THEN** the cube renders with material value 224

#### Scenario: Missing single_cube section uses defaults
- **GIVEN** config.ron does not contain a `single_cube` section
- **WHEN** loading the configuration
- **THEN** default values are used (default_material: 224)

### Requirement: Fabric Configuration Section
The config file SHALL contain a `fabric` section with root magnitude, boundary magnitude, surface radius, additive states, and default max depth for fabric model generation.

#### Scenario: Root and boundary magnitude from config (SDF convention)
- **GIVEN** config.ron contains `fabric: (root_magnitude: 0.5, boundary_magnitude: 2.0, surface_radius: 0.8)`
- **WHEN** generating a fabric model
- **THEN** the root quaternion has magnitude 0.5 (|Q| < 1 = inside/solid at origin)
- **AND** magnitude increases with Euclidean distance toward boundary_magnitude
- **AND** surface forms at distance where |Q| = 1.0

#### Scenario: Surface radius controls sphere size
- **GIVEN** config.ron contains `fabric: (surface_radius: 0.6)`
- **WHEN** generating a fabric model
- **THEN** the spherical surface forms at approximately 60% of world half-size from origin

#### Scenario: Additive states from config
- **GIVEN** config.ron contains `fabric: (additive_states: [(rotation: 0.0, magnitude: 0.0), (rotation: 0.1, magnitude: 0.05)])`
- **WHEN** generating a fabric model
- **THEN** depth 0 uses rotation=0.0, magnitude=0.0
- **AND** depth 1 uses rotation=0.1, magnitude=0.05

#### Scenario: Default max depth from config
- **GIVEN** config.ron contains `fabric: (default_max_depth: 5)`
- **WHEN** the fabric model selector is opened
- **THEN** the max depth slider defaults to 5

#### Scenario: Missing fabric section uses defaults
- **GIVEN** config.ron does not contain a `fabric` section
- **WHEN** loading the configuration
- **THEN** default values are used (root_magnitude: 0.5, boundary_magnitude: 2.0, surface_radius: 0.8, additive_states: [(0.0, 0.0)], default_max_depth: 4)

### Requirement: Model Entries Configuration
The config file SHALL contain model entries that can specify either CSM strings or VOX file paths.

#### Scenario: CSM model entry
- **GIVEN** a model entry with `csm: Some("> [224 252 ...]")`
- **WHEN** loading the model
- **THEN** the CSM string is parsed into a `Cube<u8>`

#### Scenario: VOX model entry
- **GIVEN** a model entry with `vox_path: Some("alien_bot1.vox")`
- **WHEN** loading the model
- **THEN** the VOX file is loaded from `assets/models/vox/alien_bot1.vox`

#### Scenario: Fabric model entry
- **GIVEN** a model entry with `fabric: Some((additive_override: [...]))`
- **WHEN** loading the model
- **THEN** a fabric cube is generated with the specified additive states

### Requirement: Configuration Validation
The configuration loader SHALL validate the config file and provide helpful error messages for invalid configurations.

#### Scenario: Invalid RON syntax
- **GIVEN** config.ron contains syntax errors
- **WHEN** the renderer attempts to load the config
- **THEN** a clear error message is displayed indicating the syntax error location

#### Scenario: Missing required fields
- **GIVEN** a model entry missing both `csm` and `vox_path`
- **WHEN** loading the configuration
- **THEN** an error message indicates which model entry is invalid

#### Scenario: Invalid file paths
- **GIVEN** a VOX model entry with a non-existent file path
- **WHEN** attempting to load that model
- **THEN** an error message indicates the file was not found
- **AND** other models remain loadable
