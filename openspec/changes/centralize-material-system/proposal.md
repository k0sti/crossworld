# Change: Centralize Material System

## Why
Material definitions are currently scattered across `assets/materials.json`, `renderer`, and `world` crates. This leads to duplication and potential inconsistencies. The `cube` crate is the common dependency and should hold the single source of truth for materials.

## What Changes
- **Centralize in `cube`**: Create `crates/cube/src/material.rs` to hold the material definitions (0-127) and R2G3B2 logic (128-255).
- **Update Renderer**: Refactor `cpu_tracer`, `gl_tracer`, and shaders to use the new centralized system.
- **Cleanup**: Remove `crates/renderer/src/materials.rs` and other temporary definitions.

## Impact
- **Affected Specs**: `materials` (NEW)
- **Affected Code**: `crates/cube`, `crates/renderer`, `crates/world`
