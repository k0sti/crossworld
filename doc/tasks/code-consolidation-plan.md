# Code Consolidation Plan

## Completed

- ✅ **Consolidate Lua config code** - Already properly consolidated; `editor/src/lua_config.rs` imports from `app::lua_config`
- ✅ **Add README.md files to all crates** - All crates now have concise, clear documentation
- ✅ **Keep Bevy crates** - Decision made to maintain them as alternative implementations
- ✅ **Create hot reload module in app** - Added `app/src/hot_reload.rs` with `HotReloadConfig` and `HotReloadLibrary` (commit d34699d)

## Remaining Tasks

### 1. Refactor Game to Use Hot Reload Module ✅ PARTIALLY COMPLETE

**Status:** Hot reload module created and available in app crate

**Current State:**
- ✅ `app/src/hot_reload.rs` provides `HotReloadLibrary` for library loading/watching/reloading
- ✅ File watching, library loading, and App lifecycle management extracted
- ⏸️ `game/src/main.rs` still uses custom implementation (~900 lines)

**Remaining Work:**
- Refactor `game/src/main.rs` to use `HotReloadLibrary` from app
- This requires careful refactoring of the event loop and GL context management
- Current implementation works, so this is a **nice-to-have** optimization

**Complexity:** MEDIUM-HIGH (requires careful refactoring without breaking existing functionality)

**Estimated Effort:** 3-4 hours

**Recommendation:** Leave game's custom implementation for now; it's working and well-tested

---

### 2. Add Hot Reload to Editor

**Current State:**
- Editor uses `app::run_app()` for standard event loop
- No hot reload support

**Target State:**
- Editor can use hot reload during development
- Editor maintains current non-hot-reload mode for production

**Implementation Steps:**
1. Add `editor/src/main_hotreload.rs` (or feature-gated code)
2. Use `HotReloadRuntime` from app crate
3. Ensure editor's `App` implementation works with hot reload
4. Test hot reload with editor

**Dependencies:** Requires completing task #1 first

**Complexity:** MEDIUM (depends on task #1 being clean)

**Estimated Effort:** 2-3 hours

---

### 3. Extract Shared Code to App and Core

**Current State:**
- Some duplication between `game`, `editor`, `testbed`
- CLI argument parsing duplicated (game vs editor)
- Common GL setup patterns repeated

**Potential Extractions:**

#### 3.1. Consolidate CLI Argument Parsing
**Issue:** `game/src/main.rs` has custom CLI parsing; `editor/src/main.rs` uses `app::cli::CommonArgs`

**Options:**
- **Option A:** Refactor `game` to use `CommonArgs` (cleaner, but may conflict with hot reload needs)
- **Option B:** Keep custom implementation, document why it's needed
- **Recommendation:** Option B for now, document in README

#### 3.2. Extract Common GL Setup Patterns
**Issue:** Camera setup, renderer initialization patterns similar across apps

**Potential extractions:**
- Camera factory functions in `app::camera`
- Renderer setup helpers in `renderer` crate
- GL initialization boilerplate

**Complexity:** MEDIUM (requires careful analysis to avoid premature abstraction)

**Estimated Effort:** 3-4 hours

#### 3.3. Move WASM-Compatible Types to Core
**Current State:** `core` crate already has input types

**Potential additions:**
- Common math type wrappers (if needed)
- Shared enums/constants
- Cross-platform utilities

**Complexity:** LOW

**Estimated Effort:** 1-2 hours

---

## Recommendations

### Immediate (This Session)
- ✅ README files completed
- Document remaining work (this file)

### Next Session(s)
1. **Task #1: Move Hot Reload to App** (HIGH priority if hot reload is important)
   - Breaking change, affects `game` crate
   - Should be done carefully with testing

2. **Task #3.1: CLI Consolidation Decision** (LOW priority)
   - Document why `game` needs custom parsing
   - OR refactor to use CommonArgs

3. **Task #2: Add Hot Reload to Editor** (MEDIUM priority, after #1)
   - Quality of life improvement for editor development

4. **Task #3.2 & #3.3: Code Extraction** (MEDIUM priority)
   - Incremental improvements
   - Can be done over multiple sessions

### Decision Required
**Do we want hot reload in both game and editor?**
- If YES → Prioritize tasks #1 and #2
- If NO → Skip tasks #1 and #2, focus on #3

### Alternative Approach (Minimal Effort)
If hot reload is not critical:
1. Keep `game` hot reload as-is (specialized tool)
2. Focus on task #3 (code consolidation only)
3. Document architectural decisions in READMEs

---

## Notes

- Lua config is already properly consolidated (editor imports from app)
- Bevy crates kept as alternative implementations
- Most "duplication" is actually justified specialization
- Hot reload refactoring is the biggest remaining task

---

## Testing Plan

After each task:
1. Run `cargo check --workspace`
2. Run `cargo clippy --workspace`
3. Run `cargo test --workspace`
4. Build and test affected binaries:
   - `cargo run -p game` (for #1, #2)
   - `cargo run -p editor` (for #2, #3)
5. Verify hot reload works (for #1, #2)
6. Run `just check` before committing
