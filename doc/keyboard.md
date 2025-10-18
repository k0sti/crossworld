# Keyboard Shortcuts Design

## Overview
Crossworld should be fully usable without a mouse. This document defines the keyboard shortcuts and input mode system.

---

## Walk Mode (Default)

Primary mode for navigation and interaction with the 3D world.

### Movement
- **W**: Move forward
- **S**: Move backward
- **A**: Move left / Strafe left
- **D**: Move right / Strafe right
- **SPACE**: Jump (if implemented)
- **SHIFT+W/A/S/D**: Run (2x speed)

### Camera Control
- **F**: Toggle first-person / third-person view
- **SCROLL (mouse wheel)**: Zoom in/out, on third-person view
- NOTE: add first/third person camera icon to left bar

### Quick Actions
- **ESC**: Close any open panel

### Panel Navigation
- **ALT+N**: Open Network config panel
- **ALT+P**: Open Profile panel
- **ALT+A**: Open Avatar config panel
- **ALT+C**: Toggle Chat panel open/closed
- **ALT+Q**: Logout
- Add missing left bar shortcuts and add hover info for all

### Panel-Specific (when panel is open)
- **ESC**: Close current panel
- **ENTER**: Confirm/Apply changes (if applicable)

## Chat Panel

Opened/closed by clicking the chat icon or pressing **ALT+C**.

### Chat Input
- **ENTER**: Send message
- **SHIFT+ENTER**: New line in message
- **ESC**: Close chat panel

---

## Global Shortcuts

These work in any mode and don't require mode switching:

### Application
- **F1**: Toggle help overlay
- **F5**: Refresh page (native browser)
- **F11**: Toggle fullscreen (native browser)
- **ALT+F4**: Close window (native OS)
- **CTRL+R**: Reload page (native browser)

### Accessibility
- **ALT+H**: Show keyboard shortcuts help
- **CTRL+PLUS**: Increase UI scale
- **CTRL+MINUS**: Decrease UI scale
- **CTRL+0**: Reset UI scale

---

## Implementation Notes

### Input Capture Priority
1. **Browser native shortcuts** (highest priority - cannot override)
2. **Modal dialogs** (e.g., login modal, confirmation dialogs)
3. **Chat input** (when chat panel is open and focused)
4. **Global shortcuts** (lowest priority)

### Event Handling Architecture
- **KeyboardManager** class handles all keyboard input
- Registers global event listeners on mount
- Dispatches to appropriate handler based on current mode
- Prevents default browser behavior when needed
- Maintains pressed keys state for WASD movement

### Walk Mode Movement
- Diagonal movement (e.g., W+D) is normalized to prevent faster diagonal speed
- SHIFT modifier increases speed by 2x

### Accessibility Considerations
- All functions accessible via keyboard
- Visual focus indicators for all interactive elements
