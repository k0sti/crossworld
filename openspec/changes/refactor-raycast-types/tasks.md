# Implementation Tasks - Refactor Raycast Types

## 1. Define Axis Type
- [ ] 1.1 Create `crates/cube/src/axis.rs` with `enum Axis`
- [ ] 1.2 Implement conversion to/from `Vec3` and `i32`
- [ ] 1.3 Add unit tests for Axis

## 2. Improve Error Handling
- [ ] 2.1 Define `RaycastError` enum in `crates/cube/src/raycast/error.rs`
- [ ] 2.2 Update `raycast` to return `Result<Option<RaycastHit>, RaycastError>`
- [ ] 2.3 Return specific error when ray start position is out of bounds

## 3. Update Raycast Implementation
- [ ] 3.1 Update `RaycastHit` struct to use `Axis` for normal
- [ ] 3.2 Update `raycast_recursive` to propagate errors
- [ ] 3.3 Update `calculate_entry_normal` to return `Axis`

## 4. Update Consumers
- [ ] 4.1 Update `cpu_tracer.rs` to handle `Result`
- [ ] 4.2 Render purple color for `RaycastError` in `cpu_tracer`
- [ ] 4.3 Update tests in `cube` and `renderer`
