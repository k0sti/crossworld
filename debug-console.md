# Debug Console Logging

## Manual Method (Easiest)

1. Open the app in browser
2. Open DevTools (F12)
3. Go to Console tab
4. Click the chat icon to open/close
5. Watch for log messages with prefixes:
   - `[App]` - App.tsx state changes
   - `[ChatPanel]` - Chat panel renders
   - `[WorldCanvas]` - Canvas component renders
   - `[SceneManager]` - Three.js rendering

## Browser Console Recording

### Using Chromium Remote Debugging

```bash
# Start dev server
just dev

# In another terminal, start Chromium with remote debugging
chromium --remote-debugging-port=9222 http://localhost:5173

# In another terminal, use Chrome DevTools Protocol to capture logs
# (requires a CDP client tool)
```

### Using Playwright (Rust)

If you want to automate console capture, you can use the `playwright` Rust crate:

```toml
[dev-dependencies]
playwright = "0.0.20"
```

Example code:
```rust
use playwright::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::initialize().await?;
    let chromium = playwright.chromium();
    let browser = chromium.launcher().headless(false).launch().await?;
    let page = browser.context_builder().build().await?.new_page().await?;

    // Listen to console messages
    page.on_console_message(|msg| {
        println!("{}: {}", msg.type_(), msg.text());
    });

    page.goto_builder("http://localhost:5173").goto().await?;

    // Wait and interact
    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    Ok(())
}
```

## What to Look For

When clicking the chat icon, check if you see:

1. **State changes**: `[App] onToggleChat - current: false -> new: true`
2. **Component re-renders**:
   - `[ChatPanel] Render - isOpen: true`
   - `[WorldCanvas] Render - isChatOpen: true`
3. **Rendering errors**: `[SceneManager] render() called but missing: ...`
4. **Multiple re-renders**: If WorldCanvas renders more than once, that could cause the blink

## Expected Behavior

- Opening chat should trigger 1-2 renders of WorldCanvas (React strict mode may cause 2)
- SceneManager should NOT report missing renderer/scene/camera
- No errors in console
