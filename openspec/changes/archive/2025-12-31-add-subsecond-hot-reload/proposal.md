# Change: Add Subsecond Hot-Reload Support

## Why
Enable rapid iteration during game development by implementing subsecond hot-reload capabilities inspired by Dioxus's subsecond package. Currently, any code change requires a full rebuild and restart of the application, which significantly slows down the development workflow. Hot-reload will allow developers to see code changes reflected in the running application within milliseconds without losing application state.

## What Changes
- Add new `crates/app` - Base application runtime that provides windowing, OpenGL context, and application lifecycle management
- Add new `crates/game` - Hot-reloadable game implementation that contains the actual game logic and rendering
- Implement `App` trait with lifecycle hooks (`init`, `uninit`, `event`, `update`, `render`)
- Integrate subsecond-inspired dynamic library reloading mechanism
- Migrate core application setup from `crates/renderer` to `crates/app`
- Implement initial game demo (rotating cube) in `crates/game`

## Impact
- Affected specs: New capabilities `hot-reload-runtime` and `game-application`
- Affected code:
  - New crate: `crates/app/` (main application runtime)
  - New crate: `crates/game/` (hot-reloadable game code)
  - Workspace: `Cargo.toml` (add new workspace members)
  - Existing: `crates/renderer/` (reference implementation to copy from)
- Development workflow: Developers will be able to modify game logic and see changes within subseconds
- Build system: New build targets for dynamic library compilation
