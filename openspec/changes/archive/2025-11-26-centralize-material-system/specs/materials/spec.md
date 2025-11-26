## ADDED Requirements
### Requirement: Material System
The system SHALL provide a centralized material definition in the `cube` crate.

#### Scenario: Material Lookup
- **WHEN** a material index is provided
- **THEN** the system returns the correct RGB color
- **AND** indices 0-127 return defined palette colors
- **AND** indices 128-255 return R2G3B2 encoded colors
