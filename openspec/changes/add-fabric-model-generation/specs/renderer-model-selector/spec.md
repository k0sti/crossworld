## ADDED Requirements

### Requirement: Model Selector Page Layout
The renderer SHALL display a model selector panel as a side panel instead of a top-bar dropdown, with models organized into categories.

#### Scenario: Model categories displayed in side panel
- **GIVEN** the renderer application is running
- **WHEN** the user views the main window
- **THEN** a side panel shows model categories: "Single Cube", "VOX Models", "CSM Models", "Fabric Models"
- **AND** render views (CPU, GL, GPU, BCF, Mesh, Diff) occupy the center area

#### Scenario: Category sections are collapsible
- **GIVEN** the model selector panel is visible
- **WHEN** the user clicks on a category header (e.g., "VOX Models")
- **THEN** the category expands to show available models
- **AND** clicking again collapses the category

### Requirement: Single Cube Category Parameters
The Single Cube category SHALL provide material selection controls within the category section.

#### Scenario: Material selector in Single Cube category
- **GIVEN** the "Single Cube" category is expanded
- **WHEN** the user views the category contents
- **THEN** a material value slider (0-255) is displayed
- **AND** preset material buttons are available (e.g., "Red", "Error Colors")

#### Scenario: Changing material updates render
- **GIVEN** the "Single Cube" category is selected
- **WHEN** the user changes the material value
- **THEN** all render views update to show the new material color

### Requirement: Fabric Model Category Parameters
The Fabric Models category SHALL provide editable parameters for fabric generation including additive states and max depth.

#### Scenario: Additive states array editor
- **GIVEN** the "Fabric Models" category is expanded
- **WHEN** the user views the category contents
- **THEN** an editable array of additive state values is displayed
- **AND** each value represents rotation magnitude for that depth level

#### Scenario: Max depth slider
- **GIVEN** the "Fabric Models" category is expanded
- **WHEN** the user views the category contents
- **THEN** a max depth slider (range 1-8) is displayed
- **AND** changing the slider regenerates the fabric model at the new depth

#### Scenario: Fabric parameter changes trigger regeneration
- **GIVEN** a fabric model is currently displayed
- **WHEN** the user modifies any fabric parameter (additive state or max depth)
- **THEN** the fabric cube is regenerated with new parameters
- **AND** all render views update to show the new geometry

### Requirement: Max Depth Rendering Parameter
The rendering system SHALL support a max_depth parameter that treats all nodes at that depth as leaves regardless of octree structure.

#### Scenario: Max depth limits traversal
- **GIVEN** a cube with actual depth 6
- **AND** max_depth rendering parameter set to 3
- **WHEN** rendering the cube
- **THEN** traversal stops at depth 3
- **AND** nodes at depth 3 are rendered as solid voxels

#### Scenario: Max depth applies to all renderers
- **GIVEN** max_depth parameter set to 4
- **WHEN** rendering with CPU, GL, BCF, and Mesh renderers
- **THEN** all renderers produce consistent output at depth 4 resolution

### Requirement: VOX and CSM Model Categories
The VOX Models and CSM Models categories SHALL list available models from the configuration file.

#### Scenario: VOX models listed from config
- **GIVEN** config.ron contains VOX model entries with vox_path
- **WHEN** the "VOX Models" category is expanded
- **THEN** each VOX model is listed by its display name
- **AND** clicking a model loads it into the render views

#### Scenario: CSM models listed from config
- **GIVEN** config.ron contains CSM model entries with csm strings
- **WHEN** the "CSM Models" category is expanded
- **THEN** each CSM model is listed by its display name
- **AND** clicking a model loads it into the render views
