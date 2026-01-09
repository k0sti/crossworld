# Review Mode Fix Test

## Changes Made

Reverted to simple, stable implementation:

1. **Window**: Non-collapsible, resizable, anchored to right
2. **Button**: Simple button with 14px text
3. **Removed**: Complex button styling that caused crashes

## Expected Result

- Window loads without crash
- Button is visible and clickable
- Can drag window by title bar (default egui behavior)
- Can submit with button or Ctrl+Enter

Please test and confirm it works.
