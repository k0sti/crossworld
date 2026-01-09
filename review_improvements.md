# Review Mode - Improvements

## Changes Made

### 1. Window is now draggable ✓
- Removed `.anchor()` which locked the position
- Added `.movable(true)` to enable dragging
- Set `.default_pos()` instead of anchor for initial position
- Window can now be dragged anywhere on screen

### 2. Button is now highly visible ✓
- **Full width button**: Takes up entire panel width
- **Larger size**: 36px height (was 28px)
- **Blue background**: `rgb(60, 120, 180)` for visibility
- **Icon**: 📤 emoji for visual identification
- **Larger text**: 16px size, strong weight
- **Border stroke**: Highlighted with lighter blue
- **Better spacing**: Moved help text below button

## Expected Behavior

You should now see:
1. A **large blue button** that says "📤 Send & Exit"
2. Window that can be **dragged by the title bar**
3. Button spans the **full width** of the panel
4. Help text below: "Ctrl+Enter to send • ESC to cancel"

## Test Instructions

1. Drag the window around by clicking title bar
2. Type a test comment
3. Click the blue "📤 Send & Exit" button
4. Comment should be printed to stdout and app should exit
