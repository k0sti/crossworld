# Build System

## Overview

Crossworld uses a multi-stage build system combining Rust (WASM compilation) and TypeScript (frontend bundling). The `justfile` provides convenient task automation.

**Build Tools**:
- **just** - Task runner (like make)
- **cargo** - Rust package manager and build tool
- **wasm-pack** - Rust to WASM compiler
- **bun** - JavaScript package manager and runtime
- **vite** - Frontend build tool and dev server

## Quick Reference

### Common Commands

```bash
# Development
just dev              # Build WASM (dev) + start dev server
just build-wasm-dev   # Build WASM modules (dev mode)

# Production
just build            # Build everything for production
just build-wasm       # Build WASM modules (release mode)

# Testing
just test             # Run all tests (Rust + TypeScript)
just check            # Pre-deployment validation

# Utilities
just clean            # Clean build artifacts
just install          # Install dependencies
just preview          # Preview production build
```

## Development Workflow

### Initial Setup

```bash
# 1. Install dependencies
just install

# 2. Build WASM modules (dev mode)
just build-wasm-dev

# 3. Start dev server
just dev
```

**First-time setup**: ~2-5 minutes (WASM compilation)

### Daily Development

```bash
# Start dev server (assumes WASM already built)
cd packages/app
bun run dev
```

**Hot reload**: Changes to TypeScript files reload automatically. WASM changes require rebuild.

### When to Rebuild WASM

**Rebuild Required**:
- Modified Rust code in `crates/`
- Changed Cargo dependencies
- Switched between dev/release modes

**Not Required**:
- TypeScript changes
- Asset changes
- Configuration changes

## Build Stages

### Stage 1: WASM Compilation

**Crates Compiled**:
1. `crates/world/` → `packages/wasm-world/`
2. `crates/cube/` → `packages/wasm-cube/`
3. `crates/physics/` → `packages/wasm-physics/`

**Process**:
```bash
# For each crate:
wasm-pack build \
  --target web \
  --out-dir ../../packages/wasm-<name> \
  crates/<name>
```

**Output**:
- `wasm-<name>_bg.wasm` - WebAssembly binary
- `wasm-<name>.js` - JavaScript bindings
- `wasm-<name>.d.ts` - TypeScript definitions
- `package.json` - NPM package metadata

### Stage 2: TypeScript Bundling

**Build Command**:
```bash
cd packages/app
bun run build
```

**Process**:
1. TypeScript compilation (via Vite)
2. Module bundling and tree-shaking
3. Asset optimization (images, fonts)
4. Code splitting
5. Minification (production only)

**Output** (`packages/app/dist/`):
- `index.html` - Entry point
- `assets/*.js` - Bundled JavaScript
- `assets/*.css` - Bundled styles
- `assets/*.wasm` - WASM binaries
- Other assets (images, fonts)

## Build Modes

### Development Mode

**WASM Compilation**:
```bash
wasm-pack build --target web --dev
```

**Characteristics**:
- No optimizations
- Debug symbols included
- Faster compilation (~1-2 minutes)
- Larger file sizes
- Better error messages

**Use When**:
- Local development
- Debugging
- Testing features

### Release Mode

**WASM Compilation**:
```bash
wasm-pack build --target web --release
```

**Characteristics**:
- Full optimizations (opt-level=3, lto=true)
- No debug symbols
- Slower compilation (~5-10 minutes)
- Smaller file sizes (50-70% reduction)
- Production-ready

**Use When**:
- Deploying to production
- Performance testing
- Final builds

## Parallel Builds

### WASM Crates

**Independent Crates** build in parallel:

```bash
# In justfile:
build-wasm:
  wasm-pack build crates/world &
  wasm-pack build crates/cube &
  wasm-pack build crates/physics &
  wait
```

**Performance**: ~3x faster than sequential builds

**Note**: `worldtool` crate not compiled to WASM (CLI only)

## Cargo Configuration

### Workspace Structure

**Root** `Cargo.toml`:
```toml
[workspace]
members = [
  "crates/world",
  "crates/cube",
  "crates/physics",
  "crates/renderer",
  "crates/assets",
  "crates/worldtool"
]
```

### Optimization Settings

**Release Profile** (`Cargo.toml`):
```toml
[profile.release]
opt-level = 3           # Maximum optimizations
lto = true              # Link-time optimization
codegen-units = 1       # Better optimization, slower compile
strip = true            # Strip symbols
panic = 'abort'         # Smaller binary
```

**WASM-specific** (`Cargo.toml` in each crate):
```toml
[lib]
crate-type = ["cdylib"]  # Dynamic library for WASM

[dependencies]
wasm-bindgen = "0.2"     # JavaScript interop
serde = { version = "1.0", features = ["derive"] }
```

## Bun Configuration

### Package Manager

**Install Dependencies**:
```bash
bun install
```

**Why Bun** (per project conventions):
- Faster than npm/yarn
- Drop-in replacement
- Built-in test runner
- Native TypeScript support

### Workspace Setup

**Root** `package.json`:
```json
{
  "name": "crossworld",
  "workspaces": [
    "packages/*"
  ],
  "scripts": {
    "build": "bun run build:wasm && bun run build:app",
    "dev": "cd packages/app && bun run dev"
  }
}
```

## Vite Configuration

### Development Server

**Configuration** (`packages/app/vite.config.ts`):
```typescript
export default defineConfig({
  server: {
    port: 5173,
    headers: {
      // Required for SharedArrayBuffer (WASM threads)
      'Cross-Origin-Embedder-Policy': 'require-corp',
      'Cross-Origin-Opener-Policy': 'same-origin'
    }
  },
  build: {
    target: 'esnext',
    rollupOptions: {
      output: {
        manualChunks: {
          three: ['three'],
          wasm: ['wasm-world', 'wasm-cube', 'wasm-physics']
        }
      }
    }
  }
})
```

### Build Optimizations

**Code Splitting**:
- Vendor chunks (three.js, react)
- WASM modules separate
- Route-based splitting

**Asset Handling**:
- Images optimized automatically
- Fonts inlined if small
- WASM files copied to output

## Task Runner (just)

### Justfile Structure

**Common Tasks**:
```makefile
# Default task (runs when you type 'just')
default:
  @just --list

# Development workflow
dev: build-wasm-dev
  cd packages/app && bun run dev

# Production build
build: build-wasm
  bun run build

# Clean everything
clean:
  rm -rf target/
  rm -rf packages/*/dist
  rm -rf packages/wasm-*
  rm -rf node_modules
```

### Custom Tasks

**Add New Tasks**:
```makefile
# In justfile:
my-task:
  echo "Running my task"
  cargo build
```

**Run**:
```bash
just my-task
```

## Validation & Quality

### Pre-Deployment Checks

**Command**:
```bash
just check
```

**Runs**:
1. `cargo check --workspace` - Verify Rust compiles
2. `cargo clippy --workspace -- -D warnings` - Lint Rust code
3. `cargo fmt --check` - Check Rust formatting
4. `just build-wasm` - Build WASM modules
5. `bun run build` - Build TypeScript

**Pass Criteria**: All steps must succeed

### Testing

**Run All Tests**:
```bash
just test
```

**Runs**:
- `cargo test --workspace` - Rust unit tests
- TypeScript type checking
- Optional: Integration tests

**Example Test Output**:
```
running 33 tests in cube crate
test raycast::tests::test_basic ... ok
test raycast::tests::test_depth ... ok
...
```

## Common Issues

### WASM Compilation Fails

**Issue**: "error: linker `rust-lld` not found"

**Solution**:
```bash
rustup target add wasm32-unknown-unknown
```

---

**Issue**: "getrandom" errors in WASM

**Solution**: Already handled via `getrandom` feature in Cargo.toml

### Build Performance

**Slow WASM Builds**:
- Use `just build-wasm-dev` for development
- Only build release for deployment
- Use `cargo check` instead of `cargo build` when possible

**Slow TypeScript Builds**:
- Clear Vite cache: `rm -rf packages/app/node_modules/.vite`
- Check for large dependencies
- Use code splitting

### Bun Issues

**Issue**: Command not found

**Solution**: Install bun globally:
```bash
curl -fsSL https://bun.sh/install | bash
```

---

**Issue**: "Module not found"

**Solution**: Reinstall dependencies:
```bash
rm -rf node_modules
bun install
```

## Environment Variables

### Development

**Optional Variables**:
```bash
# Override server discovery
export GAME_SERVER=https://localhost:4433

# Custom relay
export NOSTR_RELAY=wss://relay.example.com

# Verbose logging
export RUST_LOG=debug
```

### Production

**Required Headers** (already set in Vite config):
```
Cross-Origin-Embedder-Policy: require-corp
Cross-Origin-Opener-Policy: same-origin
```

**Why Required**: SharedArrayBuffer support for WASM threads

## Deployment

### Build for Production

```bash
# Full production build
just build

# Output in packages/app/dist/
```

### Deploy to Static Host

**Compatible Hosts**:
- Vercel
- Netlify
- Cloudflare Pages
- GitHub Pages
- Any static file server

**Requirements**:
1. Serve `packages/app/dist/` directory
2. Set COOP/COEP headers (for WASM)
3. HTTPS required (for WebTransport)

**Example** (nginx):
```nginx
server {
    listen 443 ssl http2;
    server_name crossworld.example.com;

    root /var/www/crossworld/dist;
    index index.html;

    # Required headers
    add_header Cross-Origin-Embedder-Policy require-corp;
    add_header Cross-Origin-Opener-Policy same-origin;

    # SPA routing
    location / {
        try_files $uri $uri/ /index.html;
    }
}
```

## Continuous Integration

### GitHub Actions Example

```yaml
name: Build and Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: wasm32-unknown-unknown

      - name: Install Bun
        uses: oven-sh/setup-bun@v1

      - name: Install dependencies
        run: bun install

      - name: Run checks
        run: just check

      - name: Run tests
        run: just test

      - name: Build
        run: just build
```

## Related Documentation

- [overview.md](../architecture/overview.md) - System architecture
- [project-structure.md](project-structure.md) - Repository layout
- `justfile` - Build tasks source
- `Cargo.toml` - Rust dependencies and build config
- `packages/app/vite.config.ts` - Vite configuration
