# Crossworld Project Audit Report

**Generated:** 2026-02-06
**Scope:** Comprehensive investigation from multiple viewpoints
**Codebase:** ~70,000+ lines of Rust, TypeScript/React frontend

---

## Executive Summary

### Overall Health Score: **B+ (Good with Notable Issues)**

| Area | Score | Status |
|------|-------|--------|
| Code Quality | A- | Compiles clean, good patterns |
| Documentation | B+ | Comprehensive but some drift |
| Organization | B | Well-structured with some redundancy |
| Consistency | B- | Version/naming inconsistencies |
| Maintainability | B | TODOs and dead code present |

**Key Strengths:**
- Clean Rust compilation (no errors, minimal warnings)
- Well-organized workspace with clear crate purposes
- Comprehensive documentation covering architecture, features, and workflows
- Good separation between WASM core and native applications

**Primary Concerns:**
- 4 experimental crates of questionable value (Bevy-based, proto)
- 26+ TODO/FIXME comments indicating incomplete work
- 652 unwrap/expect calls (many in non-test code)
- Documentation drift from actual codebase structure
- Version inconsistencies between crates

---

## 1. Consistency Check

### Cargo.toml Dependencies vs Actual Usage

**Version Inconsistencies:**
| Crate | Version | Note |
|-------|---------|------|
| cube | 0.0.1 | Foundation crate at pre-release |
| world | 0.0.1 | WASM module at pre-release |
| physics | 0.0.1 | WASM module at pre-release |
| app, editor, game, etc. | 0.1.0 | Applications at minor version |
| rapier3d | git | Using git source, not crates.io |

**Recommendation:** Align all crates to consistent versioning scheme (e.g., 0.1.0 for all).

### Crate Names Consistency

**Inconsistencies Found:**

| Directory | Package Name | Binary Name |
|-----------|--------------|-------------|
| `crates/nostr/` | `crossworld-nostr` | `nostr-login` |
| `crates/server/` | `crossworld-server` | `server` |
| `crates/physics/` | `crossworld-physics` | (library) |
| `crates/world/` | `crossworld-world` | (library) |
| `crates/test-client/` | `crossworld-test-client` | `test-client` |

**Issue:** Mix of `crossworld-*` prefixed and unprefixed names. The documentation refers to crates by directory name (e.g., `nostr`), but package names differ.

**Recommendation:** Standardize naming - either all use `crossworld-*` prefix or none.

### Module Exports vs Documentation

**doc/reference/project-structure.md Issues:**

1. **Missing Crates:** Documentation lists 8 crates, actual workspace has 22:
   - Not mentioned: `core`, `app`, `game`, `editor`, `testbed`, `proto-gl`, `app-bevy`, `editor-bevy`, `proto-bevy`, `nostr`, `xcube`, `trellis`, `robocube`, `scripting`

2. **Outdated Cargo.toml Example:**
   ```toml
   # Doc shows:
   members = ["crates/world", "crates/cube", "crates/physics", ...]  # 6 members
   # Actual:
   members = [...] # 22 members
   ```

3. **Missing WASM Package:** `wasm-core` is generated but not documented

4. **Incorrect Path Descriptions:**
   - Doc: `packages/editor/` described as "future"
   - Reality: `packages/editor/` exists with active code

---

## 2. Redundancy & Dead Code

### Unused/Experimental Crates

| Crate | Lines | Status | Recommendation |
|-------|-------|--------|----------------|
| `app-bevy` | ~100 | Minimal, experimental | **Remove** |
| `editor-bevy` | ~500 | Incomplete stubs | **Remove** |
| `proto-bevy` | ~50 | Prototype | **Remove** |
| `proto-gl` | ~800 | Superseded | **Remove** |

**Evidence:**
- `editor-bevy/src/file_ops.rs:27` contains `// TODO: Implement actual save`
- `editor-bevy/src/editing.rs:50` contains `// Mark for mesh update (actual cube update TODO)`
- Multiple `#[allow(dead_code)]` annotations in these crates

### TODO/FIXME Comments (26 Found)

**By Severity:**

| Category | Count | Example |
|----------|-------|---------|
| Missing Implementation | 12 | `TODO: Implement add_cube method for voxel objects` |
| Known Bugs | 1 | `FIXME: BCF parser bug - deserialization produces garbage` |
| Future Features | 8 | `TODO: Implement rendering for Quad and Layers` |
| Unclear/Old | 5 | Various `TODO:` without clear action |

**Files with Most TODOs:**
- `crates/editor-bevy/` - 8 TODOs (entire crate is stub implementation)
- `crates/physics/src/wasm.rs` - 2 TODOs
- `crates/world/src/lib.rs` - 2 TODOs

### Duplicate Functionality

**Identified in Previous Review (doc/review/code-cleanup.md):**
1. **Lua Configuration:** Two `lua_config.rs` files (app + editor)
2. **Bevy vs Native:** Parallel implementations of same features
3. **Camera Controls:** Similar patterns in `game` and `editor`

### Dead Code Annotations

22 instances of `#[allow(dead_code)]` found:
- `crates/editor-bevy/` - 4 instances
- `crates/world/` - 3 instances
- `crates/server/` - 4 instances
- `crates/physics/` - 2 instances
- `crates/renderer/` - 2 instances
- Other crates - 7 instances

### Ignored Tests

| File | Reason |
|------|--------|
| `bcf_roundtrip_tests.rs:368` | `FIXME: BCF parser bug` |
| `vox/loader.rs:325` | Requires asset files |
| `bcf_cpu_tracer_tests.rs:282,434` | No reason given |

---

## 3. File Organization

### Crate Categorization Analysis

**Foundation Layer:**
- `core` - Graphics primitives, input abstractions
- `cube` - Voxel octree engine (43 source files)
- `physics` - Rapier3D integration (18 source files)

**Platform Layer:**
- `app` - Application framework (10 source files)
- `renderer` - OpenGL rendering (27 source files)

**Application Layer:**
- `editor` - Voxel editor (9 source files)
- `game` - Hot-reload game (3 source files)
- `testbed` - Testing application (4 source files)

**Services Layer:**
- `server` - WebTransport multiplayer (8 source files)
- `nostr` - Decentralized identity (6 source files)
- `world` - Terrain generation (8 source files)

**Tools Layer:**
- `worldtool` - CLI for Nostr events
- `assets` - Asset management
- `xcube`, `trellis`, `robocube` - Conversion tools

**Experimental (Should Remove):**
- `app-bevy`, `editor-bevy`, `proto-bevy`, `proto-gl`

### Directory Structure Assessment

**Well-Organized:**
- `crates/` - Clear separation of Rust modules
- `packages/` - TypeScript packages properly isolated
- `doc/` - Organized into architecture/, features/, reference/, tasks/
- `assets/` - Clear manifest-based organization

**Issues:**
- `openspec/` exists but not mentioned in main documentation
- `specs/` directory referenced in docs but contains only symlinks/migrations
- No clear distinction between "active" and "experimental" crates

### Test File Organization

**Tests properly placed:**
- `crates/cube/tests/` - 11 test files
- `crates/renderer/tests/` - 13 test files
- `crates/physics/benches/` - Benchmarks

**Issue:** Some tests use relative paths to assets that may not exist in all environments.

---

## 4. Documentation Quality

### README Accuracy

**CLAUDE.md Assessment:**
- Comprehensive (500+ lines)
- Documents all crates with purposes
- Build commands accurate (`just dev`, `just build`, etc.)
- Server section well-detailed

**Inaccuracies:**
- Lists fewer crates than actually exist
- Path mappings in tsconfig don't include `crossworld-physics`

### doc/reference/project-structure.md Assessment

**Accuracy Issues:**
- Lists 8 crates, workspace has 22
- Cargo.toml example outdated
- Missing TypeScript package `@crossworld/core` (wasm-core)

**Recommendation:** Regenerate from actual `Cargo.toml` and `package.json` files.

### Source Code Comments

**Quality Assessment:**

| Metric | Count | Quality |
|--------|-------|---------|
| Doc comments (`///`) | Good coverage in cube crate | A |
| Inline comments | Sparse in most crates | C |
| TODO comments | 26 (should be tracked as issues) | C |

**Positive Examples:**
- `crates/cube/src/core/cube.rs` - Well-documented public API
- `crates/physics/src/terrain/` - Clear module organization

**Needs Improvement:**
- `crates/server/` - Minimal comments despite complex async logic
- `crates/renderer/` - Shader code lacks explanatory comments

### OpenSpec Documentation

**Location:** `openspec/specs/` (18 spec documents)

**Assessment:**
- Comprehensive technical specifications
- Covers: BCF format, materials, raycasting, collision, rendering
- Not referenced from main documentation
- Some specs may be outdated

**Recommendation:** Link OpenSpec from main docs or consolidate.

---

## 5. Code Quality Signals

### Compiler Warnings

**`cargo clippy --workspace` Results:** Clean (0 warnings with `-D warnings`)

**Rust 2024 Edition Compatibility Warnings (from diagnostics):**
- `simple_egui_gl.rs` - 10 warnings about unsafe blocks in unsafe functions
- `collision.rs` - 5 warnings about deprecated `criterion::black_box`
- `tracer_raycast_debug_test.rs` - 1 dead code warning

### Panics in Non-Test Code

**53 `panic!()` calls found across codebase:**

| Location | Count | Severity |
|----------|-------|----------|
| Test files | 35 | Expected |
| Non-test src/ | 18 | **Review needed** |

**Concerning Panics:**
```rust
// crates/renderer/src/renderers/gl_tracer.rs:679
panic!("GlTracer requires GL context. Use render_to_framebuffer() instead.");

// crates/cube/src/core/cube.rs:339
panic!("Cannot get default ID for non-Solid Cube without i32 type");
```

**Recommendation:** Replace panics with `Result` types where possible.

### Unwrap/Expect Usage

**652 occurrences across 85 files:**

| Category | Approximate Count |
|----------|-------------------|
| Test files | ~200 |
| Examples | ~50 |
| Production code | ~400 |

**High-Risk Files:**
- `crates/scripting/src/lua_engine.rs` - 18 occurrences
- `crates/cube/src/function/parser.rs` - 25 occurrences
- `crates/robocube/src/convert.rs` - 19 occurrences

**Recommendation:** Audit production unwraps; convert critical paths to proper error handling.

### Formatting

**`cargo fmt --check` Results:** Clean (all files properly formatted)

---

## 6. Actionable Cleanup Tasks

### High Priority (Clear Wins)

1. **Remove Experimental Crates**
   - Delete `crates/app-bevy/`
   - Delete `crates/editor-bevy/`
   - Delete `crates/proto-bevy/`
   - Delete `crates/proto-gl/`
   - Update workspace members in root `Cargo.toml`
   - **Impact:** -2,500+ lines, cleaner build

2. **Fix Documentation Drift**
   - Update `doc/reference/project-structure.md` with actual crate list
   - Add missing crates to CLAUDE.md if not there
   - Document OpenSpec location and purpose
   - **Impact:** Improved developer onboarding

3. **Address FIXME in BCF Parser**
   - `crates/cube/tests/bcf_roundtrip_tests.rs:368`
   - BCF deserialization produces garbage for complex nested pointers
   - **Impact:** Data integrity

### Medium Priority (Technical Debt)

4. **Version Alignment**
   - Bump all crate versions to 0.1.0
   - Or adopt semantic versioning consistently
   - **Impact:** Clearer dependency management

5. **Resolve TODO Comments**
   - Convert to GitHub issues or implement
   - Particularly: `physics/src/wasm.rs` add_cube method
   - **Impact:** Clearer task tracking

6. **Standardize Crate Naming**
   - Choose: `crossworld-*` prefix for all or none
   - Update documentation to match
   - **Impact:** Reduced confusion

7. **Audit `unwrap()` Calls**
   - Review 400+ production unwraps
   - Add proper error handling where appropriate
   - **Impact:** Improved reliability

### Low Priority (Nice to Have)

8. **Consolidate Lua Configuration**
   - Merge `app/lua_config.rs` and `editor/lua_config.rs`
   - **Impact:** Reduced duplication

9. **Remove `#[allow(dead_code)]`**
   - Either use the code or delete it
   - **Impact:** Cleaner codebase

10. **Update Rust 2024 Compatibility**
    - Fix unsafe block warnings in `simple_egui_gl.rs`
    - Update deprecated `criterion::black_box` usage
    - **Impact:** Future-proofing

---

## 7. Recommendations for Project Hygiene

### Immediate Actions

1. **Create `.cargo/config.toml`** (if not exists) with:
   ```toml
   [build]
   rustflags = ["-D", "warnings"]
   ```

2. **Add CI Check** for:
   - `cargo clippy --workspace -- -D warnings`
   - `cargo fmt --check`
   - `cargo test --workspace`

3. **Document Crate Maturity Levels:**
   - Mark experimental crates clearly in README
   - Or remove them entirely

### Ongoing Maintenance

1. **Weekly:** Run `cargo clippy` and address warnings
2. **Monthly:** Audit TODO comments, convert to issues
3. **Quarterly:** Review documentation for drift
4. **Per-Release:** Verify all crate versions aligned

### Architecture Improvements

1. **Consider Feature Flags:**
   - Make `gilrs` (gamepad) non-default in `app`
   - Reduce unnecessary dependency compilation

2. **Error Handling Strategy:**
   - Define project-wide error types
   - Audit and reduce panic paths
   - Use `anyhow` consistently for error context

3. **Test Coverage:**
   - Enable ignored tests or delete them
   - Add integration tests for server
   - Add TypeScript tests for frontend

---

## Appendix A: Crate Summary

| Crate | Version | Purpose | Files | Status |
|-------|---------|---------|-------|--------|
| cube | 0.0.1 | Voxel octree engine | 43 | Active |
| world | 0.0.1 | Terrain generation | 8 | Active |
| physics | 0.0.1 | Rapier3D wrapper | 18 | Active |
| core | 0.1.0 | Graphics foundation | 3 | Active |
| app | 0.1.0 | Application framework | 10 | Active |
| renderer | 0.1.0 | OpenGL rendering | 27 | Active |
| editor | 0.1.0 | Voxel editor | 9 | Active |
| game | 0.1.0 | Hot-reload game | 3 | Review needed |
| testbed | 0.1.0 | Testing app | 4 | Active |
| server | 0.1.0 | Multiplayer server | 8 | Active |
| nostr | 0.1.0 | Nostr integration | 6 | Active |
| worldtool | 0.0.1 | CLI tool | 1 | Active |
| assets | 0.1.0 | Asset management | 1 | Active |
| scripting | 0.1.0 | Lua + KDL config | 6 | Active |
| xcube | 0.0.1 | XCube converter | 5 | Tool |
| trellis | 0.0.1 | Trellis converter | 7 | Tool |
| robocube | 0.1.0 | Roblox converter | 5 | Tool |
| test-client | 0.1.0 | Server test client | 1 | Active |
| app-bevy | 0.1.0 | Bevy framework | 1 | **Remove** |
| editor-bevy | 0.1.0 | Bevy editor | 11 | **Remove** |
| proto-bevy | 0.1.0 | Bevy prototype | 1 | **Remove** |
| proto-gl | 0.1.0 | GL prototype | 10 | **Remove** |

---

## Appendix B: Code Metrics

| Metric | Value |
|--------|-------|
| Total Rust LOC | ~70,842 |
| Total Crates | 22 |
| TypeScript Packages | 4 (app, common, editor, root) |
| WASM Modules | 4 (cube, world, physics, core) |
| TODO Comments | 26 |
| FIXME Comments | 1 |
| Panic Calls (non-test) | 18 |
| Unwrap/Expect Calls | 652 |
| Ignored Tests | 4 |
| #[allow(dead_code)] | 22 |

---

*Report generated by deep project audit. Last updated: 2026-02-06*
