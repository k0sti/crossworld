# Spec: Documentation Structure

## Overview

This specification defines the organization and structure of project documentation to ensure clarity, maintainability, and accessibility for developers.

## ADDED Requirements

### Requirement: Single documentation folder
The project SHALL maintain all documentation in a single `doc/` folder at the repository root.

**Rationale**: Multiple documentation folders (`doc/`, `docs/`) create confusion about where to find or add documentation. A single location simplifies navigation and maintenance.

#### Scenario: Developer looks for documentation
**Given** a developer needs to find project documentation
**When** they look in the repository root
**Then** they should find a single `doc/` folder
**And** no `docs/` folder should exist

#### Scenario: Developer adds new documentation
**Given** a developer creates new documentation
**When** they choose where to place it
**Then** they should have one clear location: `doc/` or a subdirectory within it

### Requirement: Hierarchical documentation organization
Documentation SHALL be organized into clear categories: getting started, architecture, features, and reference.

**Rationale**: Categorical organization helps developers quickly find relevant information based on their needs (learning the system vs looking up specifics).

#### Scenario: Developer needs architecture information
**Given** a developer wants to understand system architecture
**When** they open the `doc/` folder
**Then** they should see an `architecture/` subfolder
**And** that folder should contain system design documents

#### Scenario: Developer looks for feature documentation
**Given** a developer needs to understand a specific feature
**When** they open the `doc/` folder
**Then** they should see a `features/` subfolder
**And** that folder should contain feature-specific guides

#### Scenario: Developer needs technical reference
**Given** a developer needs technical reference information
**When** they open the `doc/` folder
**Then** they should see a `reference/` subfolder
**And** that folder should contain technical specifications

### Requirement: Documentation index
The documentation SHALL include an index file that provides navigation and context.

**Rationale**: New developers need a starting point and overview of available documentation. An index prevents documentation from being a "pile of files."

#### Scenario: New developer starts with documentation
**Given** a new developer clones the repository
**When** they open `doc/README.md`
**Then** they should see an overview of available documentation
**And** clear links to getting started guides
**And** links to all major documentation categories

#### Scenario: Developer searches for specific topic
**Given** a developer knows roughly what topic they need
**When** they read `doc/README.md`
**Then** they should be able to quickly identify which section contains that topic
**And** follow a link to the relevant document

### Requirement: No obsolete documentation
The documentation SHALL NOT contain outdated files describing removed features, fixed bugs, or completed refactors.

**Rationale**: Obsolete documentation confuses developers about current state and increases maintenance burden.

#### Scenario: Developer reads about a feature
**Given** a developer reads documentation about a feature
**When** they look at the corresponding code
**Then** the feature should still exist in the codebase
**And** the documentation should accurately describe current implementation

#### Scenario: Documentation mentions a bug fix
**Given** documentation describes a bug and its fix
**When** the bug has been fixed
**Then** that documentation should be removed
**And** the fix should be reflected in architectural docs if architecturally significant

### Requirement: Single authoritative source per topic
Each technical topic SHALL have exactly one primary documentation file.

**Rationale**: Multiple files covering the same topic lead to conflicting information, duplication, and maintenance burden. Developers can't know which file is authoritative.

#### Scenario: Developer reads about avatar system
**Given** a developer wants to understand the avatar system
**When** they search documentation for "avatar"
**Then** they should find one primary document covering the avatar system
**And** that document should cover design, implementation, and usage
**And** other files may reference it but not duplicate its content

#### Scenario: Developer updates documentation
**Given** the avatar system changes
**When** a developer needs to update documentation
**Then** they should have exactly one file to update
**And** not need to check multiple files for consistency

### Requirement: Core guides at documentation root
Essential getting-started guides SHALL be at the `doc/` root, not buried in subdirectories.

**Rationale**: New developers need to quickly find setup and conventions without navigating deep hierarchies.

#### Scenario: Developer sets up development environment
**Given** a new developer needs to set up their environment
**When** they open the `doc/` folder
**Then** they should immediately see `QUICKSTART.md`
**And** `EDITOR_SETUP.md`
**And** `CONVENTIONS.md`
**Without** needing to navigate subdirectories

### Requirement: Consistent internal linking
Documentation SHALL use correct relative paths for internal links and reference moved/consolidated files by their new locations.

**Rationale**: Broken links frustrate developers and make documentation feel abandoned or untrustworthy.

#### Scenario: Developer follows a documentation link
**Given** a documentation file contains a link to another doc
**When** the developer clicks or follows that link
**Then** the target file should exist at that path
**And** the link should work in both file browsers and rendered markdown

#### Scenario: Documentation reorganization
**Given** documentation files are moved or consolidated
**When** the reorganization is complete
**Then** all internal links should be updated to new paths
**And** no links should point to deleted files

### Requirement: Architecture documentation separation
Architecture documentation SHALL be separated from feature documentation and reference material.

**Rationale**: System design and feature usage are different concerns. Developers understanding the system need architectural docs; developers using features need feature guides.

#### Scenario: Developer understands the system
**Given** a developer wants to understand how the system works
**When** they read `doc/architecture/` files
**Then** they should learn about system components, data flow, and design decisions
**And** not be overwhelmed with feature usage details

#### Scenario: Developer implements a new feature
**Given** a developer is implementing a new feature
**When** they need to understand related architecture
**Then** they should read architecture docs
**And** when they document the feature
**Then** they should create feature documentation that references but doesn't duplicate architecture

### Requirement: No temporary planning documents
The documentation SHALL NOT contain temporary planning files created for implementing specific changes.

**Rationale**: Planning documents are useful during development but become noise afterward. OpenSpec system provides proper home for change proposals.

#### Scenario: Refactor is completed
**Given** a refactor has been completed
**When** the refactor had a planning document
**Then** the planning document should be removed
**And** architecturally significant decisions should be captured in architecture docs or OpenSpec
**And** implementation details should be in code and tests

#### Scenario: Developer finds a dated planning doc
**Given** a documentation file has a date in its name (e.g., `2025-11-10_Feature.md`)
**When** the work described in that file is complete
**Then** that file should be deleted
**And** any important context should be in permanent documentation

## MODIFIED Requirements

None - this is new documentation structure specification.

## REMOVED Requirements

None - no prior documentation structure spec existed.

## Implementation Notes

### Folder Structure

```
doc/
├── README.md              # Documentation index and navigation
├── QUICKSTART.md          # First-run setup
├── CONVENTIONS.md         # Coding standards
├── EDITOR_SETUP.md        # Development environment
│
├── architecture/          # System design documents
│   ├── overview.md        # High-level architecture
│   ├── voxel-system.md    # Voxel engine and CSM format
│   ├── physics.md         # Physics integration
│   ├── raycast.md         # Raycasting system
│   └── rendering.md       # Rendering pipeline
│
├── features/              # Feature-specific documentation
│   ├── avatar-system.md   # Avatar design, physics, animation
│   ├── voice-chat.md      # MoQ voice chat setup and usage
│   └── nostr-integration.md # Nostr features and worldtool
│
└── reference/             # Technical references
    ├── project-structure.md  # Repository organization
    ├── build-system.md       # Build process and justfile
    └── materials.md          # Material system reference
```

### Content Guidelines

**Architecture docs should**:
- Explain system design and component interactions
- Describe data flow and boundaries
- Document design decisions and tradeoffs
- Reference code, not duplicate it

**Feature docs should**:
- Explain how to use the feature
- Cover setup and configuration
- Provide examples and common patterns
- Reference architecture docs for design context

**Reference docs should**:
- Provide technical specifications
- Document formats and schemas
- List commands and options
- Serve as quick lookup resources

**Core guides should**:
- Get developers productive quickly
- Cover setup and conventions
- Be at root for easy discovery
- Link to deeper docs as needed

### Maintenance

- Delete documentation when features are removed
- Delete planning docs when work is complete
- Consolidate when multiple docs cover same topic
- Update references when files move
- Keep `openspec/project.md` documentation section in sync
