# Vibe-Kanban Review Workflow Implementation

This document describes the implementation plan for adding an interactive review workflow to vibe-kanban, allowing task agents to trigger visual review sessions using the Crossworld `--review` argument.

## Overview

When a task reaches the "ready for review" stage, the agent should be able to launch an interactive review session using the Crossworld testbed's `--review <doc>` argument. This displays a visual overlay where the user can read the review document and provide feedback via a comment field. The comment is written to stdout when the user exits, allowing the agent to parse and act on the response.

**Key Requirements:**
- Reviewer can return **multiple workflow commands** in a single response
- At least one command is **always required**
- Agent **must execute all commands** using vibe-kanban MCP tools or equivalent

## Current Implementation Analysis

### Crossworld `--review` Argument

**Location:** `crates/testbed/src/main.rs`, `crates/app/src/review_overlay.rs`, `crates/app/src/runner.rs`

**How it works:**
1. CLI parses `--review <path>` argument
2. Loads markdown file into `ReviewConfig`
3. Renders overlay panel with:
   - Markdown-formatted document content
   - Text input field for comments
   - "Send & Exit" button (or Ctrl+Enter)
   - ESC to cancel without comment
4. On exit, comment is written to **stdout**
5. App terminates after review submission

**Review output format:** Plain text written to stdout

### Vibe-Kanban Task Lifecycle

**Status transitions:**
```
todo → inprogress → inreview → done/cancelled
```

**Key components:**
- `TaskStatus` enum: `Todo`, `InProgress`, `InReview`, `Done`, `Cancelled`
- MCP tools: `update_task`, `create_task`, `list_tasks`, `start_workspace_session`
- Approval system: Already transitions tasks to `InReview` when awaiting approval

---

## Multi-Command Response Protocol

### Response Format

The reviewer's response supports **multiple commands** separated by newlines. Each command is on its own line.

```
COMMAND1: argument
COMMAND2: argument
COMMAND3
```

### Available Commands

| Command | Arguments | Description | Required MCP Action |
|---------|-----------|-------------|---------------------|
| `APPROVE` | none | Approve changes for merge | `update_task(status: "done")` |
| `CONTINUE` | feedback text | Continue development | `update_task(status: "inprogress")` |
| `SPAWN` | task title | Create new follow-up task | `create_task(title: ...)` |
| `DISCARD` | none | Cancel task and discard changes | `update_task(status: "cancelled")` |
| `REBASE` | none | Rebase onto main branch | git commands |
| `MERGE` | none | Merge to main (after approve) | git commands |
| `COMMENT` | text | Add comment/note (no status change) | Log/record comment |

### Command Combinations

Common multi-command patterns:

```
# Approve and merge immediately
APPROVE
MERGE

# Approve but spawn follow-up tasks first
APPROVE
SPAWN: Add unit tests for new feature
SPAWN: Update documentation

# Continue with feedback and spawn related task
CONTINUE: Fix the edge case in physics calculation
SPAWN: Investigate performance regression

# Approve, rebase, then merge
APPROVE
REBASE
MERGE

# Discard current approach but spawn new task with different approach
DISCARD
SPAWN: Implement feature using alternative approach
```

### Validation Rules

1. **At least one command required** - Empty response is invalid
2. **Mutually exclusive statuses** - Cannot combine `APPROVE` + `DISCARD` or `APPROVE` + `CONTINUE`
3. **Order matters** - Commands executed in order (e.g., `REBASE` before `MERGE`)
4. **Multiple `SPAWN` allowed** - Can create multiple follow-up tasks
5. **`COMMENT` is additive** - Can combine with any other command

### Agent Execution Requirements

The agent **MUST** execute all commands in the response. For each command:

| Command | Required Agent Actions |
|---------|------------------------|
| `APPROVE` | `mcp__vibe_kanban__update_task(task_id, status: "done")` |
| `CONTINUE` | `mcp__vibe_kanban__update_task(task_id, status: "inprogress")`, incorporate feedback |
| `SPAWN` | `mcp__vibe_kanban__create_task(project_id, title: <arg>)` for each SPAWN |
| `DISCARD` | `mcp__vibe_kanban__update_task(task_id, status: "cancelled")`, delete worktree (see Worktree Operations) |
| `REBASE` | Rebase onto main (see Worktree Operations below) |
| `MERGE` | Merge to main from main worktree (see Worktree Operations below) |
| `COMMENT` | Log comment, optionally add to task description |

### Worktree Git Operations

**IMPORTANT**: This project uses git worktrees. Each task branch lives in a separate worktree directory. The `main` branch is checked out in the primary worktree. You CANNOT run `git checkout main` in a feature worktree because git prevents a branch from being checked out in multiple worktrees simultaneously.

**REBASE:**
```bash
# From the feature worktree
git fetch origin main
git rebase origin/main
```

**MERGE:**
```bash
# Must be done from the main worktree, not the feature worktree
MAIN_WORKTREE=$(git worktree list | grep '\[main\]' | awk '{print $1}')
CURRENT_BRANCH=$(git branch --show-current)
cd "$MAIN_WORKTREE"
git fetch origin main && git merge origin/main --ff-only
git merge "$CURRENT_BRANCH" --no-ff -m "Merge branch '$CURRENT_BRANCH'"
git push origin main
```

**DISCARD:**
```bash
WORKTREE_PATH=$(pwd)
BRANCH=$(git branch --show-current)
MAIN_WORKTREE=$(git worktree list | grep '\[main\]' | awk '{print $1}')
cd "$MAIN_WORKTREE"
git worktree remove "$WORKTREE_PATH" --force
git branch -D "$BRANCH"
```

---

## Recommended Implementation (Hybrid Approach)

### Components

1. **CLAUDE.md Instructions** - Define review workflow and command protocol
2. **Review Document Template** - Standardized format with command reference
3. **Response Parser** - Parse multi-command responses
4. **MCP Integration** - Execute commands via vibe-kanban tools

### 1. CLAUDE.md Instructions

Add to `crossworld/CLAUDE.md`:

```markdown
## Vibe-Kanban Review Workflow

When completing a task, use the review workflow to get user approval.

### Review Process

1. **Prepare for Review**
   - Ensure all changes are committed
   - Run tests: `just check`
   - Update task status: `mcp__vibe_kanban__update_task(task_id, status: "inreview")`

2. **Generate Review Document**
   Create `doc/review/current.md` using the template (see below)

3. **Launch Review**
   ```bash
   cargo run --bin testbed -- --review doc/review/current.md
   ```

4. **Parse and Execute Response**
   The response contains one or more commands (one per line). You MUST execute ALL commands.

### Response Commands

| Command | Example | Action |
|---------|---------|--------|
| `APPROVE` | `APPROVE` | Mark task done |
| `CONTINUE` | `CONTINUE: add error handling` | Update status to inprogress, implement feedback |
| `SPAWN` | `SPAWN: Fix related bug in module X` | Create new task |
| `DISCARD` | `DISCARD` | Cancel task, discard changes |
| `REBASE` | `REBASE` | Rebase onto main |
| `MERGE` | `MERGE` | Merge branch to main |
| `COMMENT` | `COMMENT: Good progress so far` | Record comment |

### Executing Commands

For each command in the response, execute the corresponding action:

```
APPROVE → mcp__vibe_kanban__update_task(task_id, status: "done")
CONTINUE → mcp__vibe_kanban__update_task(task_id, status: "inprogress")
SPAWN → mcp__vibe_kanban__create_task(project_id, title: "<spawn argument>")
DISCARD → mcp__vibe_kanban__update_task(task_id, status: "cancelled"), delete worktree
REBASE → git fetch origin main && git rebase origin/main (from feature worktree)
MERGE → cd <main_worktree> && git merge <branch> --no-ff (must run from main worktree)
```

**Note**: See "Worktree Git Operations" section above for detailed worktree-aware commands.

### Example Response Handling

Response from reviewer:
```
APPROVE
SPAWN: Add integration tests
SPAWN: Update README with new feature
MERGE
```

Agent must execute:
1. `mcp__vibe_kanban__update_task(task_id, status: "done")`
2. `mcp__vibe_kanban__create_task(project_id, title: "Add integration tests")`
3. `mcp__vibe_kanban__create_task(project_id, title: "Update README with new feature")`
4. `git checkout main && git merge <branch> --no-ff`
```

### 2. Review Document Template

Create `doc/templates/review.md`:

```markdown
# Task Review: {{TASK_TITLE}}

**Task ID:** {{TASK_ID}}
**Branch:** {{BRANCH_NAME}}
**Date:** {{DATE}}

## Summary
{{SUMMARY}}

## Changes Made

### Files Modified
{{FILE_LIST}}

### Key Changes
{{CHANGE_DESCRIPTION}}

## Testing

### Tests Run
{{TEST_RESULTS}}

### Manual Testing
{{MANUAL_TESTING}}

## Screenshots/Demos
{{SCREENSHOTS}}

## Open Questions
{{QUESTIONS}}

---

## Reviewer Response

Enter one or more commands (one per line). **At least one command is required.**

### Available Commands

| Command | Usage | Description |
|---------|-------|-------------|
| `APPROVE` | `APPROVE` | Approve changes, mark task as done |
| `CONTINUE` | `CONTINUE: <feedback>` | Request changes, provide feedback |
| `SPAWN` | `SPAWN: <task title>` | Create follow-up task (can use multiple times) |
| `DISCARD` | `DISCARD` | Cancel task and discard all changes |
| `REBASE` | `REBASE` | Rebase branch onto main before merge |
| `MERGE` | `MERGE` | Merge branch to main |
| `COMMENT` | `COMMENT: <note>` | Add a comment without changing status |

### Example Responses

**Simple approval:**
```
APPROVE
```

**Approve with follow-up tasks:**
```
APPROVE
SPAWN: Add unit tests for edge cases
SPAWN: Update API documentation
MERGE
```

**Request changes:**
```
CONTINUE: Please handle the null case in line 42
```

**Approve after rebase:**
```
APPROVE
REBASE
MERGE
```
```

### 3. Response Parser Logic

The agent should parse the response as follows:

```python
def parse_review_response(response: str) -> list[dict]:
    """Parse multi-command review response."""
    commands = []

    for line in response.strip().split('\n'):
        line = line.strip()
        if not line:
            continue

        # Parse command and argument
        if ':' in line:
            cmd, arg = line.split(':', 1)
            cmd = cmd.strip().upper()
            arg = arg.strip()
        else:
            cmd = line.strip().upper()
            arg = None

        # Validate command
        valid_commands = ['APPROVE', 'CONTINUE', 'SPAWN', 'DISCARD', 'REBASE', 'MERGE', 'COMMENT']
        if cmd not in valid_commands:
            raise ValueError(f"Unknown command: {cmd}")

        commands.append({'command': cmd, 'argument': arg})

    if not commands:
        raise ValueError("At least one command is required")

    # Validate mutual exclusivity
    statuses = [c['command'] for c in commands if c['command'] in ['APPROVE', 'CONTINUE', 'DISCARD']]
    if len(statuses) > 1:
        raise ValueError(f"Mutually exclusive commands: {statuses}")

    return commands
```

### 4. Command Execution Logic

```python
async def execute_review_commands(commands: list[dict], task_id: str, project_id: str, branch: str, worktree_path: str):
    """Execute all review commands in order.

    IMPORTANT: This project uses git worktrees. The main branch is checked out
    in a separate worktree. You cannot `git checkout main` from a feature worktree.
    """

    # Find the main worktree path
    result = await run_bash('git worktree list')
    main_worktree = None
    for line in result.split('\n'):
        if '[main]' in line:
            main_worktree = line.split()[0]
            break

    for cmd in commands:
        match cmd['command']:
            case 'APPROVE':
                await mcp_vibe_kanban_update_task(task_id, status='done')

            case 'CONTINUE':
                await mcp_vibe_kanban_update_task(task_id, status='inprogress')
                # Store feedback for agent to act on
                feedback = cmd['argument']

            case 'SPAWN':
                await mcp_vibe_kanban_create_task(
                    project_id=project_id,
                    title=cmd['argument']
                )

            case 'DISCARD':
                await mcp_vibe_kanban_update_task(task_id, status='cancelled')
                # Must delete worktree from main worktree, not from within it
                await run_bash(f'cd {main_worktree} && git worktree remove {worktree_path} --force')
                await run_bash(f'cd {main_worktree} && git branch -D {branch}')

            case 'REBASE':
                # Rebase can be done from within the feature worktree
                await run_bash('git fetch origin main')
                await run_bash('git rebase origin/main')

            case 'MERGE':
                # MUST be done from main worktree - cannot checkout main in feature worktree
                await run_bash(f'cd {main_worktree} && git fetch origin main')
                await run_bash(f'cd {main_worktree} && git merge origin/main --ff-only')
                await run_bash(f'cd {main_worktree} && git merge {branch} --no-ff -m "Merge branch \'{branch}\'"')
                await run_bash(f'cd {main_worktree} && git push origin main')

            case 'COMMENT':
                # Log or store comment
                log_comment(cmd['argument'])
```

---

## Implementation Phases

### Phase 1: Configuration-Only (Immediate)

1. Add review workflow instructions to `CLAUDE.md`
2. Create review document template at `doc/templates/review.md`
3. Document multi-command response protocol

**Deliverables:**
- Updated `CLAUDE.md` with review workflow section
- Review template file
- This specification document

### Phase 2: Validation & Tooling (Short-term)

1. Add response validation in agent instructions
2. Create example review documents
3. Test workflow end-to-end

**Deliverables:**
- Example review documents
- Validation rules documentation
- Test scenarios

### Phase 3: Full Integration (Long-term)

1. Add `request_review` MCP tool to vibe-kanban
2. Server-side response parsing and validation
3. Auto-execute commands server-side
4. Review history tracking

**Deliverables:**
- New MCP tool implementation
- Server-side command execution
- Review audit trail

---

## Example Complete Workflow

### 1. Agent Completes Work

```
Agent: I've finished implementing the physics component.
       Let me prepare for review.

# Run checks
$ just check
✓ All checks passed

# Update task status
mcp__vibe_kanban__update_task(task_id="abc123", status="inreview")
```

### 2. Agent Creates Review Document

```markdown
# Task Review: Implement Physics Component

**Task ID:** abc123
**Branch:** vk/abc123-physics-component
**Date:** 2024-01-15

## Summary
Implemented Rapier3D physics integration with character controllers.

## Changes Made
- crates/physics/src/lib.rs - New physics world wrapper
- crates/physics/src/character.rs - Character controller
- packages/app/src/physics/ - TypeScript bindings

## Testing
- Unit tests: ✓ All passing
- Manual testing: Character movement works in testbed

## Open Questions
- Should we add collision callbacks for game events?
```

### 3. Agent Launches Review

```bash
$ cargo run --bin testbed -- --review doc/review/current.md
```

### 4. User Reviews and Responds

User sees the review overlay, reads the summary, and types:

```
APPROVE
SPAWN: Add collision callback support
SPAWN: Write physics documentation
COMMENT: Great implementation! Clean code.
MERGE
```

### 5. Agent Parses Response

```
Commands parsed:
1. APPROVE
2. SPAWN: Add collision callback support
3. SPAWN: Write physics documentation
4. COMMENT: Great implementation! Clean code.
5. MERGE
```

### 6. Agent Executes All Commands

```python
# 1. APPROVE - Mark task done
mcp__vibe_kanban__update_task(task_id="abc123", status="done")

# 2. SPAWN - Create first follow-up task
mcp__vibe_kanban__create_task(
    project_id="proj456",
    title="Add collision callback support"
)

# 3. SPAWN - Create second follow-up task
mcp__vibe_kanban__create_task(
    project_id="proj456",
    title="Write physics documentation"
)

# 4. COMMENT - Log the comment
print("Reviewer comment: Great implementation! Clean code.")

# 5. MERGE - Merge to main (from main worktree, not feature worktree!)
$ MAIN_WORKTREE=$(git worktree list | grep '\[main\]' | awk '{print $1}')
$ cd "$MAIN_WORKTREE"
$ git fetch origin main && git merge origin/main --ff-only
$ git merge vk/abc123-physics-component --no-ff -m "Merge branch 'vk/abc123-physics-component'"
$ git push origin main
```

### 7. Workflow Complete

- Original task marked as `done`
- Two new tasks created in backlog
- Code merged to main
- Agent ready for next task

---

## Validation Checklist

When implementing review workflow, verify:

- [ ] Review document contains all required sections
- [ ] Response contains at least one command
- [ ] No conflicting status commands (APPROVE + DISCARD)
- [ ] All SPAWN commands have task titles
- [ ] CONTINUE commands include feedback text
- [ ] All commands executed via appropriate MCP tools
- [ ] Git operations complete successfully
- [ ] Task status updated correctly in vibe-kanban

---

## Future Enhancements

1. **Rich Review UI**: Enhanced testbed overlay with diff viewer
2. **Command Autocomplete**: Suggest commands in review UI
3. **Review Templates**: Pre-defined templates for different task types
4. **Review History**: Track all review cycles in vibe-kanban
5. **Automated Checks**: Include test results, lint status in review doc
6. **PR Integration**: Auto-create GitHub PR from MERGE command
7. **Batch Review**: Review multiple related tasks together
8. **Review Metrics**: Track approval rates, iteration counts
