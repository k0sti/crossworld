# Specification: Migrate OpenSpec to Auto-Claude Task System

## Overview

Migrate active planning documentation from the legacy `openspec/` folder into the new `.auto-claude/specs/` task system, then deprecate the old openspec files. The openspec system contains 11 active change proposals with detailed tasks, specs, and design documents that need to be extracted and converted to the new JSON-based task format.

## Workflow Type

**Type**: feature

**Rationale**: This is a content migration task that creates new task entries in a different format. It requires reading, transforming, and writing structured data rather than bug fixing or refactoring code.

## Task Scope

### Services Involved
- **openspec** (source) - Legacy specification system with markdown-based proposals
- **.auto-claude** (target) - New JSON-based task management system

### This Task Will:
- [ ] Catalog all 11 active openspec changes and their task status
- [ ] Create new task entries in `.auto-claude/specs/` for each actionable item
- [ ] Extract incomplete tasks from openspec tasks.md files
- [ ] Preserve context and requirements from proposal.md files
- [ ] Mark or remove deprecated openspec files after migration

### Out of Scope:
- Migrating archived openspec changes (already completed work)
- Modifying the openspec CLI tooling (`.claude/commands/openspec/`)
- Creating new features based on the specifications
- Implementing any of the planned changes

## Source System Analysis

### OpenSpec Active Changes (11 total)

Located in `openspec/changes/`:

| Change ID | Status | Summary |
|-----------|--------|---------|
| `add-fabric-model-generation` | Active | Procedural voxel generation with quaternion fields |
| `add-function-cube-type` | Active | Dynamic material expressions with CPU/GPU backends |
| `add-terrain-composite-collider` | Active | TypedSimdCompositeShape for terrain collision |
| `create-bevy-physics-prototype` | Active | Standalone Bevy physics validation app |
| `create-bevy-voxel-editor` | Active | Native Bevy-based voxel editor |
| `create-proto-gl-physics-viewer` | Active | Lightweight egui+GL physics viewer |
| `implement-gpu-raytracer` | Active | GPU shader-based octree raytracing |
| `improve-renderer-quality` | Partial | Mesh renderer fixes (some tasks complete) |
| `optimize-world-collision` | Partial | World collision strategies (some tasks complete) |
| `refactor-renderer-interface` | Active | Unified renderer trait interface |

### OpenSpec File Structure (per change)

```
openspec/changes/[change-id]/
├── proposal.md     # Why, what, impact (requirements source)
├── tasks.md        # Implementation checklist with [ ] / [x] status
├── design.md       # Technical decisions (optional)
└── specs/          # Capability delta specifications
    └── [capability]/
        └── spec.md
```

## Target System Analysis

### Auto-Claude Task Structure

Located in `.auto-claude/specs/`:

```
.auto-claude/specs/[task-id]/
├── requirements.json       # Task description, workflow type
├── context.json           # Files to modify/reference
├── implementation_plan.json # Phases and subtasks
├── project_index.json     # Copied from parent
└── spec.md               # Full specification
```

### Example Implementation Plan Format

```json
{
  "feature": "Task Name",
  "description": "Task description",
  "created_at": "ISO timestamp",
  "updated_at": "ISO timestamp",
  "status": "pending|in_progress|completed",
  "phases": [
    {
      "name": "Phase 1",
      "status": "pending",
      "subtasks": [
        {
          "id": "1.1",
          "description": "Subtask description",
          "status": "pending"
        }
      ]
    }
  ]
}
```

## Files to Modify

| File | Service | What to Change |
|------|---------|---------------|
| `.auto-claude/specs/003-*/` | auto-claude | Create new task directories for each active openspec change |
| `openspec/changes/*/proposal.md` | openspec | Read for requirements extraction |
| `openspec/changes/*/tasks.md` | openspec | Read for subtask extraction |

## Files to Reference

These files show patterns to follow:

| File | Pattern to Copy |
|------|----------------|
| `.auto-claude/specs/001-physics-optimization/implementation_plan.json` | Task JSON structure |
| `.auto-claude/specs/001-physics-optimization/requirements.json` | Requirements format |
| `openspec/AGENTS.md` | Understanding openspec workflow |
| `openspec/project.md` | Project context and conventions |

## Patterns to Follow

### Task ID Naming

From existing `.auto-claude/specs/`:

- Format: `NNN-kebab-case-name`
- Example: `001-physics-optimization`, `002-openspec-tasks`
- Next available: `003-*` through `013-*` for 11 active changes

### Requirements JSON Pattern

```json
{
  "task_description": "Brief description of what needs to be done",
  "workflow_type": "feature|refactor|investigation"
}
```

### Implementation Plan Pattern

Extract from openspec tasks.md checkbox items:
- `- [ ]` -> `"status": "pending"`
- `- [x]` -> `"status": "completed"`

Group by phase headings (## Phase N) in tasks.md.

## Requirements

### Functional Requirements

1. **Extract Active Tasks**
   - Description: Parse all 11 active openspec changes and extract incomplete tasks
   - Acceptance: Each openspec change maps to one `.auto-claude/specs/` directory

2. **Preserve Task Status**
   - Description: Maintain completion status of partially-done changes
   - Acceptance: Tasks marked `[x]` in openspec appear as `"completed"` in JSON

3. **Capture Requirements Context**
   - Description: Extract motivation, goals, and impact from proposal.md
   - Acceptance: Each task has meaningful `task_description` from proposal

4. **Create Valid JSON**
   - Description: Generated files must be valid JSON matching existing patterns
   - Acceptance: All JSON files parse without errors

5. **Deprecate Old Files**
   - Description: After successful migration, mark or remove openspec changes
   - Acceptance: Clear indication that migration is complete

### Edge Cases

1. **Partially Complete Changes** - Changes like `improve-renderer-quality` and `optimize-world-collision` have mixed [x]/[ ] status. Extract only pending tasks but note completed work in context.

2. **Empty Task Phases** - Some phases may be entirely complete. Skip them or include as completed context.

3. **Design Documents** - Include `design.md` content in spec.md if present, not as separate file.

4. **Delta Specs** - Openspec delta specs (`specs/*/spec.md` under changes) are capability-specific. Include as context in new spec.md.

## Implementation Notes

### DO
- Follow the task ID numbering pattern (003, 004, ... 013)
- Parse markdown checkboxes accurately (`- [ ]` vs `- [x]`)
- Preserve the hierarchical structure of phases/subtasks
- Create one auto-claude task per openspec change
- Include sufficient context from proposal.md for future implementation

### DON'T
- Create tasks for archived openspec changes
- Remove openspec files before verifying migration success
- Modify the openspec CLI commands (`.claude/commands/openspec/`)
- Attempt to implement any of the planned changes during migration

## Migration Mapping

| OpenSpec Change | New Task ID | Priority |
|-----------------|-------------|----------|
| `optimize-world-collision` | `003-world-collision-optimization` | High (in progress) |
| `improve-renderer-quality` | `004-renderer-quality` | High (in progress) |
| `implement-gpu-raytracer` | `005-gpu-raytracer` | Medium |
| `refactor-renderer-interface` | `006-renderer-interface` | Medium |
| `create-proto-gl-physics-viewer` | `007-proto-gl-viewer` | Low (superseded?) |
| `create-bevy-physics-prototype` | `008-bevy-physics` | Low |
| `create-bevy-voxel-editor` | `009-bevy-editor` | Low |
| `add-fabric-model-generation` | `010-fabric-model` | Medium |
| `add-function-cube-type` | `011-function-cube` | Medium |
| `add-terrain-composite-collider` | `012-terrain-collider` | Medium |

## Development Environment

### Relevant Directories

```bash
# Source: openspec active changes
ls openspec/changes/

# Target: auto-claude specs
ls .auto-claude/specs/

# Reference: existing task
cat .auto-claude/specs/001-physics-optimization/implementation_plan.json
```

### Validation Commands

```bash
# Verify JSON validity
python3 -c "import json; json.load(open('.auto-claude/specs/003-*/implementation_plan.json'))"

# Count migrated tasks
ls -d .auto-claude/specs/*/ | wc -l

# Verify no incomplete openspec changes remain
find openspec/changes -maxdepth 1 -type d ! -name archive | wc -l
```

## Success Criteria

The task is complete when:

1. [ ] All 11 active openspec changes have corresponding `.auto-claude/specs/` directories
2. [ ] Each new task has valid `requirements.json` with task description
3. [ ] Each new task has valid `implementation_plan.json` with extracted subtasks
4. [ ] Partially complete tasks (improve-renderer-quality, optimize-world-collision) have accurate status
5. [ ] Each new task has `spec.md` with context from proposal/design docs
6. [ ] Openspec active changes are marked as migrated or removed
7. [ ] No console errors when loading JSON files
8. [ ] Migration is reversible (old files backed up or still accessible)

## QA Acceptance Criteria

**CRITICAL**: These criteria must be verified by the QA Agent before sign-off.

### Unit Tests
| Test | File | What to Verify |
|------|------|----------------|
| JSON Validity | All `implementation_plan.json` | Valid JSON, required fields present |
| Task Count | `.auto-claude/specs/` | 11+ new task directories created |
| Status Mapping | Migrated tasks | `[x]` -> completed, `[ ]` -> pending |

### Integration Tests
| Test | Services | What to Verify |
|------|----------|----------------|
| Cross-reference | openspec -> auto-claude | Each openspec change maps to one task |
| Content Preservation | proposal.md -> spec.md | Key requirements captured |

### End-to-End Tests
| Flow | Steps | Expected Outcome |
|------|-------|------------------|
| Migration Completeness | 1. List openspec changes 2. List auto-claude tasks | All active changes migrated |
| Deprecation Verification | 1. Check openspec/changes 2. Verify empty or marked | No active unmigrated changes |

### Browser Verification (if frontend)
N/A - This is a documentation/metadata migration task.

### Database Verification (if applicable)
N/A - No database involved.

### QA Sign-off Requirements
- [ ] All JSON files are valid and parseable
- [ ] Task count matches expected (11 new tasks minimum)
- [ ] Status mapping is accurate for partial changes
- [ ] Spec.md files contain meaningful context
- [ ] No duplicate task IDs
- [ ] Openspec changes properly deprecated/marked
- [ ] Migration can be verified by listing both systems
