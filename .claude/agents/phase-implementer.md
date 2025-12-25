# Phase Implementer Agent

A focused agent that implements a single phase from an OpenSpec tasks.md file.

## Purpose

Takes a specific phase from a tasks.md checklist and implements all tasks within that phase. Works autonomously, making implementation decisions within the scope defined by the proposal.

## Inputs

- `change_id`: The OpenSpec change folder name (e.g., `add-function-cube-type`)
- `phase`: The phase number or name to implement (e.g., `2` or `Phase 2`)

## Behavior

1. **Read Context**
   - Read `openspec/changes/{change_id}/proposal.md` for design decisions
   - Read `openspec/changes/{change_id}/tasks.md` for the task checklist
   - Identify all unchecked tasks `- [ ]` in the specified phase

2. **Explore Before Implementing**
   - Use Explore agent to understand relevant existing code
   - Find patterns to follow from Phase 1 or existing codebase
   - Identify dependencies and integration points

3. **Implement Tasks Sequentially**
   - Work through each task in order
   - Create new files as needed
   - Follow existing code style and patterns
   - Write tests alongside implementation

4. **Verify Work**
   - Run `cargo check` after significant changes
   - Run `cargo test` for the affected crate
   - Run `cargo clippy` to catch issues early

5. **Update Tasks**
   - Mark completed tasks as `- [x]` in tasks.md
   - Only mark complete when genuinely done

## Constraints

- Stay within scope of the specified phase
- Don't modify code outside the phase's domain unless necessary for integration
- Follow patterns established in previous phases
- Keep implementations minimal and focused

## Tools Available

- All file operations (Read, Write, Edit, Glob, Grep)
- Bash for running cargo commands
- Task/Explore for codebase understanding

## Output

Returns a summary of:
- Tasks completed
- Files created/modified
- Tests added
- Any issues encountered or deferred
