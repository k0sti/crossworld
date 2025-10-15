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
