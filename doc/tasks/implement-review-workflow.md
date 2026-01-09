# Vibe-Kanban Task: Implement Review Workflow

**Title:** Implement vibe-kanban review workflow with multi-command support

**Status:** todo

## Description

Implement the review workflow for vibe-kanban as specified in `doc/vibe-kanban-review-workflow.md`.

### Requirements

1. **Add Review Workflow Instructions to CLAUDE.md**
   - Add the review process section to `crossworld/CLAUDE.md`
   - Document the multi-command response protocol
   - Include command execution requirements

2. **Create Review Document Template**
   - Create `doc/templates/review.md` with placeholder variables
   - Include command reference in template
   - Add example responses

3. **Test Workflow End-to-End**
   - Create a sample review document
   - Test with `cargo run --bin testbed -- --review <doc>`
   - Verify stdout output parsing
   - Test multi-command responses

### Acceptance Criteria

- [ ] `CLAUDE.md` contains review workflow section with:
  - Review process steps (prepare, generate doc, launch, parse response)
  - Command reference table (APPROVE, CONTINUE, SPAWN, DISCARD, REBASE, MERGE, COMMENT)
  - Execution requirements for each command
  - Example multi-command response handling

- [ ] `doc/templates/review.md` exists with:
  - Task metadata placeholders ({{TASK_TITLE}}, {{TASK_ID}}, etc.)
  - Standard sections (Summary, Changes, Testing, Questions)
  - Command reference for reviewers
  - Example responses

- [ ] Workflow tested successfully with:
  - Single command responses (APPROVE, CONTINUE)
  - Multi-command responses (APPROVE + SPAWN + MERGE)
  - Command validation (mutual exclusivity)

### Implementation Notes

The review workflow uses the existing `--review` argument in `crates/testbed`. No code changes are required - this is a configuration-only implementation (Phase 1).

Key files to modify:
- `crossworld/CLAUDE.md` - Add review workflow section
- `doc/templates/review.md` - Create new file

Reference:
- `doc/vibe-kanban-review-workflow.md` - Full specification

### Related

- Specification: `doc/vibe-kanban-review-workflow.md`
- Review overlay: `crates/app/src/review_overlay.rs`
- Testbed CLI: `crates/testbed/src/main.rs`

---

## To Create This Task

When vibe-kanban server is running:

```
mcp__vibe_kanban__create_task(
    project_id: "<crossworld-project-id>",
    title: "Implement vibe-kanban review workflow with multi-command support",
    description: "<contents of this file>"
)
```
