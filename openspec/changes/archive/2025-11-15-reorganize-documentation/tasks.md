# Tasks: Reorganize Documentation

## Phase 1: Cleanup - Delete Obsolete Files

### Task 1.1: Delete completed refactor documentation
- [x] Delete `doc/2025-11-10_AppRefactoring.md`
- [x] Delete `doc/cube-refactor.md`
- [x] Delete `doc/reimplement.md`
- [x] Verify: Run `git status` to confirm deletions staged

**Validation**: Files no longer exist, no broken references in remaining docs

### Task 1.2: Delete bug fix documentation
- [x] Delete `doc/subdepth-fix.md`
- [x] Delete `doc/subdepth-fix-solution.md`
- [x] Verify: Search for references: `rg -l "subdepth" doc/`

**Validation**: No references to subdepth in remaining documentation

### Task 1.3: Delete removed feature and outdated UI docs
- [x] Delete `doc/ideas/emoji-hash.md`
- [x] Delete `doc/ResponsivePanel.md`
- [x] Remove empty `doc/ideas/` folder
- [x] Verify: `ls doc/ideas/` returns "No such file"

**Validation**: Obsolete content removed, no empty folders remain

## Phase 2: Structure - Create New Folders and Index

### Task 2.1: Create documentation folder structure
- [x] Create `doc/architecture/` folder
- [x] Create `doc/features/` folder
- [x] Create `doc/reference/` folder
- [x] Verify: `ls -d doc/*/` shows new folders

**Validation**: All three new folders exist

### Task 2.2: Create documentation index
- [x] Create `doc/README.md` with:
  - Documentation overview and navigation
  - "Getting Started" section pointing to QUICKSTART.md
  - Architecture section linking to architecture/
  - Features section linking to features/
  - Reference section linking to reference/
  - Link to `openspec/` for change proposals
- [x] Verify: File exists and has all sections

**Validation**: `doc/README.md` provides clear entry point and navigation

## Phase 3: Migration - Move and Consolidate Content

### Task 3.1: Move raycast documentation
- [x] Move `docs/raycast.md` → `doc/architecture/raycast.md`
- [x] Remove empty `docs/` folder
- [x] Verify: `ls docs/` returns "No such file"

**Validation**: `docs/` folder removed, raycast.md in correct location

### Task 3.2: Consolidate voxel/CSM documentation
- [x] Create `doc/architecture/voxel-system.md` combining:
  - `doc/cube-script-model.md` - CSM format overview
  - `doc/csm-examples.md` - Format examples
  - `doc/csm-save-load.md` - Save/load mechanics
  - Sections: CSM Format, Octree Structure, Examples, Save/Load
- [x] Delete original files after consolidation
- [x] Verify: Search old filenames: `ls doc/csm-*.md doc/cube-script-model.md`

**Validation**: Single voxel-system.md exists, old files deleted, no duplicate content

### Task 3.3: Consolidate avatar system documentation
- [x] Create `doc/features/avatar-system.md` combining:
  - `doc/vox-avatar-system.md` - System design
  - `doc/avatar_model.md` - Model format
  - `doc/implementation-avatars.md` - Implementation approach
  - `doc/avatar-implementation-summary.md` - Summary
  - `doc/avatar-physics.md` - Physics integration
  - `doc/avatar-physics-implementation-summary.md` - Physics summary
  - Sections: Overview, Architecture, Model Format, Physics, Animation
- [x] Delete original avatar files after consolidation
- [x] Verify: `ls doc/avatar*.md doc/vox-avatar*.md` returns no results

**Validation**: Single avatar-system.md exists, 6 old files deleted

### Task 3.4: Consolidate voice chat documentation
- [x] Create `doc/features/voice-chat.md` from `doc/voice-moq/`:
  - Extract setup from `moq-relay-setup.md`
  - Extract debugging from `moq-debugging-guide.md`
  - Extract overview from `moq.md`, `voicechat.md`
  - Extract implementation notes from `moq-implementation-summary.md`
  - Skip `moq-innpub.md` (protocol internals, too detailed)
  - Sections: Overview, Setup, Debugging, Architecture
- [x] Delete `doc/voice-moq/` folder after consolidation
- [x] Verify: `ls doc/voice-moq/` returns "No such file"

**Validation**: Single voice-chat.md exists, voice-moq/ folder removed

### Task 3.5: Move existing documentation to new structure
- [x] Move `doc/physics.md` → `doc/architecture/physics.md`
- [x] Move `doc/materials.md` → `doc/reference/materials.md`
- [x] Move `doc/project-structure.md` → `doc/reference/project-structure.md`
- [x] Keep at root: `QUICKSTART.md`, `CONVENTIONS.md`, `EDITOR_SETUP.md`
- [x] Verify: Files in correct locations

**Validation**: Core guides at root, technical docs organized by category

## Phase 4: Creation - Write New Documentation

### Task 4.1: Create architecture overview
- [x] Create `doc/architecture/overview.md` with:
  - System components diagram (text/ASCII)
  - Rust/WASM/TypeScript architecture
  - Data flow between subsystems
  - Links to detailed architecture docs
  - Reference to `openspec/project.md` for tech stack
- [x] Verify: File covers high-level system design

**Validation**: Overview provides architectural context without duplicating project.md

### Task 4.2: Create rendering documentation
- [x] Create `doc/architecture/rendering.md` with:
  - Three.js integration
  - Shader system overview
  - CPU vs GPU rendering paths
  - Reference to renderer crate
- [x] Verify: Covers rendering architecture

**Validation**: Rendering pipeline documented, links to code

### Task 4.3: Create Nostr integration documentation
- [x] Create `doc/features/nostr-integration.md` with:
  - Nostr identity system
  - worldtool CLI usage
  - NIP-33 live events
  - Server/player discovery
  - Extract relevant content from `doc/server.md` and `doc/model_store_event.md`
- [x] Verify: Covers all Nostr features

**Validation**: Nostr features documented in one place

### Task 4.4: Create build system documentation
- [x] Create `doc/reference/build-system.md` with:
  - justfile commands explained
  - WASM build process (dev vs release)
  - TypeScript build with Vite
  - Parallel builds explanation
  - Common build issues and solutions
- [x] Verify: All justfile commands documented

**Validation**: Build process clearly documented for new developers

## Phase 5: Updates - Fix Existing Documentation

### Task 5.1: Update packages/app/README.md
- [x] Remove reference to non-existent `/packages/quad`
- [x] Update to reference actual WASM packages: `wasm-world`, `wasm-cube`, `wasm-physics`
- [x] Add link to main documentation: `../../doc/README.md`
- [x] Verify: No broken references, current package names

**Validation**: README reflects current project structure

### Task 5.2: Update doc/reference/project-structure.md
- [x] Remove `ref/` directory reference (no longer used)
- [x] Add missing crates: `renderer`, `assets`, `worldtool`
- [x] Update WASM packages list
- [x] Ensure structure matches current reality
- [x] Verify: `ls -d crates/*/ packages/*/` matches documented structure

**Validation**: Project structure documentation is accurate

### Task 5.3: Update openspec/project.md documentation section
- [x] Update "Documentation Files" section (lines 247-269) to reflect new structure
- [x] Update paths to moved files
- [x] Add new documentation files
- [x] Remove references to deleted files
- [x] Verify: All file paths in project.md are valid

**Validation**: `openspec/project.md` references correct documentation files

### Task 5.4: Consider design-master.md disposition
- [x] Review `doc/design-master.md` content
- [x] Decision: Keep as-is, rename to `architecture-decisions.md`, or merge into overview.md
- [x] If renaming: Update `openspec/project.md` reference
- [x] If merging: Extract unique content into overview.md, delete original
- [x] Verify: No duplicate content, clear purpose

**Validation**: Design decisions have single authoritative location

## Phase 6: Validation - Verify Documentation Quality

### Task 6.1: Verify all internal links work
- [x] Check links in `doc/README.md`
- [x] Check cross-references in consolidated docs
- [x] Check `openspec/project.md` documentation links
- [x] Fix any broken links found
- [x] Verify: `rg '\[.*\]\(doc/' openspec/ doc/` shows valid paths only

**Validation**: No broken internal documentation links

### Task 6.2: Verify no duplicate content
- [x] Review consolidated files for redundancy
- [x] Remove duplicate information
- [x] Ensure each topic has one authoritative source
- [x] Add cross-references instead of duplicating
- [x] Verify: Each architectural concept documented once

**Validation**: No significant content duplication across docs

### Task 6.3: Verify documentation completeness
- [x] Check that all major features are documented
- [x] Check that all architecture components covered
- [x] Verify getting started path is clear
- [x] Ensure all build commands documented
- [x] Verify: New developer can navigate from README to needed info

**Validation**: Documentation covers essential project information

### Task 6.4: Final structure verification
- [x] Run: `tree doc/ -L 2` (or `find doc/ -type f` if no tree)
- [x] Verify structure matches proposal
- [x] Check no obsolete files remain
- [x] Check all new files created
- [x] Verify: Documentation structure is clean and organized

**Validation**: Final structure matches planned organization

## Dependencies

- **Phase 2** depends on **Phase 1** (cleanup before creating structure)
- **Phase 3** depends on **Phase 2** (folders must exist before moving files)
- **Phase 4** can run in parallel with **Phase 3** (different files)
- **Phase 5** depends on **Phase 3** (update references after moves)
- **Phase 6** depends on all previous phases (final validation)

## Parallelizable Work

- Tasks 3.2, 3.3, 3.4 can be done in parallel (different file consolidations)
- Tasks 4.1, 4.2, 4.3, 4.4 can be done in parallel (different new files)
- Tasks 5.1, 5.2, 5.4 can be done in parallel (different updates)

## Success Metrics

- **0 obsolete files** in doc/
- **1 doc folder** (not 2)
- **3 category folders** (architecture, features, reference)
- **1 clear entry point** (doc/README.md)
- **0 broken links** in documentation
- **100% major features documented**
