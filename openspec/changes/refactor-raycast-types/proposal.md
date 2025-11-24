# Change: Refactor Raycast Types

## Why
To improve type safety and error handling in the raycast system.
- **Error Handling**: Currently `raycast` returns `None` for out-of-bounds, which swallows potential errors. Explicit error types allow better debugging (e.g. rendering errors as purple).
- **Type Safety**: Normals are currently `Vec3`, but they are always axis-aligned in the voxel world. An `Axis` enum will enforce this constraint and improve code clarity.

## What Changes
- **Cube Crate**:
    - Create `enum Axis { PosX, NegX, PosY, NegY, PosZ, NegZ }`.
    - Update `RaycastHit.normal` to use `Axis`.
    - Update `raycast` signature to return `Result<Option<RaycastHit>, RaycastError>`.
- **Renderer Crate**:
    - Update `cpu_tracer` to handle `RaycastError` and render debug colors (purple).
    - Update `gl_tracer` if applicable (though shaders use vec3).

## Impact
- **Affected Specs**: `raycast`
- **Affected Code**: `crates/cube`, `crates/renderer`
