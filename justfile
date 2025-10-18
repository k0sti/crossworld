# Show available commands
default:
    @echo "Available commands:"
    @echo ""
    @echo "  just dev              - Start development server (builds WASM first)"
    @echo "  just build            - Build everything for production"
    @echo "  just build-wasm       - Build WASM module in release mode"
    @echo "  just build-wasm-dev   - Build WASM module in development mode"
    @echo "  just install          - Install dependencies"
    @echo "  just preview          - Preview production build"
    @echo "  just clean            - Clean build artifacts"
    @echo "  just test             - Run all tests (Rust + TypeScript)"
    @echo "  just check            - Check everything before deployment"
    @echo "  just start-live       - Initialize live event with default parameters"
    @echo ""

# Build WASM module in development mode
build-wasm-dev:
    cd crates/world && wasm-pack build --dev --target web --out-dir ../../packages/wasm --out-name crossworld-world

# Build WASM module in release mode
build-wasm:
    cd crates/world && wasm-pack build --target web --out-dir ../../packages/wasm --out-name crossworld-world

# Start development server (builds WASM first)
dev: build-wasm-dev
    cd packages/app && bun run dev

# Build everything for production
build: build-wasm
    cd packages/app && bun run build

# Clean build artifacts
clean:
    rm -rf packages/wasm
    cd packages/app && rm -rf dist node_modules/.vite

# Install dependencies
install:
    cd packages/app && bun install

# Preview production build
preview:
    cd packages/app && bun run preview

# Initialize Crossworld Nostr live event
start-live:
    cd crates/worldtool && cargo run -- init-live --streaming

# Run all tests
test:
    @echo "Running Rust tests..."
    cargo test --workspace
    @echo "\nRunning TypeScript type check..."
    cd packages/app && bun run build

# Check everything before deployment
check:
    @echo "=== Checking Rust code ==="
    cargo check --workspace
    cargo clippy --workspace -- -D warnings
    cargo fmt --check
    @echo "\n=== Building WASM ==="
    just build-wasm
    @echo "\n=== Building TypeScript app ==="
    cd packages/app && bun run build
    @echo "\n✅ All checks passed! Ready for deployment."
