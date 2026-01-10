# Task Review: Add action buttons to app review feature

**Task ID:** 085a-add-action-butto
**Branch:** vk/085a-add-action-butto
**Date:** 2026-01-10

## Summary
Added action buttons to the review overlay UI, replacing the single "Send & Exit" button with multiple action buttons that output vibe-kanban review commands. This allows reviewers to approve, continue, spawn tasks, discard, rebase, or merge directly from the UI.

## Changes Made

### Files Modified
- `crates/app/src/review_overlay.rs` - Main UI changes
- `crates/app/src/runner.rs` - Action handling

### Key Changes

**ReviewAction enum expanded:**
- `None` - No action yet (continue reviewing)
- `Approve` - Outputs `APPROVE`
- `ContinueWithFeedback(String)` - Outputs `CONTINUE: <message>`
- `Spawn(String)` - Outputs `SPAWN: <title>`
- `Discard` - Outputs `DISCARD`
- `Rebase` - Outputs `REBASE`
- `Merge` - Outputs `MERGE`
- `Complete` - Outputs `APPROVE`, `REBASE`, `MERGE` (combined workflow)
- `Cancel` - Exit without action

**New action buttons:**
| Button | Color | Shortcut | Output |
|--------|-------|----------|--------|
| Complete | Green | Ctrl+Shift+C | APPROVE + REBASE + MERGE |
| Approve | Blue | Ctrl+A | APPROVE |
| Rebase | Gray | Ctrl+R | REBASE |
| Merge | Gray | Ctrl+M | MERGE |
| Discard | Red | Ctrl+D | DISCARD |
| Continue | Orange | Ctrl+Enter | CONTINUE: <message> |
| Spawn Task | Blue | Ctrl+S | SPAWN: <title> |

**Text input field:**
- Used for Continue feedback or Spawn task title
- Continue and Spawn buttons are disabled until text is entered

## Testing

### Tests Run
- `cargo clippy -p app -p testbed -- -D warnings` - Passed
- `cargo check -p app -p testbed` - Passed

### Manual Testing
- Launch testbed with `--review` flag to verify UI renders correctly
- Test each button click triggers correct action
- Test keyboard shortcuts

## Screenshots/Demos
N/A - Run `cargo run -p testbed -- --review doc/review/current.md` to see the UI

## Open Questions
None

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
