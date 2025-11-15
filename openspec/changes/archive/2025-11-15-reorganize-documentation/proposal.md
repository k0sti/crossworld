# Change: Reorganize and Clean Up Documentation

## Why

The project documentation is scattered across `doc/` and `docs/` folders with a mix of:
- **Outdated documentation**: Files describing removed features (emoji-hash), old bugs (subdepth-fix), and completed refactors (cube-refactor)
- **Temporary planning docs**: Time-stamped files meant for implementation planning (2025-11-10_AppRefactoring.md)
- **Inconsistent organization**: Two separate doc folders (`doc/` and `docs/`), mixed organization within
- **Duplicate/overlapping content**: Multiple files covering similar topics (avatar system has 4+ files)
- **Missing structure**: No clear documentation hierarchy or entry point (no root README.md)
- **Outdated references**: `packages/app/README.md` references non-existent packages

The current state makes it difficult for developers to:
- Find authoritative information
- Understand which docs are current vs historical
- Navigate the documentation structure
- Know what to read first when joining the project

**Scope**: Documentation cleanup and reorganization only - no code changes, spec changes, or feature work.

**Approach**: Audit, delete obsolete files, consolidate duplicates, establish clear hierarchy.

## What Changes

### Delete Outdated/Temporary Files
- `doc/2025-11-10_AppRefactoring.md` - Temporary planning doc, implementation completed
- `doc/cube-refactor.md` - Completed refactor, context captured in project.md
- `doc/reimplement.md` - Completed work from world size refactor
- `doc/subdepth-fix.md` - Fixed bug documentation, no longer relevant
- `doc/subdepth-fix-solution.md` - Fixed bug solution, no longer relevant
- `doc/ideas/emoji-hash.md` - Feature removed from codebase, purely historical
- `doc/ResponsivePanel.md` - Old UI implementation detail, superseded by current code

### Consolidate Documentation Folders
- **Merge `docs/` into `doc/`** - Single documentation folder at `doc/`
  - Move `docs/raycast.md` → `doc/architecture/raycast.md`
- **Remove `docs/` folder** after migration

### Reorganize by Category
Create clear structure within `doc/`:

```
doc/
├── README.md                  # NEW: Documentation index and getting started
├── QUICKSTART.md              # Keep: First-run setup guide
├── CONVENTIONS.md             # Keep: Coding standards (authoritative)
├── EDITOR_SETUP.md            # Keep: Development environment
│
├── architecture/              # NEW: System design documents
│   ├── overview.md            # NEW: High-level architecture
│   ├── voxel-system.md        # Consolidated from CSM files
│   ├── physics.md             # Existing
│   ├── raycast.md             # Moved from docs/
│   └── rendering.md           # NEW: Consolidated rendering info
│
├── features/                  # NEW: Feature-specific documentation
│   ├── avatar-system.md       # Consolidated from 4+ avatar files
│   ├── voice-chat.md          # Consolidated from voice-moq/
│   └── nostr-integration.md   # NEW: Nostr features and worldtool
│
└── reference/                 # NEW: Technical references
    ├── project-structure.md   # Keep, potentially update
    ├── build-system.md        # NEW: Build, tasks, justfile
    └── materials.md           # Keep: Material system reference
```

### Consolidate Redundant Content

**Avatar System** (4+ files → 1):
- `doc/vox-avatar-system.md` (2KB) - Design concept
- `doc/avatar_model.md` (5KB) - Model format
- `doc/implementation-avatars.md` (26KB) - Implementation plan
- `doc/avatar-implementation-summary.md` (6KB) - Summary
- `doc/avatar-physics.md` (37KB) - Physics integration
- `doc/avatar-physics-implementation-summary.md` (10KB) - Summary
→ Consolidate into `doc/features/avatar-system.md` with clear sections

**Voice Chat** (6 files → 1):
- `doc/voice-moq/` subfolder with 6 files
→ Consolidate into `doc/features/voice-chat.md`, keep essential setup/debugging

**Voxel/CSM** (3 files → 1):
- `doc/cube-script-model.md` - Format overview
- `doc/csm-examples.md` - Examples
- `doc/csm-save-load.md` - Save/load
→ Consolidate into `doc/architecture/voxel-system.md`

### Create New Documentation

**doc/README.md** - Documentation index:
- Quick navigation to all doc sections
- "Start here" guidance for new developers
- Links to external resources (OpenSpec, MoQ, Nostr)

**doc/architecture/overview.md** - High-level architecture:
- System components diagram
- Rust ↔ WASM ↔ TypeScript boundaries
- Data flow between subsystems
- Reference `openspec/project.md` for tech stack details

**doc/architecture/rendering.md** - Rendering pipeline:
- Three.js integration
- Shader system
- CPU vs GPU rendering paths

**doc/features/nostr-integration.md** - Nostr features:
- Identity system
- worldtool CLI
- Live events (NIP-33)
- Discovery

**doc/reference/build-system.md** - Build documentation:
- Justfile commands explained
- WASM build process
- Development vs production builds
- Common build issues

### Update Existing Files

**packages/app/README.md**:
- Remove reference to non-existent `/packages/quad`
- Update to reference current WASM packages
- Add link to main documentation

**doc/project-structure.md**:
- Update to reflect current crate structure
- Remove `ref/` directory (no longer used)
- Add missing crates (renderer, assets)

**doc/design-master.md**:
- Consider renaming to `architecture-decisions.md` for clarity
- Or merge into `doc/architecture/overview.md`

## Impact

### Affected Specs
- **NEW**: `documentation-structure` - Specification for project documentation organization

### Affected Files

**Deleted** (7 files):
- `doc/2025-11-10_AppRefactoring.md`
- `doc/cube-refactor.md`
- `doc/reimplement.md`
- `doc/subdepth-fix.md`
- `doc/subdepth-fix-solution.md`
- `doc/ideas/emoji-hash.md`
- `doc/ResponsivePanel.md`

**Moved**:
- `docs/raycast.md` → `doc/architecture/raycast.md`

**Created** (8 new files):
- `doc/README.md`
- `doc/architecture/overview.md`
- `doc/architecture/voxel-system.md`
- `doc/architecture/rendering.md`
- `doc/features/avatar-system.md`
- `doc/features/voice-chat.md`
- `doc/features/nostr-integration.md`
- `doc/reference/build-system.md`

**Updated** (3 files):
- `packages/app/README.md`
- `doc/project-structure.md`
- `openspec/project.md` - Update documentation section

**Removed** (empty after migration):
- `docs/` folder
- `doc/ideas/` folder (only contained emoji-hash.md)
- `doc/voice-moq/` folder (content consolidated)

### Not Affected
- All code files (no code changes)
- All specs in `openspec/specs/` (no spec changes)
- `CLAUDE.md` and `openspec/` system files
- Build system (`justfile`, `package.json`, etc.)

### Dependencies
- No external dependencies
- No code changes required
- Documentation-only changes

### Breaking Changes
None - documentation changes don't affect code

### Success Criteria
- Single `doc/` folder with clear hierarchy
- No outdated/temporary files remaining
- Each topic has one authoritative document
- `doc/README.md` provides clear entry point
- All internal doc links work correctly
- References in `openspec/project.md` updated

## Migration Notes

### Content Consolidation Strategy

When consolidating multiple files into one:
1. **Identify authoritative content** - Most detailed/recent version
2. **Extract unique information** - Don't duplicate what's in code
3. **Remove implementation details** - Link to code instead
4. **Keep architecture/design rationale** - Why decisions were made
5. **Remove obsolete information** - No outdated API examples

### Link Updates

After reorganization, update links in:
- `openspec/project.md` - Documentation Files section
- `doc/README.md` - Internal cross-references
- Any remaining docs that reference moved files

### Archive vs Delete

**Delete** (not archive):
- Bug fix documentation (subdepth-fix*)
- Temporary planning docs (dated files)
- Removed feature docs (emoji-hash)
- Completed refactor plans (cube-refactor, reimplement)

**Keep in consolidated form**:
- Architecture decisions
- Feature designs
- System overviews
